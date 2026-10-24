//! MW running-text markup → 纯文本。
//!
//! MW 定义/例句文本中大量使用 `{bc}`、`{it}...{/it}`、`{sx|word||1}` 之类
//! token（docs/Merriam-Webster.md §2.29）。Phase 1 采用纯文本渲染：
//! 包裹型 token 去壳保留内容，替换型 token 输出等价文本，交叉引用 token
//! 取链接文字。富文本（粗体/斜体/超链接）留待 Phase 2。

/// 将 MW markup 文本转换为可展示纯文本。
pub fn to_plain(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = input[i..].chars().next().unwrap();
        if c == '{' {
            // 找到匹配的 '}'（MW token 内部不再嵌套 '{'）
            match input[i + 1..].find('}') {
                Some(end_off) => {
                    let token_body = &input[i + 1..i + 1 + end_off];
                    // {bc} 特殊处理：句首丢弃（冒号语义属于词性引导）；
                    // 句中输出去掉前导空格的 ": "。
                    if token_body == "bc" {
                        if !out.trim().is_empty() {
                            if out.ends_with(' ') {
                                out.pop();
                            }
                            out.push_str(": ");
                        }
                    } else {
                        out.push_str(&render_token(token_body));
                    }
                    i += 1 + end_off + 1;
                }
                None => {
                    // 未闭合：原样保留
                    out.push_str(&input[i..]);
                    break;
                }
            }
        } else {
            out.push(c);
            i += c.len_utf8();
        }
    }
    out.trim().to_string()
}

