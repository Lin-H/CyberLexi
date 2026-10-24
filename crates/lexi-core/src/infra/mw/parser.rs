//! MW Collegiate JSON 解析：`Vec<MwEntry>`。
//!
//! 两种响应形态（官方文档 §JSON）：
//! - 命中：Entry 对象数组（含 homograph 多条，如 test:1/test:2）
//! - 未命中：字符串数组（拼写建议）
//!
//! MW 的 `def[].sseq[][]` 是异构嵌套数组（文档 §2.10），serde derive
//! 无法直接表达，故用 `serde_json::Value` 手工提取；抽取字段 + 完整
//! raw_json 落库。

use serde_json::Value;

use crate::domain::types::{MwEntry, Sense};

/// 解析后的 MW 响应。
#[derive(Debug, Clone, PartialEq)]
pub enum MwResponse {
    Entries(Vec<MwEntry>),
    Suggestions(Vec<String>),
}

/// 解析 MW Collegiate 响应体。
pub fn parse_body(body: &str) -> anyhow::Result<MwResponse> {
    let v: Value = serde_json::from_str(body)?;
    let Value::Array(items) = v else {
        anyhow::bail!("mw response is not a JSON array");
    };
    if items.is_empty() {
        return Ok(MwResponse::Suggestions(Vec::new()));
    }
    // 全为字符串 → 拼写建议；含对象 → Entry 数组（混合时忽略字符串项）
    if items.iter().all(|i| i.is_string()) {
        let sugg = items
            .iter()
            .filter_map(|i| i.as_str().map(str::to_string))
            .collect();
        return Ok(MwResponse::Suggestions(sugg));
    }
    let mut entries = Vec::new();
    for item in &items {
        if item.is_object() {
            entries.push(parse_entry(item)?);
        }
    }
    if entries.is_empty() {
        anyhow::bail!("mw response array contains no entry objects");
    }
    Ok(MwResponse::Entries(entries))
}

/// 提取单个 entry（homograph）。
fn parse_entry(v: &Value) -> anyhow::Result<MwEntry> {
    let hwi = &v["hwi"];
    let headword = MwEntry::clean_headword(hwi["hw"].as_str().unwrap_or_default());
    if headword.is_empty() {
        anyhow::bail!("entry missing hwi.hw");
    }

    let mut phonetic = String::new();
    let mut audio_codes = Vec::new();
    // hwi.prs：首个 mw 作为音标；全部 sound.audio 收集
    if let Some(prs) = hwi["prs"].as_array() {
        for pr in prs {
            if phonetic.is_empty() {
                if let Some(mw) = pr["mw"].as_str() {
                    phonetic = mw.to_string();
                }
            }
            if let Some(code) = pr["sound"]["audio"].as_str() {
                if !code.is_empty() && !audio_codes.iter().any(|c: &String| c == code) {
                    audio_codes.push(code.to_string());
                }
            }
        }
    }
    // uros.prs 音频码（spec 指定收集）
    if let Some(uros) = v["uros"].as_array() {
        for uro in uros {
            if let Some(prs) = uro["prs"].as_array() {
                for pr in prs {
                    if let Some(code) = pr["sound"]["audio"].as_str() {
                        if !code.is_empty() && !audio_codes.iter().any(|c: &String| c == code) {
                            audio_codes.push(code.to_string());
                        }
                    }
                }
            }
        }
    }

    let pos = v["fl"].as_str().unwrap_or_default().to_string();
    let short_defs = v["shortdef"]
        .as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str()).map(str::to_string).collect())
        .unwrap_or_default();

    // def[].sseq 展开
    let mut senses = Vec::new();
    if let Some(defs) = v["def"].as_array() {
        for def in defs {
            if let Some(sseq) = def["sseq"].as_array() {
                for group in sseq {
                    if let Some(elems) = group.as_array() {
                        walk_sense_elements(elems, &mut senses);
                    }
                }
            }
        }
    }

    let etymology = v["et"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|pair| pair.as_array())
        .and_then(|pair| {
            (pair.first().and_then(|k| k.as_str()) == Some("text"))
                .then(|| pair.get(1))
                .flatten()
        })
        .and_then(|t| t.as_str())
        .map(crate::infra::mw::markup::to_plain);

    let date = v["date"].as_str().map(crate::infra::mw::markup::to_plain);

    let stems = v["meta"]["stems"]
        .as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    // spec 提及 meta.labels，但官方文档 meta 无此字段；entry 级 lbs 是
    // 通用标签（§2.7.2），取之。标注：若实测 meta.labels 存在再并入。
    let labels = v["lbs"]
        .as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
        .unwrap_or_default();

    Ok(MwEntry {
        headword,
        phonetic,
        pos,
        short_defs,
        senses,
        audio_codes,
        etymology,
        date,
        stems,
        labels,
        raw_json: serde_json::to_string(v)?,
    })
}

