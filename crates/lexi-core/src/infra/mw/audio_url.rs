//! MW 音频码 → MP3 URL 纯函数。
//!
//! 规则来自官方文档（docs/Merriam-Webster.md §2.6 Pronunciations）：
//! `https://media.merriam-webster.com/audio/prons/{lang}/{country}/mp3/{subdir}/{code}.mp3`
//! - lang/country：collegiate（ref="c"）为 en/us；Spanish-English（ref="es"）为 es/me
//! - subdir：code 以 "bix" 开头 → "bix"；以 "gg" 开头 → "gg"；
//!   以数字或标点（含 '_'）开头 → "number"；否则为 code 首字母
//!
//! 注意：文档规则已官方确认，但子目录细节（尤其前缀特例是否新增）
//! 仍标注「待 MW 实测」——单测覆盖文档给出的全部已知形态。

/// 构建 MW 音频 MP3 URL。`code` 为空返回 None。
///
/// `ref_` 为 sound.ref（"c" = collegiate / "es" = Spanish-English）。
pub fn build_audio_url(code: &str, ref_: &str) -> Option<String> {
    if code.is_empty() {
        return None;
    }
    let (lang, country) = match ref_ {
        "es" => ("es", "me"),
        _ => ("en", "us"),
    };
    Some(format!(
        "https://media.merriam-webster.com/audio/prons/{lang}/{country}/mp3/{sub}/{code}.mp3",
        sub = subdir(code),
    ))
}

/// 音频子目录规则（文档 §2.6）。
fn subdir(code: &str) -> String {
    let first = code.chars().next().unwrap_or_default();
    if code.starts_with("bix") {
        "bix".to_string()
    } else if code.starts_with("gg") {
        "gg".to_string()
    } else if !first.is_ascii_alphabetic() {
        // 数字或标点（含 '_' 开头）→ number
        "number".to_string()
    } else {
        first.to_ascii_lowercase().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文档示例 1：数字开头 → number
    #[test]
    fn doc_example_numeric() {
        assert_eq!(
            build_audio_url("3d000001", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/number/3d000001.mp3")
        );
    }

    /// 文档示例 2：普通字母 → 首字母目录
    #[test]
    fn doc_example_letter() {
        assert_eq!(
            build_audio_url("pajama02", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/p/pajama02.mp3")
        );
    }

    /// 文档示例 3：Spanish-English → es/me
    #[test]
    fn doc_example_spanish() {
        assert_eq!(
            build_audio_url("hola001sp", "es").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/es/me/mp3/h/hola001sp.mp3")
        );
    }

    /// 文档规则：bix 前缀 → bix 目录
    #[test]
    fn bix_prefix() {
        assert_eq!(
            build_audio_url("bixdod04", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/bix/bixdod04.mp3")
        );
    }

    /// 文档规则：gg 前缀 → gg 目录
    #[test]
    fn gg_prefix() {
        assert_eq!(
            build_audio_url("ggword01", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/gg/ggword01.mp3")
        );
    }

    /// spec 原假设中的 '_' 开头：文档归为标点 → number（非 dir='_'）
    #[test]
    fn underscore_goes_to_number() {
        assert_eq!(
            build_audio_url("_test01", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/number/_test01.mp3")
        );
    }

    /// 大写首字母 → 小写目录（防御性；MW code 实际均小写）
    #[test]
    fn uppercase_first_char_lowered() {
        assert_eq!(
            build_audio_url("Test0001", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/t/Test0001.mp3")
        );
    }

    #[test]
    fn empty_code_is_none() {
        assert_eq!(build_audio_url("", "c"), None);
    }

    /// 真实词条（voluminous）音频码。
    #[test]
    fn real_entry_voluminous() {
        assert_eq!(
            build_audio_url("volumi02", "c").as_deref(),
            Some("https://media.merriam-webster.com/audio/prons/en/us/mp3/v/volumi02.mp3")
        );
    }
}
