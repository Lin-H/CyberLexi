//! 查询归一化：trait + 可扩展 pipeline。
//!
//! Phase 1 仅启用 L1（trim + 小写）。Phase 2 可在不改调用方的情况下
//! 向 [`Normalizer::with_step`] 追加 L2（屈折还原）/ L3（变体归一）步骤。

use super::types::Lang;

/// 归一化 pipeline 的单个步骤。
pub trait NormalizerStep: Send + Sync {
    fn name(&self) -> &'static str;
    /// 步骤内完成的输出会传给下一个步骤；`key` 的生成是全链顺序应用。
    fn apply(&self, input: &str) -> String;
}

/// L1-a：去除首尾空白并折叠连续空白为单个空格。
pub struct TrimCollapseStep;

impl NormalizerStep for TrimCollapseStep {
    fn name(&self) -> &'static str {
        "trim_collapse"
    }
    fn apply(&self, input: &str) -> String {
        input.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// L1-b：全小写（Unicode 感知）。
pub struct LowercaseStep;

impl NormalizerStep for LowercaseStep {
    fn name(&self) -> &'static str {
        "lowercase"
    }
    fn apply(&self, input: &str) -> String {
        input.to_lowercase()
    }
}

/// 归一化结果：`display` 保留用户原始输入（去首尾空白）用于回显，
/// `key` 为归一化查询键（缓存/词典主键），`lang` 决定走英文链或中文链。
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedQuery {
    pub display: String,
    pub key: String,
    pub lang: Lang,
}

/// 可扩展归一化器。
pub struct Normalizer {
    steps: Vec<Box<dyn NormalizerStep>>,
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::l1()
    }
}

impl Normalizer {
    /// Phase 1 的 L1 归一化。
    pub fn l1() -> Self {
        Normalizer {
            steps: vec![Box::new(TrimCollapseStep), Box::new(LowercaseStep)],
        }
    }

    /// 追加步骤（Phase 2 在 L1 后插入 L2/L3 时使用）。
    pub fn with_step(mut self, step: Box<dyn NormalizerStep>) -> Self {
        self.steps.push(step);
        self
    }

    pub fn step_names(&self) -> Vec<&'static str> {
        self.steps.iter().map(|s| s.name()).collect()
    }

    /// 归一化：空白输入返回 key 为空、lang 英文（由调用方拒绝空查询）。
    pub fn normalize(&self, raw: &str) -> NormalizedQuery {
        let display = raw.trim().to_string();
        let mut key = display.clone();
        for step in &self.steps {
            key = step.apply(&key);
        }
        let lang = Lang::detect(&display);
        NormalizedQuery { display, key, lang }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_lowercases() {
        let n = Normalizer::l1();
        let q = n.normalize("  Hello   WORLD  ");
        assert_eq!(q.key, "hello world");
        assert_eq!(q.display, "Hello   WORLD");
    }

    #[test]
    fn collapses_inner_whitespace() {
        let n = Normalizer::l1();
        assert_eq!(n.normalize("a \t b\n c").key, "a b c");
    }

    #[test]
    fn detects_english_chain() {
        let n = Normalizer::l1();
        assert_eq!(n.normalize("Hello").lang, Lang::English);
        // 混合字母与汉字：含 ASCII 字母且非纯汉字 → 英文链
        assert_eq!(n.normalize("ok了").lang, Lang::English);
        // 纯数字/符号走英文链兜底
        assert_eq!(n.normalize("123").lang, Lang::English);
    }

    #[test]
    fn detects_chinese_chain() {
        let n = Normalizer::l1();
        assert_eq!(n.normalize("你好").lang, Lang::Chinese);
        assert_eq!(n.normalize("世界，你好").lang, Lang::Chinese);
        // 中文标点 + 汉字，无 ASCII 字母
        assert_eq!(n.normalize("你好？").lang, Lang::Chinese);
    }

    #[test]
    fn chinese_key_not_lowercased_meaningfully() {
        // to_lowercase 对汉字恒等，中文 key 与 display 一致
        let n = Normalizer::l1();
        let q = n.normalize(" 你好 ");
        assert_eq!(q.key, "你好");
        assert_eq!(q.display, "你好");
    }

    #[test]
    fn pipeline_is_extensible() {
        struct DashToSpace;
        impl NormalizerStep for DashToSpace {
            fn name(&self) -> &'static str {
                "dash_to_space"
            }
            fn apply(&self, input: &str) -> String {
                input.replace('-', " ")
            }
        }
        let n = Normalizer::l1().with_step(Box::new(DashToSpace));
        assert_eq!(n.step_names(), ["trim_collapse", "lowercase", "dash_to_space"]);
        assert_eq!(n.normalize("E-Mail").key, "e mail");
    }

    #[test]
    fn empty_input() {
        let n = Normalizer::l1();
        let q = n.normalize("   ");
        assert_eq!(q.key, "");
        assert_eq!(q.lang, Lang::English);
    }
}
