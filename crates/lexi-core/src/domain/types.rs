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

/// MW 释义（一条 sense，含例句与细分义）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Sense {
    /// sense number，如 "1 a" / "(2)"
    pub sn: Option<String>,
    /// 本 sense 所属 def 块的词性（MW def[].fl；多词性词条各块不同，
    /// 如 test 名词条与动词条）。缺省回退 entry 级 fl。
    #[serde(default)]
    pub pos: Option<String>,
    /// 定义文本（MW markup 已转纯文本）
    pub text: String,
    /// verbal illustrations 例句（markup 已转纯文本）
    pub examples: Vec<String>,
    /// 细分义（sdsense）：作为 sense 内子块渲染（"↳ also: ..."）
    #[serde(default)]
    pub subsenses: Vec<SubSense>,
}

/// sdsense 细分义（sd 分隔词 + 定义 + 例句）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SubSense {
    /// sd 分隔词，如 "also" / "specifically"
    pub label: String,
    pub text: String,
    pub examples: Vec<String>,
}

/// MW meta 对象（id/sort/src/section/stems/offensive）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MwMeta {
    pub id: String,
    pub src: String,
    pub section: String,
    pub sort: String,
    pub stems: Vec<String>,
    pub offensive: bool,
}

/// hwi.ins 屈折变体（if 词形）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Inflection {
    /// 变体词形（'/' 分隔符已清理）
    pub display: String,
}

/// uros 派生词条（defined run-on）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RunOn {
    /// 派生词词形（'*' 分隔符已清理）
    pub ure: String,
    /// 词性（如 adverb / noun）
    pub fl: String,
    /// uros.prs[0].mw 音标
    pub phonetic: String,
}

/// cxs 交叉引用（cxref，如 "also"/"synonym of"）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CrossRef {
    /// cxl 引用说明
    pub label: String,
    /// cxref 目标词列表
    pub refs: Vec<String>,
}

/// MW 返回的独立短语词条（如查询 remain 附带的 "remain to be seen"）。
/// hw/meta.id 含空格即为短语——不并入义项，独立展示。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PhraseEntry {
    pub headword: String,
    /// 词性（通常 "phrase"）
    pub pos: String,
    /// 首条释义（shortdef 优先）
    pub definition: String,
    pub example: Option<String>,
}

/// Merriam-Webster Collegiate 词条（我方使用的子集，完整报文落 `raw_json`）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MwEntry {
    /// hwi.hw 去除音节分隔符 '*' 后的词形
    pub headword: String,
    /// prs[0].mw 音标
    pub phonetic: String,
    /// fl 词性
    pub pos: String,
    /// shortdef
    pub short_defs: Vec<String>,
    /// def[].sseq 展开后的释义序列（sense/bs/pseq；sdsense 挂到 subsenses）
    pub senses: Vec<Sense>,
    /// hwi.prs 与 uros.prs 中的 sound.audio 音频码
    pub audio_codes: Vec<String>,
    pub etymology: Option<String>,
    pub date: Option<String>,
    pub stems: Vec<String>,
    pub labels: Vec<String>,
    /// hom：同形异义词序号（如 "1"/"2"）
    #[serde(default)]
    pub hom: Option<String>,
    /// meta 对象（id/src/section/sort/stems/offensive）
    #[serde(default)]
    pub meta: Option<MwMeta>,
    /// hwi.altprs 备用音标
    #[serde(default)]
    pub alt_phonetics: Vec<String>,
    /// hwi.ins 屈折变体
    #[serde(default)]
    pub inflections: Vec<Inflection>,
    /// uros 派生词条
    #[serde(default)]
    pub run_ons: Vec<RunOn>,
    /// cxs 交叉引用
    #[serde(default)]
    pub cross_refs: Vec<CrossRef>,
    /// 响应中附带的独立短语词条（combine 时分离）
    #[serde(default)]
    pub phrases: Vec<PhraseEntry>,
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
    /// IPA 风格音标（来自 ECDICT phonetic；MW 音标在 `phonetic`）
    #[serde(default)]
    pub ipa: Option<String>,
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
    /// 同形异义词序号（MW hom）
    #[serde(default)]
    pub hom: Option<String>,
    /// 词干/词形族（MW meta.stems）
    #[serde(default)]
    pub stems: Vec<String>,
    /// MW meta（src/section/sort/id/offensive）
    #[serde(default)]
    pub meta: Option<MwMeta>,
    /// 备用音标（hwi.altprs）
    #[serde(default)]
    pub alt_phonetics: Vec<String>,
    /// 屈折变体（hwi.ins）
    #[serde(default)]
    pub inflections: Vec<Inflection>,
    /// 派生词条（uros）
    #[serde(default)]
    pub run_ons: Vec<RunOn>,
    /// 交叉引用（cxs）
    #[serde(default)]
    pub cross_refs: Vec<CrossRef>,
    /// 同词附带的独立短语词条（remain to be seen 等）
    #[serde(default)]
    pub phrases: Vec<PhraseEntry>,
    /// 冒犯性标记（meta.offensive）
    #[serde(default)]
    pub offensive: bool,
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

/// 单词本卡片摘要（离线聚合：cache + ECDICT，零网络）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordSummary {
    pub word_key: String,
    pub headword: String,
    /// IPA/MW 音标（可用者优先）
    pub phonetic: String,
    pub pos: String,
    /// 首行中文释义（缺则 shortdef 首条）
    pub definition: String,
    /// 考试标签（ECDICT tag）
    pub exam_tags: Vec<String>,
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
