//! 核心领域类型：贯穿查询链路的统一数据结构。

use serde::{Deserialize, Serialize};

/// 查询语言链路判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lang {
    English,
    Chinese,
}

impl Lang {
    /// 规则（spec）：含 ASCII 字母且非纯汉字 → 英文链；否则中文链。
    /// 混合输入（字母 + 汉字，如 "ok了"）判英文链；纯汉字/中文标点判中文链；
    /// 两者皆无（纯数字/符号）判英文链兜底。
    pub fn detect(input: &str) -> Lang {
        let has_ascii_alpha = input.chars().any(|c| c.is_ascii_alphabetic());
        let has_cjk = input.chars().any(is_cjk);
        if has_cjk && !has_ascii_alpha {
            Lang::Chinese
        } else {
            Lang::English
        }
    }
}

/// CJK 统一表意文字（含扩展 A）。
pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

/// 词条数据来源标签（词条头展示用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceTag {
    /// Merriam-Webster 在线结果（含缓存回放的 MW 数据）
    Mw,
    /// ECDICT 离线兜底
    Ecdict,
    /// CC-CEDICT 中文链
    Cedict,
}

impl SourceTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceTag::Mw => "MW",
            SourceTag::Ecdict => "ecdict",
            SourceTag::Cedict => "cedict",
        }
    }
}

/// MW 释义（一条 sense，含例句）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sense {
    /// sense number，如 "1 a" / "(2)"
    pub sn: Option<String>,
    /// 定义文本（MW markup 已转纯文本）
    pub text: String,
    /// verbal illustrations 例句（markup 已转纯文本）
    pub examples: Vec<String>,
}

/// Merriam-Webster Collegiate 词条（我方使用的子集，完整报文落 `raw_json`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MwEntry {
    /// hwi.hw 去除音节分隔符 '*' 后的词形
    pub headword: String,
    /// prs[0].mw 音标
    pub phonetic: String,
    /// fl 词性
    pub pos: String,
    /// shortdef
    pub short_defs: Vec<String>,
    /// def[].sseq 展开后的释义序列（sense/bs/pseq/sdsense 均摊平）
    pub senses: Vec<Sense>,
    /// hwi.prs 与 uros.prs 中的 sound.audio 音频码
    pub audio_codes: Vec<String>,
    pub etymology: Option<String>,
    pub date: Option<String>,
    pub stems: Vec<String>,
    pub labels: Vec<String>,
    /// MW 完整 JSON 报文（单 entry；数组场景由 parser 分别产出）
    pub raw_json: String,
}

impl MwEntry {
    /// MW hw 音节分隔表示（如 "vo*lu*mi*nous"）转显示词形。
    pub fn clean_headword(hw: &str) -> String {
        hw.replace('*', "")
    }
}

/// ECDICT 记录（英文离线兜底）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EcdictRecord {
    pub word: String,
    pub phonetic: String,
    pub pos: String,
    /// 中文释义，'\n' 分隔多行
    pub translation: String,
    pub collins: i32,
    pub oxford: i32,
    /// 考试标签，空格分隔（zk gk cet4 cet6 ky toefl ielts gre）
    pub tag: String,
    pub bnc: i32,
    pub frq: i32,
    /// 词形变换（本期不启用归一，仅预留存储）
    pub exchange: String,
}

impl EcdictRecord {
    /// translation 按行拆分；空则空 vec。
    pub fn translation_lines(&self) -> Vec<String> {
        self.translation
            .split("\\n")
            .flat_map(|s| s.split('\n'))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// 考试标签拆分为列表。
    pub fn exam_tags(&self) -> Vec<String> {
        self.tag
            .split_whitespace()
            .map(str::to_string)
            .collect()
    }
}

/// CC-CEDICT 记录（中文→英文兜底）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CedictRecord {
    pub traditional: String,
    pub simplified: String,
    /// 带声调数字拼音 "ni3 hao3"
    pub pinyin: String,
    /// 无声调拼音 "ni hao"
    pub pinyin_plain: String,
    /// 英文释义，原样 "/gloss1/gloss2/"
    pub english: String,
}

impl CedictRecord {
    /// "/gloss1/gloss2/" → ["gloss1", "gloss2"]
    pub fn english_glosses(&self) -> Vec<String> {
        self.english
            .split('/')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    }

    /// 首个英文释义截断（候选列表展示用）。
    pub fn first_gloss(&self, max_chars: usize) -> String {
        let g = self
            .english_glosses()
            .first()
            .cloned()
            .unwrap_or_default();
        if g.chars().count() > max_chars {
            g.chars().take(max_chars).collect::<String>() + "…"
        } else {
            g
        }
    }
}

/// 合并后的词条（UI 渲染的唯一来源）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedEntry {
    /// 显示词形：MW hwi.hw 优先，缺则 ECDICT word，再缺则查询原词
    pub headword_display: String,
    /// 归一化查询键（去重/单词本主键）
    pub normalized_key: String,
    pub phonetic: String,
    pub pos: String,
    pub short_defs: Vec<String>,
    pub senses: Vec<Sense>,
    /// 中文释义（永远来自 ECDICT，MW 无中文）
    pub chinese: Option<String>,
    /// 词频 (bnc, frq, collins, oxford)，来自 ECDICT
    pub freq: Option<(i32, i32, i32, i32)>,
    /// 考试标签，来自 ECDICT tag
    pub exam_tags: Vec<String>,
    pub audio_codes: Vec<String>,
    pub etymology: Option<String>,
    pub date: Option<String>,
    /// 来源标签集合
    pub sources: Vec<SourceTag>,
}

/// 中文链候选条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    /// 候选词（CEDICT simp/trad 或 ECDICT word）
    pub word: String,
    pub pinyin: Option<String>,
    /// 简短英文/中文释义（单行）
    pub gloss: String,
}

/// 查询链路统一出口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    /// 英文链完整词条
    Entry(MergedEntry),
    /// 中文链候选列表（≤20 条，点击后复用英文链补全）
    Candidates(Vec<Candidate>),
    /// 未命中；suggestions 来自 MW 拼写建议（可能为空）
    NotFound {
        input: String,
        suggestions: Vec<String>,
    },
}
