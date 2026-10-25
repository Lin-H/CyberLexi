//! 字段级合并：MW 在线结果 + ECDICT 离线记录 → [`MergedEntry`]。
//!
//! 规则（spec 查询链路第 5 步）：
//! - 音标/词性/例句/音频/词源/标签：优先 MW，缺则 ECDICT；
//! - 中文释义：永远来自 ECDICT；
//! - 词频/考试标签：ECDICT；
//! - headword 以 MW `hwi.hw` 为准回写显示。

use super::types::{EcdictRecord, MergedEntry, MwEntry, Sense, SourceTag};

#[derive(Default)]
pub struct ResultMerger;

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
                        examples: Vec::new(),
                        subsenses: Vec::new(),
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
        let (hom, stems, meta, alt_phonetics, inflections, run_ons, cross_refs, offensive) =
            match &mw {
                Some(m) => (
                    m.hom.clone(),
                    m.stems.clone(),
                    m.meta.clone(),
                    m.alt_phonetics.clone(),
                    m.inflections.clone(),
                    m.run_ons.clone(),
                    m.cross_refs.clone(),
                    m.meta.as_ref().is_some_and(|x| x.offensive),
                ),
                None => Default::default(),
            };

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
            offensive,
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