/// 展开 sseq 组内元素：sense / sen / bs / pseq（文档 §2.10）。
fn walk_sense_elements(elems: &[Value], out: &mut Vec<Sense>) {
    for elem in elems {
        let Some(pair) = elem.as_array() else { continue };
        let kind = pair.first().and_then(Value::as_str).unwrap_or_default();
        match kind {
            "sense" => {
                if let Some(body) = pair.get(1) {
                    push_sense(body, None, out);
                    // sdsense（§2.10.7）：追加为独立 sense
                    if let Some(sd) = body.get("sdsense") {
                        let sd_label = sd["sd"].as_str().unwrap_or_default();
                        push_sense(sd, Some(sd_label), out);
                    }
                }
            }
            // 截断 sense（无定义文本，§2.10.8）：跳过
            "sen" => {}
            // binding substitute（§2.10.9）：内含一个 sense
            "bs" => {
                if let Some(inner) = pair.get(1).and_then(|b| b.get("sense")) {
                    push_sense(inner, None, out);
                }
            }
            // 括号序列（§2.10.10）：递归
            "pseq" => {
                if let Some(inner) = pair.get(1).and_then(Value::as_array) {
                    walk_sense_elements(inner, out);
                }
            }
            _ => {}
        }
    }
}

/// 从 sense 对象提取 sn/dt（text + vis）。
fn push_sense(body: &Value, sd_label: Option<&str>, out: &mut Vec<Sense>) {
    let sn = body["sn"].as_str().map(str::to_string);
    let mut text = String::new();
    let mut examples = Vec::new();
    if let Some(dt) = body["dt"].as_array() {
        for part in dt {
            let Some(pair) = part.as_array() else { continue };
            match pair.first().and_then(Value::as_str).unwrap_or_default() {
                "text" => {
                    if let Some(t) = pair.get(1).and_then(Value::as_str) {
                        let rendered = crate::infra::mw::markup::to_plain(t);
                        if !rendered.is_empty() {
                            if !text.is_empty() {
                                text.push(' ');
                            }
                            text.push_str(&rendered);
                        }
                    }
                }
                "vis" => {
                    if let Some(vis) = pair.get(1).and_then(Value::as_array) {
                        for vi in vis {
                            if let Some(t) = vi["t"].as_str() {
                                let rendered = crate::infra::mw::markup::to_plain(t);
                                if !rendered.is_empty() {
                                    examples.push(rendered);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // sdsense 的 sd 分隔词（如 "also"/"specifically"）并入文本开头
    if let Some(label) = sd_label {
        if !label.is_empty() && !text.is_empty() {
            text = format!("{label}: {text}");
        }
    }
    if !text.is_empty() || !examples.is_empty() {
        out.push(Sense { sn, text, examples });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文档开头的 voluminous 完整响应样例。
    const VOLUMINOUS: &str = r#"[
    {"meta":{
     "id":"voluminous",
     "uuid":"0d01b967-971f-4ec5-8fe0-10513d29c39b",
     "sort":"220130400",
     "src":"collegiate",
     "section":"alpha",
     "stems":[
        "voluminous",
        "voluminously",
        "voluminousness",
        "voluminousnesses"
     ],
     "offensive":false
    },
    "hwi":{
     "hw":"vo*lu*mi*nous",
     "prs":[
        {
           "mw":"və-ˈlü-mə-nəs",
           "sound":{"audio":"volumi02","ref":"c","stat":"1"}
        }
     ]
    },
    "fl":"adjective",
    "def":[
     {"sseq":[
        [
           ["sense",
            {"sn":"1 a",
             "dt":[
                ["text","{bc}having or marked by great {a_link|volume} or bulk {bc}{sx|large||} "],
                ["vis",[{"t":"long {wi}voluminous{\/wi} tresses"}]]
             ],
             "sdsense":{
                "sd":"also",
                "dt":[["text","{bc}{sx|full||} "]]
             }
            }],
           ["sense",
            {"sn":"b",
             "dt":[
                ["text","{bc}{sx|numerous||} "],
                ["vis",[{"t":"trying to keep track of {wi}voluminous{\/wi} slips of paper"}]]
             ]}
           ]
        ],
        [
           ["sense",{"sn":"3","dt":[["text","{bc}consisting of many folds"]]}]
        ]
     ]}
    ],
    "uros":[
     {"ure":"vo*lu*mi*nous*ly","fl":"adverb"},
     {"ure":"vo*lu*mi*nous*ness",
      "prs":[{"mw":"-nəs","sound":{"audio":"volumi03","ref":"c","stat":"1"}}]}
    ],
    "et":[
     ["text","Late Latin {it}voluminosus{\/it}, from Latin {it}volumin-, volumen{\/it}"]
    ],
    "date":"1611{ds||3||}",
    "shortdef":[
     "having or marked by great volume or bulk : large; also : full",
     "numerous",
     "filling or capable of filling a large volume or several volumes"
    ]}
]"#;

    #[test]
    fn parses_full_entry() {
        let resp = parse_body(VOLUMINOUS).unwrap();
        let MwResponse::Entries(entries) = resp else {
            panic!("expected entries");
        };
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.headword, "voluminous"); // '*' 已去除
        assert_eq!(e.phonetic, "və-ˈlü-mə-nəs");
        assert_eq!(e.pos, "adjective");
        assert_eq!(e.short_defs.len(), 3);
        assert_eq!(e.stems.len(), 4);
        // 音频码：hwi + uros（去重）
        assert_eq!(e.audio_codes, vec!["volumi02", "volumi03"]);
        // 词源 markup 转纯文本
        assert_eq!(
            e.etymology.as_deref(),
            Some("Late Latin voluminosus, from Latin volumin-, volumen")
        );
        assert_eq!(e.date.as_deref(), Some("1611"));
        // raw_json 可再解析
        let raw: Value = serde_json::from_str(&e.raw_json).unwrap();
        assert_eq!(raw["meta"]["id"], "voluminous");
    }

    #[test]
    fn senses_include_sdsense_and_examples() {
        let resp = parse_body(VOLUMINOUS).unwrap();
        let MwResponse::Entries(entries) = resp else { panic!() };
        let senses = &entries[0].senses;
        // 1a、also(sdsense)、b、3
        assert_eq!(senses.len(), 4);
        assert_eq!(senses[0].sn.as_deref(), Some("1 a"));
        assert!(senses[0].text.contains("having or marked by great volume"));
        assert_eq!(senses[0].examples, vec!["long voluminous tresses"]);
        // sdsense：sn 继承？——设计为 None（sd 并入文本）
        assert!(senses[1].text.starts_with("also: "));
        assert_eq!(senses[3].sn.as_deref(), Some("3"));
    }

    #[test]
    fn string_array_is_suggestions() {
        let body = r#"["python","pythonic","pylon"]"#;
        match parse_body(body).unwrap() {
            MwResponse::Suggestions(s) => assert_eq!(s, vec!["python", "pythonic", "pylon"]),
            _ => panic!(),
        }
    }

    #[test]
    fn empty_array_is_empty_suggestions() {
        match parse_body("[]").unwrap() {
            MwResponse::Suggestions(s) => assert!(s.is_empty()),
            _ => panic!(),
        }
    }

    #[test]
    fn homographs_produce_multiple_entries() {
        // 简化的双 homograph（test:1 noun / test:2 verb，文档 §2.2）
        let body = r#"[
            {"meta":{"id":"test:1","stems":["test"]},"hwi":{"hw":"test","prs":[{"mw":"ˈtest","sound":{"audio":"test0001","ref":"c","stat":"1"}}]},"fl":"noun","shortdef":["a"],"def":[{"sseq":[[["sense",{"sn":"1","dt":[["text","{bc}a trial"]]}]]]}]},
            {"meta":{"id":"test:2","stems":["test"]},"hwi":{"hw":"test"},"fl":"verb","shortdef":["b"],"def":[{"sseq":[[["sense",{"sn":"1","dt":[["text","{bc}to try"]]}]]]}]}
        ]"#;
        match parse_body(body).unwrap() {
            MwResponse::Entries(es) => {
                assert_eq!(es.len(), 2);
                assert_eq!(es[0].pos, "noun");
                assert_eq!(es[1].pos, "verb");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn bs_and_pseq_are_flattened() {
        // 文档 §2.10.10 pseq/bs 例（monitor 词条节选）
        // sseq = [G1, G2]；G1=[[pseq, [bs, sense(1)]]]；G2=[[sen],[sense(a)]]
        let body = r#"[
 {"meta":{"id":"m"},"hwi":{"hw":"m"},"fl":"noun","shortdef":["x"],
  "def":[{"sseq":[
    [["pseq",[
       ["bs",{"sense":{"sn":"c","dt":[["text","{bc}one that {a_link|monitors}: such as"]]}}],
       ["sense",{"sn":"(1)","dt":[["text","an electronic device"]]}]
    ]]],
    [["sen",{"sn":"3","et":[["text","by shortening"]]}],
     ["sense",{"sn":"a","dt":[["text","{bc}a warship"]]}]]
  ]}]
}]"#;
        match parse_body(body).unwrap() {
            MwResponse::Entries(es) => {
                let sns = &es[0].senses;
                // bs 的 c、(1)、a（sen 截断跳过）
                let sns_labels: Vec<&str> = sns.iter().filter_map(|s| s.sn.as_deref()).collect();
                assert_eq!(sns_labels, vec!["c", "(1)", "a"]);
                assert!(sns[0].text.contains("one that monitors"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn minimal_entry_without_def() {
        let body = r#"[{"meta":{"id":"x"},"hwi":{"hw":"x"},"fl":"verb"}]"#;
        match parse_body(body).unwrap() {
            MwResponse::Entries(es) => {
                assert!(es[0].senses.is_empty());
                assert_eq!(es[0].pos, "verb");
            }
            _ => panic!(),
        }
    }
}
