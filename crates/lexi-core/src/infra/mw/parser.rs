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

use crate::domain::types::{CrossRef, Inflection, MwEntry, MwMeta, RunOn, Sense, SubSense};

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

/// 缓存回放：raw_json（combine 后为全 entry 的 JSON 数组；旧缓存可能是
/// 单 entry 对象或 entry 数组）→ 全部 MwEntry，交 combine_entries 还原
/// 完整合并结果（含短语分离）。后续新增 MW 字段无需再为缓存做列迁移。
pub fn parse_cached_entries(raw: &str) -> Vec<MwEntry> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let objects: Vec<Value> = match v {
        Value::Array(items) => items.into_iter().filter(|i| i.is_object()).collect(),
        Value::Object(_) => vec![v],
        _ => Vec::new(),
    };
    objects
        .iter()
        .filter_map(|o| parse_entry(o).ok())
        .collect()
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
    // hwi.altprs：备用音标（无 sound）
    let alt_phonetics = hwi["altprs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|p| p["mw"].as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    // hwi.ins：屈折变体（if 词形）
    let inflections = hwi["ins"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|i| i["if"].as_str())
                .filter(|s| !s.is_empty())
                .map(|s| Inflection {
                    display: MwEntry::clean_headword(&s.replace('/', "")),
                })
                .collect()
        })
        .unwrap_or_default();

    // uros：派生词条（ure/fl/prs；音频码归派生词自身，行内播放）
    let mut run_ons = Vec::new();
    if let Some(uros) = v["uros"].as_array() {
        for uro in uros {
            let ure = uro["ure"].as_str().unwrap_or_default();
            if ure.is_empty() {
                continue;
            }
            let pr = uro["prs"].as_array().and_then(|a| a.first());
            let uro_phonetic = pr
                .and_then(|p| p["mw"].as_str())
                .map(str::to_string)
                .unwrap_or_default();
            let uro_audio = pr
                .and_then(|p| p["sound"]["audio"].as_str())
                .filter(|c| !c.is_empty())
                .unwrap_or_default()
                .to_string();
            run_ons.push(RunOn {
                ure: MwEntry::clean_headword(ure),
                fl: uro["fl"].as_str().unwrap_or_default().to_string(),
                phonetic: uro_phonetic,
                audio_code: uro_audio,
            });
        }
    }

    // cxs：交叉引用
    let cross_refs = v["cxs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|cx| {
                    let refs = cx["cxref"]
                        .as_array()
                        .map(|rs| {
                            rs.iter()
                                .filter_map(|r| r.as_str())
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    (!refs.is_empty()).then(|| CrossRef {
                        label: cx["cxl"].as_str().unwrap_or_default().to_string(),
                        refs,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let pos = v["fl"].as_str().unwrap_or_default().to_string();
    let short_defs = v["shortdef"]
        .as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str()).map(str::to_string).collect())
        .unwrap_or_default();

    // def[].sseq 展开；def 块可自带 fl（多词性词条各块不同）；缺省
    // 回退 entry 级 fl——同形词（test:1 名词 / test:2 动词）合并后
    // 各 sense 仍携带正确词性
    let mut senses = Vec::new();
    if let Some(defs) = v["def"].as_array() {
        for def in defs {
            let fl = def["fl"].as_str().or(Some(pos.as_str()));
            if let Some(sseq) = def["sseq"].as_array() {
                for group in sseq {
                    if let Some(elems) = group.as_array() {
                        walk_sense_elements(elems, fl, &mut senses);
                        collect_dt_audio(elems, &mut audio_codes);
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

    let hom = v["hom"].as_u64().map(|h| h.to_string());

    let meta = v["meta"].as_object().map(|m| MwMeta {
        id: m.get("id").and_then(Value::as_str).unwrap_or_default().to_string(),
        src: m.get("src").and_then(Value::as_str).unwrap_or_default().to_string(),
        section: m
            .get("section")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        sort: m
            .get("sort")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        stems: m
            .get("stems")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
        offensive: m.get("offensive").and_then(Value::as_bool).unwrap_or(false),
    });
    let stems = meta
        .as_ref()
        .map(|m| m.stems.clone())
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
        hom,
        meta,
        alt_phonetics,
        inflections,
        run_ons,
        cross_refs,
        phrases: Vec::new(),
        raw_json: serde_json::to_string(v)?,
    })
}

/// 收集 sseq 组内所有 sense/sdsense dt 的 ["audio", code] 元组
/// （义项内嵌录音，§2.8.2）——作为词头发音变体补充（去重后追加）。
fn collect_dt_audio(elems: &[Value], out: &mut Vec<String>) {
    fn dt_audio(dt: &Value, out: &mut Vec<String>) {
        for part in dt.as_array().into_iter().flatten() {
            if let Some(pair) = part.as_array() {
                if pair.first().and_then(Value::as_str) == Some("audio") {
                    if let Some(code) = pair.get(1).and_then(Value::as_str) {
                        if !code.is_empty() && !out.iter().any(|c| c == code) {
                            out.push(code.to_string());
                        }
                    }
                }
            }
        }
    }
    for elem in elems {
        let Some(pair) = elem.as_array() else { continue };
        let kind = pair.first().and_then(Value::as_str).unwrap_or_default();
        match kind {
            "sense" => {
                if let Some(body) = pair.get(1) {
                    dt_audio(&body["dt"], out);
                    if let Some(sd) = body.get("sdsense") {
                        dt_audio(&sd["dt"], out);
                    }
                }
            }
            "bs" => {
                if let Some(body) = pair.get(1).and_then(|b| b.get("sense")) {
                    dt_audio(&body["dt"], out);
                }
            }
            "pseq" => {
                if let Some(inner) = pair.get(1).and_then(Value::as_array) {
                    collect_dt_audio(inner, out);
                }
            }
            _ => {}
        }
    }
}

/// 展开 sseq 组内元素：sense / sen / bs / pseq（文档 §2.10）。
fn walk_sense_elements(elems: &[Value], fl: Option<&str>, out: &mut Vec<Sense>) {    for elem in elems {
        let Some(pair) = elem.as_array() else { continue };
        let kind = pair.first().and_then(Value::as_str).unwrap_or_default();
        match kind {
            "sense" => {
                if let Some(body) = pair.get(1) {
                    push_sense(body, fl, out);
                }
            }
            // 截断 sense（无定义文本，§2.10.8）：跳过
            "sen" => {}
            // binding substitute（§2.10.9）：内含一个 sense
            "bs" => {
                if let Some(inner) = pair.get(1).and_then(|b| b.get("sense")) {
                    push_sense(inner, fl, out);
                }
            }
            // 括号序列（§2.10.10）：递归
            "pseq" => {
                if let Some(inner) = pair.get(1).and_then(Value::as_array) {
                    walk_sense_elements(inner, fl, out);
                }
            }
            _ => {}
        }
    }
}

/// 从 sense 对象提取 sn/dt（text + vis）；sdsense（§2.10.7）挂为
/// `subsenses` 子块（UI 以 "↳ label: text" 形式渲染）。
fn push_sense(body: &Value, fl: Option<&str>, out: &mut Vec<Sense>) {
    let sn = body["sn"].as_str().map(str::to_string);
    let (text, examples) = extract_dt(&body["dt"]);
    let subsenses: Vec<SubSense> = body
        .get("sdsense")
        .map(|sd| {
            let (sd_text, sd_examples) = extract_dt(&sd["dt"]);
            SubSense {
                label: sd["sd"].as_str().unwrap_or_default().to_string(),
                text: sd_text,
                examples: sd_examples,
            }
        })
        .into_iter()
        .filter(|sd| !sd.text.is_empty() || !sd.examples.is_empty())
        .collect();
    if !text.is_empty() || !examples.is_empty() || !subsenses.is_empty() {
        out.push(Sense {
            sn,
            pos: fl.filter(|s| !s.is_empty()).map(str::to_string),
            text,
            examples,
            subsenses,
        });
    }
}

/// dt 数组 → (拼接文本, 例句列表)；MW markup 已转纯文本。
fn extract_dt(dt: &Value) -> (String, Vec<String>) {
    let mut text = String::new();
    let mut examples = Vec::new();
    let Some(parts) = dt.as_array() else { return (text, examples) };
    for part in parts {
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
    (text, examples)
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
        // 音频码：hwi 词头（uros 派生词的录音归派生词自身，不混入）
        assert_eq!(e.audio_codes, vec!["volumi02"]);
        // 词源 markup 转纯文本
        assert_eq!(
            e.etymology.as_deref(),
            Some("Late Latin voluminosus, from Latin volumin-, volumen")
        );
        assert_eq!(e.date.as_deref(), Some("1611"));
        // meta 提取
        let meta = e.meta.as_ref().expect("meta");
        assert_eq!(meta.id, "voluminous");
        assert_eq!(meta.src, "collegiate");
        assert_eq!(meta.section, "alpha");
        assert_eq!(meta.sort, "220130400");
        assert!(!meta.offensive);
        // uros 派生词（第二个自带录音，行内播放；第一个无 prs）
        assert_eq!(e.run_ons.len(), 2);
        assert_eq!(e.run_ons[0].ure, "voluminously");
        assert_eq!(e.run_ons[0].fl, "adverb");
        assert_eq!(e.run_ons[0].audio_code, "");
        assert_eq!(e.run_ons[1].ure, "voluminousness");
        assert_eq!(e.run_ons[1].audio_code, "volumi03");
        // raw_json 可再解析
        let raw: Value = serde_json::from_str(&e.raw_json).unwrap();
        assert_eq!(raw["meta"]["id"], "voluminous");
    }

    /// 义项 dt 内嵌录音（§2.8.2）：作为词头发音变体收集（含 sdsense、
    /// pseq 嵌套），与 hwi 音频去重。
    #[test]
    fn collects_sense_dt_audio_as_variants() {
        let raw = r#"[{"meta":{"id":"sound"},"hwi":{"hw":"sound","prs":[{"mw":"ˈsaʊnd","sound":{"audio":"sound001","ref":"c"}}]},"fl":"noun","shortdef":["x"],"def":[{"sseq":[
            [["sense",{"sn":"1","dt":[["text","a noise"],["audio","sound004"]]}]],
            [["pseq",[["sense",{"sn":"2 a","dt":[["audio","sound002"]]}],["sense",{"sn":"b","dt":[["text","y"],["audio","sound001"]]}]]]],
            [["bs",{"sense":{"sn":"3","dt":[["audio","sound003"]]}}]]
        ]}]}]"#;
        let MwResponse::Entries(entries) = parse_body(raw).unwrap() else { panic!() };
        let e = &entries[0];
        assert_eq!(
            e.audio_codes,
            vec!["sound001", "sound004", "sound002", "sound003"]
        );
    }

    #[test]
    fn senses_attach_sdsense_as_subsense() {        let resp = parse_body(VOLUMINOUS).unwrap();
        let MwResponse::Entries(entries) = resp else { panic!() };
        let senses = &entries[0].senses;
        // 1a（含 subsense）、b、3 —— sdsense 不再摊平为独立 sense
        assert_eq!(senses.len(), 3);
        assert_eq!(senses[0].sn.as_deref(), Some("1 a"));
        assert!(senses[0].text.contains("having or marked by great volume"));
        assert_eq!(senses[0].examples, vec!["long voluminous tresses"]);
        let sd = &senses[0].subsenses[0];
        assert_eq!(sd.label, "also");
        assert_eq!(sd.text, "full");
        assert_eq!(senses[2].sn.as_deref(), Some("3"));
        assert!(senses[0].subsenses.len() == 1);
    }

    #[test]
    fn homograph_and_hom_number_extracted() {
        // 双 homograph（test:1/test:2，文档 §2.2）+ hom 序号 + cxs/ins
        let body = r#"[
            {"meta":{"id":"test:1","stems":["test"]},"hom":1,
             "hwi":{"hw":"test","ins":[{"if":"tests"},{"if":"tested"}],
                    "prs":[{"mw":"ˈtest","sound":{"audio":"test0001","ref":"c","stat":"1"}}]},
             "fl":"noun","shortdef":["a"],
             "cxs":[{"cxl":"synonym of","cxref":["trial","proof"]}],
             "def":[{"sseq":[[["sense",{"sn":"1","dt":[["text","{bc}a trial"]]}]]]}]},
            {"meta":{"id":"test:2","stems":["test"]},"hom":2,
             "hwi":{"hw":"test"},"fl":"verb","shortdef":["b"],
             "def":[{"sseq":[[["sense",{"sn":"1","dt":[["text","{bc}to try"]]}]]]}]}
        ]"#;
        match parse_body(body).unwrap() {
            MwResponse::Entries(es) => {
                assert_eq!(es.len(), 2);
                assert_eq!(es[0].pos, "noun");
                assert_eq!(es[1].pos, "verb");
                assert_eq!(es[0].hom.as_deref(), Some("1"));
                assert_eq!(es[1].hom.as_deref(), Some("2"));
                assert_eq!(
                    es[0].inflections.iter().map(|i| i.display.as_str()).collect::<Vec<_>>(),
                    vec!["tests", "tested"]
                );
                assert_eq!(es[0].cross_refs.len(), 1);
                assert_eq!(es[0].cross_refs[0].label, "synonym of");
                assert_eq!(es[0].cross_refs[0].refs, vec!["trial", "proof"]);
                // def 无 fl 时 sense 回退 entry 级 fl（同形词合并后词性正确）
                assert_eq!(es[0].senses[0].pos.as_deref(), Some("noun"));
                assert_eq!(es[1].senses[0].pos.as_deref(), Some("verb"));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn string_array_is_suggestions() {
        let body = r#"["python","pythonic","pylon"]"#;
        match parse_body(body).unwrap() {
            MwResponse::Suggestions(s) => assert_eq!(s, vec!["python", "pythonic", "pylon"]),
            _ => panic!(),
        }
    }

    /// 多词性词条：def 块各自带 fl，sense 携带块级词性（非 entry 级）。
    #[test]
    fn per_def_fl_attaches_to_senses() {
        let body = r#"[
            {"meta":{"id":"test"},"hwi":{"hw":"test"},
             "fl":"noun",
             "def":[
               {"fl":"noun","sseq":[[["sense",{"sn":"1","dt":[["text","{bc}an exam"]]}]]]},
               {"fl":"verb","sseq":[[["sense",{"sn":"1","dt":[["text","{bc}to try"]]}]]]}
             ],
             "shortdef":["a"]}
        ]"#;
        match parse_body(body).unwrap() {
            MwResponse::Entries(es) => {
                assert_eq!(es[0].pos, "noun"); // entry 级仍是首个 fl
                assert_eq!(es[0].senses[0].pos.as_deref(), Some("noun"));
                assert_eq!(es[0].senses[1].pos.as_deref(), Some("verb"));
            }
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