/// 解析单个 token 内部 `name|f2|f3|f4` 并给出替代文本。
fn render_token(body: &str) -> String {
    // 闭合标签：{/dx_def} 补右括号，其余闭壳直接丢弃
    if let Some(close) = body.strip_prefix('/') {
        return match close {
            "dx_def" => ")".to_string(),
            _ => String::new(),
        };
    }
    let (name, rest) = match body.find('|') {
        Some(p) => (&body[..p], &body[p + 1..]),
        None => (body, ""),
    };
    let fields: Vec<&str> = rest.split('|').collect();
    let f2 = fields.first().copied().unwrap_or("");
    let f3 = fields.get(1).copied().unwrap_or("");
    let f4 = fields.get(2).copied().unwrap_or("");
    let _ = f3; // Phase 1 不使用 field 3（交叉引用目标 id）

    match name {
        // 标点
        "ldquo" => "\u{201C}".to_string(),
        "rdquo" => "\u{201D}".to_string(),
        "p_br" => "\n".to_string(),

        // date sense：date 字段专用；字段全空 → 无输出（文档 §2.29.5）。
        // Phase 1 只保留日期本身，忽略 sense 定位文本。
        "ds" => String::new(),

        // 交叉引用：取链接文字（field 2），sx/dxt 的 field 4 是跟在链接后的
        // 义项号，非空则附加。
        "sx" | "dxt" => {
            let mut s = f2.to_string();
            if !f4.is_empty() {
                s.push(' ');
                s.push_str(f4);
            }
            s
        }
        "a_link" | "d_link" | "i_link" | "et_link" | "mat" => f2.to_string(),

        // 交叉引用分组（文档 §2.29.3：dx/dx_ety/ma 前显示 em-dash，
        // ma 后接 "more at"；dx_def 显示在括号内）
        "dx" | "dx_ety" => "\u{2014} ".to_string(),
        "ma" => "\u{2014} more at ".to_string(),
        "dx_def" => "(".to_string(),

        // 包裹型分组/标记 token：无自身文本，内容在外层流中已保留
        "b" | "it" | "inf" | "sup" | "sc" | "wi" | "phrase" | "qword" | "parahw"
        | "gloss" => String::new(),

        // 未知 token：保守去壳，避免把 markup 泄漏到 UI
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_colon_and_text() {
        // 句首 {bc} 丢弃（冒号语义属于词性引导）；句中 {bc} 输出 ": "
        assert_eq!(
            to_plain("{bc}having or marked by great volume"),
            "having or marked by great volume"
        );
        assert_eq!(to_plain("{bc}a {a_link|backward} somersault"), "a backward somersault");
        assert_eq!(to_plain("numerous {bc}filling"), "numerous: filling");
    }

    #[test]
    fn italic_and_wi_stripped_content_kept() {
        assert_eq!(
            to_plain("Late Latin {it}voluminosus{\\/it}, from Latin {it}volumin-, volumen{\\/it}"),
            "Late Latin voluminosus, from Latin volumin-, volumen"
        );
        // JSON 中 "\\/" 反序列化后是 "/"，两种形态都要稳：
        assert_eq!(
            to_plain("Late Latin {it}voluminosus{/it}"),
            "Late Latin voluminosus"
        );
    }

    #[test]
    fn sx_with_sense_number() {
        assert_eq!(
            to_plain("{bc}having or marked by great volume {bc}{sx|large||} "),
            "having or marked by great volume: large"
        );
    }

    #[test]
    fn sx_with_field4() {
        assert_eq!(to_plain("{bc}{sx|dance||3}"), "dance 3");
    }

    #[test]
    fn d_link_keeps_text() {
        assert_eq!(
            to_plain("a state or condition in which something is {d_link|absent|absent:1}"),
            "a state or condition in which something is absent"
        );
    }

    #[test]
    fn a_link() {
        assert_eq!(
            to_plain("one that {a_link|monitors} or is used in {a_link|monitoring}: such as"),
            "one that monitors or is used in monitoring: such as"
        );
    }

    #[test]
    fn dxt_inside_dx() {
        // 文档 §2.29.3：dx 前显示 em-dash
        assert_eq!(
            to_plain("{bc}a bird of any kind {dx}compare {dxt|waterfowl||}, {dxt|wildfowl||}{/dx}"),
            "a bird of any kind \u{2014} compare waterfowl, wildfowl"
        );
    }

    #[test]
    fn dx_def_in_parens() {
        assert_eq!(
            to_plain("any of several domesticated {dx_def}see {dxt|domesticate:1||2}{/dx_def} birds"),
            "any of several domesticated (see domesticate:1 2) birds"
        );
    }

    #[test]
    fn ma_more_at() {
        assert_eq!(
            to_plain("Latin monere to warn {ma}{mat|mind|}{/ma}"),
            "Latin monere to warn \u{2014} more at mind"
        );
    }

    #[test]
    fn gloss_in_brackets_is_dropped_shell() {
        // gloss 语义上应显示为 [..]；Phase 1 纯文本选择去壳保留内容
        assert_eq!(
            to_plain("an {wi}absence{\\/wi} {gloss}=lack{\\/gloss} of detail"),
            "an absence =lack of detail"
        );
    }

    #[test]
    fn date_ds_token_removed() {
        assert_eq!(to_plain("1611{ds||3||}"), "1611");
        assert_eq!(to_plain("before 12th century{ds|t|1|a|1}"), "before 12th century");
    }

    #[test]
    fn quotes_and_p_br() {
        assert_eq!(to_plain("{ldquo}hi{rdquo}{p_br}line2"), "\u{201C}hi\u{201D}\nline2");
    }

    #[test]
    fn inf_sup_subscripts() {
        assert_eq!(
            to_plain("an ion NH{inf}4{\\/inf}{sup}+{\\/sup} derived"),
            "an ion NH4+ derived"
        );
    }

    #[test]
    fn unclosed_brace_kept_verbatim() {
        assert_eq!(to_plain("broken {token"), "broken {token");
    }

    #[test]
    fn unknown_token_stripped() {
        assert_eq!(to_plain("keep {zzz|drop|this} tail"), "keep  tail");
    }

    #[test]
    fn phrase_and_qword() {
        assert_eq!(
            to_plain("{phrase}In the absence of{\\/phrase} reform, progress will be slow."),
            "In the absence of reform, progress will be slow."
        );
    }
}
