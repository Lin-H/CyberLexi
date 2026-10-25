//! 字段级合并：MW 在线结果 + ECDICT 离线记录 → [`MergedEntry`]。
//!
//! 规则（spec 查询链路第 5 步）：
//! - 音标/词性/例句/音频/词源/标签：优先 MW，缺则 ECDICT；
//! - 中文释义：永远来自 ECDICT；
//! - 词频/考试标签：ECDICT；
//! - headword 以 MW `hwi.hw` 为准回写显示。

use super::types::{EcdictRecord, MergedEntry, MwEntry, PhraseEntry, Sense, SourceTag};

#[derive(Default)]
pub struct ResultMerger;

/// 多词条合并（MW 对一个词可能返回多个 entry）：
/// - **同形词条**（remain:1 动词 / remain:2 名词）：首 entry 为骨架，
///   senses/audio/stems 等串联；
/// - **短语词条**（hw 或 meta.id 含空格，如 "remain to be seen"）：
///   不并入义项，收集为 `phrases` 独立展示；
/// - `raw_json` 存全 entry 的 JSON 数组（缓存重放时还原完整合并结果）。
pub fn combine_entries(entries: Vec<MwEntry>) -> MwEntry {
    let mut it = entries.into_iter();
    let mut first = it.next().expect("combine called with entries");
    let mut phrases: Vec<PhraseEntry> = Vec::new();
    let mut all_raw = vec![first.raw_json.clone()];
    for e in it {
        all_raw.push(e.raw_json.clone());
        let is_phrase = e.headword.contains(' ')
            || e.meta.as_ref().is_some_and(|m| m.id.contains(' '));
        if is_phrase {
            phrases.push(PhraseEntry {
                headword: e.headword.clone(),
                pos: e.pos.clone(),
                definition: e
                    .short_defs
                    .first()
                    .cloned()
                    .or_else(|| e.senses.first().map(|s| s.text.clone()))
                    .unwrap_or_default(),
                example: e.senses.first().and_then(|s| s.examples.first().cloned()),
            });
            continue;
        }
        first.senses.extend(e.senses);
        for c in e.audio_codes {
            if !first.audio_codes.contains(&c) {
                first.audio_codes.push(c);
            }
        }
        for s in e.stems {
            if !first.stems.contains(&s) {
                first.stems.push(s);
            }
        }
        first.run_ons.extend(e.run_ons);
        first.inflections.extend(e.inflections);
        first.cross_refs.extend(e.cross_refs);
        first.alt_phonetics.extend(e.alt_phonetics);
        first.dros.extend(e.dros);
        first.usages.extend(e.usages);
        first.syns.extend(e.syns);
        first.quotes.extend(e.quotes);
        first.dxnls.extend(e.dxnls);
        for va in e.variants {
            if !first.variants.iter().any(|v| v.va == va.va) {
                first.variants.push(va);
            }
        }
        for hw in e.alt_headwords {
            if !first.alt_headwords.contains(&hw) {
                first.alt_headwords.push(hw);
            }
        }
        if first.phonetic.is_empty() {
            first.phonetic = e.phonetic;
        }
        if first.pos.is_empty() {
            first.pos = e.pos;
        }
        if first.etymology.is_none() {
            first.etymology = e.etymology;
        }
        if first.date.is_none() {
            first.date = e.date;
        }
        if first.hom.is_none() {
            first.hom = e.hom;
        }
        if first.meta.is_none() {
            first.meta = e.meta;
        }
        if first.pron_label.is_none() {
            first.pron_label = e.pron_label;
        }
        if first.art.is_none() {
            first.art = e.art;
        }
        if first.table.is_none() {
            first.table = e.table;
        }
    }
    first.phrases = phrases;
    first.raw_json = format!("[{}]", all_raw.join(","));
    first
}

/// ECDICT 中文行行首词性（"n. 测试…"）→ (完整词性, 去前缀文本)。
/// 无法识别时返回 (None, 原文)。
fn split_ecdict_pos(line: &str) -> (Option<String>, String) {
    let t = line.trim();
    let Some((head, rest)) = t.split_once('.') else {
        return (None, t.to_string());
    };
    let head = head.trim();
    let lower = head.to_ascii_lowercase();
    let full = match lower.as_str() {
        "n" => "noun",
        "v" | "vt" | "vi" => "verb",
        "a" | "adj" => "adjective",
        "ad" | "adv" => "adverb",
        "prep" => "preposition",
        "conj" => "conjunction",
        "pron" => "pronoun",
        "art" => "article",
        "num" => "numeral",
        "int" => "interjection",
        "aux" => "auxiliary",
        _ => return (None, t.to_string()),
    };
    // head 必须是纯 ASCII 字母缩写（排除 "e.g." 或普通句中句点）
    if !head.chars().all(|c| c.is_ascii_alphabetic()) || head.is_empty() {
        return (None, t.to_string());
    }
    let body = rest.trim();
    if body.is_empty() {
        (None, t.to_string())
    } else {
        (Some(full.to_string()), body.to_string())
    }
}

