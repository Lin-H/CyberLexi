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

use crate::domain::types::{
    Artwork, CrossRef, DRunOn, Inflection, MwEntry, MwMeta, Quote, RunOn, Sense, SubSense,
    TableRef, TextBlock, Variant,
};

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
    // hwi.psl：音标标签（如 "chiefly US"）
    let pron_label = hwi["psl"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    // vrs：变体拼写（vl 标签 + va 词形）
    let variants = v["vrs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|vr| {
                    let va = vr["va"].as_str().unwrap_or_default();
                    (!va.is_empty()).then(|| Variant {
                        vl: vr["vl"].as_str().unwrap_or_default().to_string(),
                        va: MwEntry::clean_headword(va),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    // ahws：备选词头（地区拼写等）
    let alt_headwords = v["ahws"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|h| h["hw"].as_str())
                .filter(|s| !s.is_empty())
                .map(|s| MwEntry::clean_headword(s))
                .collect()
        })
        .unwrap_or_default();
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
    // ins：屈折变体（§2.8：词条顶层成员；兼容旧数据也看 hwi.ins）
    // il 标签 + if/ifc 词形（ifc 优先级低于 if）
    let inflections = v["ins"]
        .as_array()
        .or_else(|| hwi["ins"].as_array())
        .map(|a| {
            a.iter()
                .filter_map(|i| {
                    let display = i["if"]
                        .as_str()
                        .or_else(|| i["ifc"].as_str())
                        .filter(|s| !s.is_empty())?;
                    Some(Inflection {
                        display: MwEntry::clean_headword(&display.replace('/', "")),
                        label: i["il"].as_str().unwrap_or_default().to_string(),
                    })
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
            let mut uro_labels = str_array(&uro["sls"]);
            uro_labels.extend(str_array(&uro["lbs"]));
            // utxt：派生词例句（vis.t）
            let examples = uro["utxt"]
                .as_array()
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p.as_array())
                        .filter(|p| p.first().and_then(Value::as_str) == Some("vis"))
                        .flat_map(|p| p.get(1).and_then(Value::as_array).into_iter().flatten())
                        .filter_map(|vi| vi["t"].as_str())
                        .map(crate::infra::mw::markup::to_plain)
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            run_ons.push(RunOn {
                ure: MwEntry::clean_headword(ure),
                fl: uro["fl"].as_str().unwrap_or_default().to_string(),
                phonetic: uro_phonetic,
                audio_code: uro_audio,
                labels: uro_labels,
                examples,
            });
        }
    }

    // dros：带释义派生短语（drp + 自带 def，sseq 展开复用 sense 解析）
    let mut dros = Vec::new();
    if let Some(arr) = v["dros"].as_array() {
        for dro in arr {
            let drp = dro["drp"].as_str().unwrap_or_default();
            if drp.is_empty() {
                continue;
            }
            let dro_fl = dro["fl"].as_str().unwrap_or_default().to_string();
            let mut dro_senses = Vec::new();
            for def in dro["def"].as_array().into_iter().flatten() {
                let fl = def["fl"].as_str().or(Some(dro_fl.as_str()));
                if let Some(sseq) = def["sseq"].as_array() {
                    for group in sseq {
                        if let Some(elems) = group.as_array() {
                            walk_sense_elements(elems, fl, &mut dro_senses);
                        }
                    }
                }
            }
            dros.push(DRunOn {
                drp: MwEntry::clean_headword(drp),
                fl: dro_fl,
                senses: dro_senses,
            });
        }
    }

    // dxnls：方向性参见（{dxt|target||} 已转纯文本）
    let dxnls = str_array(&v["dxnls"])
        .into_iter()
        .map(|s| crate::infra::mw::markup::to_plain(&s))
        .filter(|s| !s.is_empty())
        .collect();

    // usages / syns：段落（pl 标题 + pt 正文/例句）
    let usages = parse_text_blocks(&v["usages"]);
    let syns = parse_text_blocks(&v["syns"]);

    // quotes：引文 + 出处
    let quotes = v["quotes"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|q| {
                    let text = q["t"].as_str()?;
                    let aq = &q["aq"];
                    Some(Quote {
                        text: crate::infra::mw::markup::to_plain(text),
                        auth: aq["auth"].as_str().unwrap_or_default().to_string(),
                        source: crate::infra::mw::markup::to_plain(aq["source"].as_str().unwrap_or_default()),
                        date: aq["aqdate"].as_str().unwrap_or_default().to_string(),
                    })
                })
                .filter(|q| !q.text.is_empty())
                .collect()
        })
        .unwrap_or_default();

    // art / table
    let art = v["art"].as_object().and_then(|o| {
        let artid = o.get("artid").and_then(Value::as_str)?;
        (!artid.is_empty()).then(|| Artwork {
            artid: artid.to_string(),
            caption: o
                .get("capt")
                .and_then(Value::as_str)
                .map(crate::infra::mw::markup::to_plain)
                .unwrap_or_default(),
        })
    });
    let table = v["table"].as_object().and_then(|o| {
        let tableid = o.get("tableid").and_then(Value::as_str)?;
        (!tableid.is_empty()).then(|| TableRef {
            tableid: tableid.to_string(),
            display: o
                .get("displayname")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        })
    });

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
    // 各 sense 仍携带正确词性。vd（动词分隔）挂到块内首个 sense；
    // def.sls（块级标签）并入块内全部 sense。
    let mut senses = Vec::new();
    if let Some(defs) = v["def"].as_array() {
        for def in defs {
            let fl = def["fl"].as_str().or(Some(pos.as_str()));
            let vd = def["vd"].as_str().map(str::to_string);
            let def_sls = str_array(&def["sls"]);
            let before = senses.len();
            if let Some(sseq) = def["sseq"].as_array() {
                for group in sseq {
                    if let Some(elems) = group.as_array() {
                        walk_sense_elements(elems, fl, &mut senses);
                        collect_dt_audio(elems, &mut audio_codes);
                    }
                }
            }
            if senses.len() > before {
                if let Some(vd) = vd {
                    senses[before].divider = Some(vd);
                }
                for sense in &mut senses[before..] {
                    for s in &def_sls {
                        if !sense.labels.contains(s) {
                            sense.labels.push(s.clone());
                        }
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
        pron_label,
        variants,
        alt_headwords,
        dros,
        dxnls,
        usages,
        syns,
        quotes,
        art,
        table,
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

/// 从 sense 对象提取 sn/dt（text + vis + 说明性 dt 元素）；sdsense
/// （§2.10.7）挂为 `subsenses` 子块（UI 以 "↳ label: text" 形式渲染）。
/// sense 级 sls/lbs/sgram/ins/et 一并提取。
fn push_sense(body: &Value, fl: Option<&str>, out: &mut Vec<Sense>) {
    let sn = body["sn"].as_str().map(str::to_string);
    let dt = extract_dt(&body["dt"]);
    let mut labels = str_array(&body["sls"]);
    labels.extend(str_array(&body["lbs"]));
    let sgram = body["sgram"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let inflections = parse_ins_labels(&body["ins"]);
    let etymology = body["et"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|pair| pair.as_array())
        .and_then(|pair| {
            (pair.first().and_then(|k| k.as_str()) == Some("text"))
                .then(|| pair.get(1))
                .flatten()
        })
        .and_then(Value::as_str)
        .map(crate::infra::mw::markup::to_plain);
    let subsenses: Vec<SubSense> = body
        .get("sdsense")
        .map(|sd| {
            let sd_dt = extract_dt(&sd["dt"]);
            SubSense {
                label: sd["sd"].as_str().unwrap_or_default().to_string(),
                text: sd_dt.text,
                examples: sd_dt.examples,
            }
        })
        .into_iter()
        .filter(|sd| !sd.text.is_empty() || !sd.examples.is_empty())
        .collect();
    if !dt.text.is_empty()
        || !dt.examples.is_empty()
        || !subsenses.is_empty()
        || !dt.notes.is_empty()
        || !dt.called_also.is_empty()
    {
        out.push(Sense {
            sn,
            pos: fl.filter(|s| !s.is_empty()).map(str::to_string),
            text: dt.text,
            examples: dt.examples,
            subsenses,
            labels,
            sgram,
            inflections,
            notes: dt.notes,
            called_also: dt.called_also,
            etymology,
            divider: None,
        });
    }
}

/// dt 数组的提取结果：text（含 ri/bnw 并入）+ 例句（含 aq 出处后缀）
/// + 说明性文本（snote/uns）+ called also（ca）。
struct DtContent {
    text: String,
    examples: Vec<String>,
    notes: Vec<String>,
    called_also: Vec<String>,
}

/// dt 数组 → 文本/例句/说明（§2.10.6）。MW markup 已转纯文本：
/// - ri（行内 run-in）与 bnw（人名 wrap）按顺序并入 text
/// - vis 的 aq 出处以 "—auth, source (date)" 后缀并入例句
/// - uns/snote → notes（带 USAGE/NOTE 前缀）
/// - ca → called_also（intro: cat 列表）
fn extract_dt(dt: &Value) -> DtContent {
    let mut c = DtContent {
        text: String::new(),
        examples: Vec::new(),
        notes: Vec::new(),
        called_also: Vec::new(),
    };
    let Some(parts) = dt.as_array() else { return c };
    for part in parts {
        let Some(pair) = part.as_array() else { continue };
        match pair.first().and_then(Value::as_str).unwrap_or_default() {
            "text" => {
                if let Some(t) = pair.get(1).and_then(Value::as_str) {
                    push_text(&mut c.text, &crate::infra::mw::markup::to_plain(t));
                }
            }
            "ri" => {
                // ["ri", [["riw",{rie}],["text",...],...]] → 拼接
                for el in pair.get(1).and_then(Value::as_array).into_iter().flatten() {
                    let Some(p2) = el.as_array() else { continue };
                    match p2.first().and_then(Value::as_str).unwrap_or_default() {
                        "riw" => {
                            if let Some(rie) = p2.get(1).and_then(|o| o["rie"].as_str()) {
                                push_text(&mut c.text, &MwEntry::clean_headword(rie));
                            }
                        }
                        "text" => {
                            if let Some(t) = p2.get(1).and_then(Value::as_str) {
                                push_text(&mut c.text, &crate::infra::mw::markup::to_plain(t));
                            }
                        }
                        _ => {}
                    }
                }
            }
            "bnw" => {
                // ["bnw", {pname/sname/altname}] → 按序并入
                if let Some(o) = pair.get(1) {
                    for k in ["pname", "sname", "altname"] {
                        if let Some(n) = o[k].as_str() {
                            push_text(&mut c.text, &MwEntry::clean_headword(n));
                        }
                    }
                }
            }
            "vis" => {
                if let Some(vis) = pair.get(1).and_then(Value::as_array) {
                    for vi in vis {
                        if let Some(t) = vi["t"].as_str() {
                            let rendered = crate::infra::mw::markup::to_plain(t);
                            if rendered.is_empty() {
                                continue;
                            }
                            // aq 出处后缀（—auth, source, date）
                            let aq = &vi["aq"];
                            let auth = aq["auth"].as_str().unwrap_or_default();
                            if auth.is_empty() {
                                c.examples.push(rendered);
                            } else {
                                let mut attr = auth.to_string();
                                let source = aq["source"]
                                    .as_str()
                                    .map(crate::infra::mw::markup::to_plain)
                                    .unwrap_or_default();
                                if !source.is_empty() {
                                    attr.push_str(&format!(", {source}"));
                                }
                                if let Some(d) = aq["aqdate"].as_str() {
                                    if !d.is_empty() {
                                        attr.push_str(&format!(", {d}"));
                                    }
                                }
                                c.examples.push(format!("{rendered} — {attr}"));
                            }
                        }
                    }
                }
            }
            "uns" => {
                // ["uns", [[ ["text",..], ["vis",..] ], ...]]
                for note in pair.get(1).and_then(Value::as_array).into_iter().flatten() {
                    for el in note.as_array().into_iter().flatten() {
                        if let Some(p2) = el.as_array() {
                            if p2.first().and_then(Value::as_str) == Some("text") {
                                if let Some(t) = p2.get(1).and_then(Value::as_str) {
                                    let s = crate::infra::mw::markup::to_plain(t);
                                    if !s.is_empty() {
                                        c.notes.push(format!("USAGE · {s}"));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "snote" => {
                // ["snote", [["t",..], ["vis",..]]]
                for el in pair.get(1).and_then(Value::as_array).into_iter().flatten() {
                    if let Some(p2) = el.as_array() {
                        if p2.first().and_then(Value::as_str) == Some("t") {
                            if let Some(t) = p2.get(1).and_then(Value::as_str) {
                                let s = crate::infra::mw::markup::to_plain(t);
                                if !s.is_empty() {
                                    c.notes.push(format!("NOTE · {s}"));
                                }
                            }
                        }
                    }
                }
            }
            "ca" => {
                // ["ca", {intro, cats:[{cat}]}] → "called also A, B"
                if let Some(o) = pair.get(1) {
                    let intro = o["intro"].as_str().unwrap_or("called also");
                    let cats: Vec<String> = o["cats"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|cat| cat["cat"].as_str())
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default();
                    if !cats.is_empty() {
                        c.called_also.push(format!("{intro} {}", cats.join(", ")));
                    }
                }
            }
            _ => {}
        }
    }
    c
}

/// 追加文本片段（空段间以空格衔接）。
fn push_text(buf: &mut String, s: &str) {
    if s.is_empty() {
        return;
    }
    if !buf.is_empty() {
        buf.push(' ');
    }
    buf.push_str(s);
}

/// JSON 字符串数组（sls/lbs/dxnls 等标签字段通用）。
fn str_array(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// sense.ins → 标签化屈折说明（"il if" 拼接，如 "also acoustic" /
/// 仅 spl 时 "singular in construction"）。
fn parse_ins_labels(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|i| {
                    if let Some(spl) = i["spl"].as_str() {
                        return Some(spl.to_string());
                    }
                    let form = i["if"]
                        .as_str()
                        .or_else(|| i["ifc"].as_str())?
                        .replace('*', "");
                    let il = i["il"].as_str().unwrap_or_default();
                    Some(if il.is_empty() {
                        form
                    } else {
                        format!("{il} {form}")
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// usages / syns 段落：pl 标题 + pt（text/vis 交错）。
fn parse_text_blocks(v: &Value) -> Vec<TextBlock> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|b| {
                    let label = b["pl"].as_str().unwrap_or_default().to_string();
                    let mut text = String::new();
                    let mut examples = Vec::new();
                    for el in b["pt"].as_array().into_iter().flatten() {
                        let Some(p) = el.as_array() else { continue };
                        match p.first().and_then(Value::as_str).unwrap_or_default() {
                            "text" => {
                                if let Some(t) = p.get(1).and_then(Value::as_str) {
                                    push_text(&mut text, &crate::infra::mw::markup::to_plain(t));
                                }
                            }
                            "vis" => {
                                for vi in p.get(1).and_then(Value::as_array).into_iter().flatten() {
                                    if let Some(t) = vi["t"].as_str() {
                                        let s = crate::infra::mw::markup::to_plain(t);
                                        if !s.is_empty() {
                                            examples.push(s);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    (!text.is_empty() || !examples.is_empty())
                        .then(|| TextBlock {
                            label,
                            text,
                            examples,
                        })
                })
                .collect()
        })
        .unwrap_or_default()
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
    /// 文档全字段补齐（§2.4/2.5/2.7.3/2.7.4/2.7.6/2.10.2/2.12/2.13/2.15/
    /// 2.16/2.17/2.18/2.19/2.20/2.21/2.22/2.23/2.24/2.25）：
    /// psl/vrs/ahws/vd/sls/sgram/sense-ins/uns/snote/ca/ri/aq/
    /// uros 标签与例句/dros/dxnls/usages/syns/quotes/art/table。
    #[test]
    fn parses_all_documented_fields() {
        let raw = r#"[
        {
         "meta":{"id":"abide","src":"collegiate","section":"alpha","stems":["abide"]},
         "hwi":{"hw":"abide","psl":"chiefly US","prs":[{"mw":"ə-ˈbīd","sound":{"audio":"abide001","ref":"c"}}]},
         "vrs":[{"vl":"or less commonly","va":"a*bide"},{"vl":"or","va":"a*bide"}],
         "ahws":[{"hw":"abided"},{"hw":"abode"}],
         "fl":"verb",
         "ins":[{"il":"plural","if":"abides"},{"ifc":"-ing","if":"a*bid*ing"}],
         "lbs":["chiefly British"],
         "def":[
          {"vd":"transitive verb","sseq":[[
            ["sense",{"sn":"1","sls":["archaic"],"sgram":"T","dt":[
              ["text","{bc}to await"],
              ["snote",[["t","supplemental note text"]]],
              ["ca",{"intro":"called also","cats":[{"cat":"Manila hemp"},{"cat":"abaca fiber"}]}]
            ]}]
          ]]},
          {"vd":"intransitive verb","sls":["formal"],"sseq":[[
            ["sense",{"sn":"2","ins":[{"spl":"singular in construction"},{"il":"also","if":"abode"}],"et":[["text","from Old English"]],"dt":[
              ["text","{bc}to continue in a place"],
              ["ri",[["riw",{"rie":"Great Abaco"}],["text"," and "],["riw",{"rie":"Little Abaco"}]]],
              ["uns",[[["text","used chiefly in the phrase {phrase}in abeyance{\/phrase}"]]]],
              ["vis",[{"t":"a {wi}abiding{\/wi} place","aq":{"auth":"Bruce Ingram","source":"{it}The Book{\/it}","aqdate":"1975"}}]]
            ]}]
          ]]
        }],
         "uros":[{"ure":"a*bid*ing*ly","fl":"adverb","sls":["formal"],"prs":[{"mw":"ə-ˈbī-diŋ-lē","sound":{"audio":"abiding01","ref":"c"}}],"utxt":[["vis",[{"t":"gestured {wi}abidingly{\/wi}"}]]]}],
         "dros":[{"drp":"abide by","def":[{"sseq":[[
            ["sense",{"sn":"1","dt":[["text","{bc}to conform to"],["vis",[{"t":"{it}abide by{\/it} the rules"}]]]}],
            ["sense",{"sn":"2","dt":[["text","{bc}to accept without objection"]]}
         ]]]}]}],
         "dxnls":["see also {dxt|abidian||}"],
         "usages":[{"pl":"Using abide","pt":[["text","The verb is standard."],["vis",[{"t":"none of the {it}abiding{\/it}"}]]]}],
         "syns":[{"pl":"synonyms","pt":[["text","{sc}abide{\/sc} {sc}wait{\/sc} mean to remain."]],"sarefs":["assent"]}],
         "quotes":[{"t":"a {qword}quotable{\/qword} line","aq":{"auth":"Maxine Kumin","source":"{it}To Make a Prairie{\/it}","aqdate":"1979"}}],
         "art":{"artid":"heart","capt":"heart 1a: {it}1{\/it} aorta"},
         "table":{"tableid":"alphabet","displayname":"Alphabet Table"},
         "et":[["text","Middle English {it}abiden{\/it}"]],
         "date":"before 12th century",
         "shortdef":["to bear patiently"]
        }]"#;
        let MwResponse::Entries(entries) = parse_body(raw).unwrap() else { panic!() };
        let e = &entries[0];

        // psl / vrs / ahws
        assert_eq!(e.pron_label.as_deref(), Some("chiefly US"));
        assert_eq!(e.variants.len(), 2);
        assert_eq!(e.variants[0].vl, "or less commonly");
        assert_eq!(e.variants[0].va, "abide");
        assert_eq!(e.alt_headwords, vec!["abided", "abode"]);

        // ins：il 标签保留；ifc 兜底
        assert_eq!(e.inflections[0].label, "plural");
        assert_eq!(e.inflections[1].display, "abiding");

        // senses：vd 分隔挂块内首 sense；def.sls 并入块内 sense；
        // sense.sls/sgram/spl-ins/et/notes/called_also/ri 并入 text/vis aq
        assert_eq!(e.senses.len(), 2);
        assert_eq!(e.senses[0].divider.as_deref(), Some("transitive verb"));
        assert_eq!(e.senses[1].divider.as_deref(), Some("intransitive verb"));
        assert_eq!(e.senses[0].labels, vec!["archaic"]);
        assert_eq!(e.senses[1].labels, vec!["formal"]);
        assert_eq!(e.senses[0].sgram.as_deref(), Some("T"));
        assert_eq!(e.senses[1].inflections, vec!["singular in construction", "also abode"]);
        assert_eq!(e.senses[1].etymology.as_deref(), Some("from Old English"));
        assert_eq!(e.senses[0].notes, vec!["NOTE · supplemental note text"]);
        assert_eq!(
            e.senses[1].notes,
            vec!["USAGE · used chiefly in the phrase in abeyance"]
        );
        assert_eq!(
            e.senses[0].called_also,
            vec!["called also Manila hemp, abaca fiber"]
        );
        // ri 并入 text
        assert!(e.senses[1].text.contains("Great Abaco and Little Abaco"));
        // vis aq 出处后缀
        assert_eq!(
            e.senses[1].examples,
            vec!["a abiding place — Bruce Ingram, The Book, 1975"]
        );

        // uros：标签 + utxt 例句
        assert_eq!(e.run_ons[0].labels, vec!["formal"]);
        assert_eq!(e.run_ons[0].examples, vec!["gestured abidingly"]);
        assert_eq!(e.run_ons[0].audio_code, "abiding01");

        // dros：drp + 自带 sense
        assert_eq!(e.dros.len(), 1);
        assert_eq!(e.dros[0].drp, "abide by");
        assert_eq!(e.dros[0].senses.len(), 2);
        assert_eq!(e.dros[0].senses[0].text, "to conform to");
        assert_eq!(e.dros[0].senses[0].examples, vec!["abide by the rules"]);

        // dxnls（markup 转纯文本）/ usages / syns / quotes
        assert_eq!(e.dxnls, vec!["see also abidian"]);
        assert_eq!(e.usages.len(), 1);
        assert_eq!(e.usages[0].label, "Using abide");
        assert!(e.usages[0].text.starts_with("The verb is standard."));
        assert_eq!(e.usages[0].examples, vec!["none of the abiding"]);
        assert_eq!(e.syns[0].label, "synonyms");
        assert!(e.syns[0].text.starts_with("abide wait mean to remain."));
        assert_eq!(e.quotes[0].auth, "Maxine Kumin");
        assert_eq!(e.quotes[0].source, "To Make a Prairie");
        assert_eq!(e.quotes[0].date, "1979");
        assert!(e.quotes[0].text.starts_with("a quotable line"));

        // art / table
        assert_eq!(e.art.as_ref().unwrap().artid, "heart");
        assert!(e.art.as_ref().unwrap().caption.starts_with("heart 1a:"));
        assert_eq!(e.table.as_ref().unwrap().tableid, "alphabet");
        assert_eq!(e.table.as_ref().unwrap().display, "Alphabet Table");
    }

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