impl ResultMerger {
    pub fn new() -> Self {
        Self
    }

    /// 合并 MW 与 ECDICT。两者皆 `None` 时产出仅含查询键的空词条
    ///（调用方 `QueryOrchestrator` 据此决定返回 `NotFound`）。
    pub fn merge(
        &self,
        mw: Option<MwEntry>,
        ecdict: Option<EcdictRecord>,
        queried_key: &str,
    ) -> MergedEntry {
        let mut sources = Vec::new();
        if mw.is_some() {
            sources.push(SourceTag::Mw);
        }
        if ecdict.is_some() {
            sources.push(SourceTag::Ecdict);
        }

        let headword_display = mw
            .as_ref()
            .map(|m| m.headword.clone())
            .or_else(|| ecdict.as_ref().map(|e| e.word.clone()))
            .unwrap_or_else(|| queried_key.to_string());

        // 音标：MW 优先，缺则 ECDICT（ECDICT 音标为 IPA 风格，直接展示）
        let phonetic = mw
            .as_ref()
            .filter(|m| !m.phonetic.is_empty())
            .map(|m| m.phonetic.clone())
            .or_else(|| {
                ecdict
                    .as_ref()
                    .filter(|e| !e.phonetic.is_empty())
                    .map(|e| e.phonetic.clone())
            })
            .unwrap_or_default();
        // IPA：独立保留 ECDICT 音标（与 MW 记音并列展示）
        let ipa = ecdict
            .as_ref()
            .filter(|e| !e.phonetic.is_empty())
            .map(|e| e.phonetic.clone());

        let pos = mw
            .as_ref()
            .filter(|m| !m.pos.is_empty())
            .map(|m| m.pos.clone())
            .or_else(|| {
                ecdict
                    .as_ref()
                    .filter(|e| !e.pos.is_empty())
                    .map(|e| e.pos.clone())
            })
            .unwrap_or_default();

        // 英英释义：MW shortdef；缺则由 ECDICT 中文释义行充当（离线模式下作为主释义展示）
        let short_defs = match (&mw, &ecdict) {
            (Some(m), _) if !m.short_defs.is_empty() => m.short_defs.clone(),
            (_, Some(e)) if !e.translation.is_empty() => e.translation_lines(),
            _ => Vec::new(),
        };

        // 释义序列：MW sseq 展开；无 MW 时把 ECDICT 中文行包装为 senses
        //（无例句；行首词性标注如 "n." / "vt." 解析为 sense 级词性并剥前缀）
        let senses: Vec<Sense> = match (&mw, &ecdict) {
            (Some(m), _) if !m.senses.is_empty() => m.senses.clone(),
            (_, Some(e)) if !e.translation.is_empty() => e
                .translation_lines()
                .into_iter()
                .enumerate()
                .map(|(i, text)| {
                    let (pos, body) = split_ecdict_pos(&text);
                    Sense {
                        sn: Some((i + 1).to_string()),
                        pos,
                        text: body,
                        ..Sense::default()
                    }
                })
                .collect(),
            _ => Vec::new(),
        };

        let chinese = ecdict
            .as_ref()
            .filter(|e| !e.translation.is_empty())
            .map(|e| e.translation_lines().join("\n"));

        let freq = ecdict.as_ref().filter(|e| e.bnc > 0 || e.frq > 0).map(|e| {
            (
                if e.bnc > 0 { e.bnc } else { 0 },
                if e.frq > 0 { e.frq } else { 0 },
                e.collins,
                e.oxford,
            )
        });

        let exam_tags = ecdict
            .as_ref()
            .map(EcdictRecord::exam_tags)
            .unwrap_or_default();

        let (audio_codes, etymology, date) = match &mw {
            Some(m) => (m.audio_codes.clone(), m.etymology.clone(), m.date.clone()),
            None => (Vec::new(), None, None),
        };

        // MW 结构化字段直通（无 MW 时空值）
        let empty = MwEntry::default();
        let m = mw.as_ref().unwrap_or(&empty);
        let (hom, stems, meta, alt_phonetics, inflections, run_ons, cross_refs, phrases, offensive,
             pron_label, variants, alt_headwords, dros, dxnls, usages, syns, quotes, art, table) = (
            m.hom.clone(),
            m.stems.clone(),
            m.meta.clone(),
            m.alt_phonetics.clone(),
            m.inflections.clone(),
            m.run_ons.clone(),
            m.cross_refs.clone(),
            m.phrases.clone(),
            m.meta.as_ref().is_some_and(|x| x.offensive),
            m.pron_label.clone(),
            m.variants.clone(),
            m.alt_headwords.clone(),
            m.dros.clone(),
            m.dxnls.clone(),
            m.usages.clone(),
            m.syns.clone(),
            m.quotes.clone(),
            m.art.clone(),
            m.table.clone(),
        );

        MergedEntry {
            headword_display,
            normalized_key: queried_key.to_string(),
            phonetic,
            ipa,
            pos,
            short_defs,
            senses,
            chinese,
            freq,
            exam_tags,
            audio_codes,
            etymology,
            date,
            sources,
            hom,
            stems,
            meta,
            alt_phonetics,
            inflections,
            run_ons,
            cross_refs,
            phrases,
            offensive,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::SourceTag;

    fn mw() -> MwEntry {
        MwEntry {
            headword: "voluminous".into(),
            phonetic: "və-ˈlü-mə-nəs".into(),
            pos: "adjective".into(),
            short_defs: vec!["having or marked by great volume".into()],
            senses: vec![Sense {
                sn: Some("1 a".into()),
                text: "having or marked by great volume or bulk".into(),
                examples: vec!["long voluminous tresses".into()],
                ..Default::default()
            }],
            audio_codes: vec!["volumi02".into()],
            etymology: Some("Late Latin voluminosus".into()),
            date: Some("1611".into()),
            stems: vec!["voluminous".into(), "voluminously".into()],
            labels: Vec::new(),
            raw_json: "[]".into(),
            ..Default::default()
        }
    }

    fn ecdict() -> EcdictRecord {
        EcdictRecord {
            word: "voluminous".into(),
            phonetic: "vəˈluːmɪnəs".into(),
            pos: "a".into(),
            translation: "adj. 多卷的；著作多的；庞大的\\nn. 庞然大物".into(),
            collins: 3,
            oxford: 1,
            tag: "gre toefl".into(),
            bnc: 12345,
            frq: 6789,
            exchange: String::new(),
        }
    }

    #[test]
    fn merge_prefers_mw_for_english_fields() {
        let m = ResultMerger::new().merge(Some(mw()), Some(ecdict()), "voluminous");
        assert_eq!(m.headword_display, "voluminous");
        assert_eq!(m.phonetic, "və-ˈlü-mə-nəs"); // MW 音标优先
        assert_eq!(m.ipa.as_deref(), Some("vəˈluːmɪnəs")); // ECDICT IPA 并列保留
        assert_eq!(m.pos, "adjective");
        assert_eq!(m.short_defs, vec!["having or marked by great volume"]);
        assert_eq!(m.audio_codes, vec!["volumi02"]);
        assert_eq!(m.etymology.as_deref(), Some("Late Latin voluminosus"));
        assert!(m.sources.contains(&SourceTag::Mw));
        assert!(m.sources.contains(&SourceTag::Ecdict));
    }

    #[test]
    fn chinese_always_from_ecdict() {
        let m = ResultMerger::new().merge(Some(mw()), Some(ecdict()), "voluminous");
        let chinese = m.chinese.expect("chinese present");
        assert!(chinese.contains("多卷的"));
        assert!(chinese.contains("庞然大物"));
        // 词频/考试标签来自 ECDICT
        assert_eq!(m.freq, Some((12345, 6789, 3, 1)));
        assert_eq!(m.exam_tags, vec!["gre", "toefl"]);
    }

    #[test]
    fn offline_only_falls_back_to_ecdict() {
        let m = ResultMerger::new().merge(None, Some(ecdict()), "voluminous");
        assert_eq!(m.phonetic, "vəˈluːmɪnəs");
        assert_eq!(m.pos, "a");
        assert_eq!(m.sources, vec![SourceTag::Ecdict]);
        // 离线模式：中文释义行充当 short_defs
        assert!(!m.short_defs.is_empty());
        assert!(m.short_defs[0].contains("多卷的"));
        assert_eq!(m.etymology, None);
    }

    #[test]
    fn ecdict_lines_carry_parsed_pos() {
        let mut e = ecdict();
        e.translation = "n. 庞然大物\\nvt. 使成卷\\nvi. 变成卷".into();
        let m = ResultMerger::new().merge(None, Some(e), "voluminous");
        let poses: Vec<&str> = m.senses.iter().map(|s| s.pos.as_deref().unwrap_or("")).collect();
        assert_eq!(poses, vec!["noun", "verb", "verb"]);
        // 前缀已剥离
        assert_eq!(m.senses[0].text, "庞然大物");
        assert_eq!(m.senses[1].text, "使成卷");
        // 无词性行保持原样
        let mut e2 = ecdict();
        e2.translation = "无前缀行".into();
        let m2 = ResultMerger::new().merge(None, Some(e2), "w");
        assert_eq!(m2.senses[0].pos, None);
        assert_eq!(m2.senses[0].text, "无前缀行");
    }

    #[test]
    fn mw_only_keeps_english() {
        let m = ResultMerger::new().merge(Some(mw()), None, "voluminous");
        assert_eq!(m.chinese, None);
        assert_eq!(m.freq, None);
        assert!(m.exam_tags.is_empty());
        assert_eq!(m.sources, vec![SourceTag::Mw]);
    }

    #[test]
    fn both_missing_yields_empty_entry() {
        let m = ResultMerger::new().merge(None, None, "zzz");
        assert_eq!(m.headword_display, "zzz");
        assert!(m.sources.is_empty());
        assert!(m.short_defs.is_empty());
    }

    /// MW remain 实测结构：remain:1（动词）+ remain:2（名词）同形合并，
    /// "remain to be seen" / "right to remain silent"（hw 含空格）分离为
    /// 短语词条，不混入 senses；raw_json 变为全 entry 数组。
    #[test]
    fn combine_separates_phrase_entries_and_merges_homographs() {
        let mk = |hw: &str, id: &str, fl: &str, sns: Vec<&str>| MwEntry {
            headword: hw.into(),
            pos: fl.into(),
            senses: sns
                .iter()
                .map(|sn| Sense {
                    sn: Some(sn.to_string()),
                    ..Default::default()
                })
                .collect(),
            meta: Some(crate::domain::types::MwMeta {
                id: id.into(),
                ..Default::default()
            }),
            // raw 与结构体同构（含 def.sseq），保证重放还原
            raw_json: {
                let senses = sns
                    .iter()
                    .map(|sn| format!("[\"sense\",{{\"sn\":\"{sn}\",\"dt\":[[\"text\",\"d\"]]}}]"))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{{\"meta\":{{\"id\":\"{id}\"}},\"hwi\":{{\"hw\":\"{hw}\"}},\"fl\":\"{fl}\",\"def\":[{{\"sseq\":[[{senses}]]}}]}}"
                )
            },
            ..Default::default()
        };
        let entries = vec![
            mk("remain", "remain:1", "verb", vec!["1 a", "b", "2", "3"]),
            mk("remain", "remain:2", "noun", vec!["1", "2", "3"]),
            mk("remain to be seen", "remain to be seen", "phrase", vec![]),
            mk("right to remain silent", "right to remain silent", "phrase", vec![]),
        ];
        let combined = combine_entries(entries);
        // 同形合并：4 + 3 个义项；短语不进 senses
        assert_eq!(combined.senses.len(), 7);
        assert_eq!(combined.phrases.len(), 2);
        assert_eq!(combined.phrases[0].headword, "remain to be seen");
        assert_eq!(combined.phrases[0].pos, "phrase");
        // raw_json 是全 entry 数组，可再次重放
        let replayed = crate::infra::mw::parse_cached_entries(&combined.raw_json);
        assert_eq!(replayed.len(), 4);
        let recombined = combine_entries(replayed);
        assert_eq!(recombined.senses.len(), 7);
        assert_eq!(recombined.phrases.len(), 2);
    }

    #[test]
    fn headword_from_ecdict_when_mw_missing() {
        let m = ResultMerger::new().merge(None, Some(ecdict()), "voluminous");
        assert_eq!(m.headword_display, "voluminous");
    }

    #[test]
    fn empty_mw_phonetic_falls_back() {
        let mut e = mw();
        e.phonetic = String::new();
        let m = ResultMerger::new().merge(Some(e), Some(ecdict()), "voluminous");
        assert_eq!(m.phonetic, "vəˈluːmɪnəs");
    }
}
