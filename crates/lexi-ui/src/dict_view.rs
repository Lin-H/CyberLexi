//! 词典视图：单词 Hero（headword/hom/发音/标签/meta 条）→ 释义卡片
//!（sense/subsense/例句/交叉引用）→ 词源/词形/派生/首见年代 →
//! 右侧信息栏（shortdef/stems/中文/词频/学习档案）。
//! 设计稿 #view-dict（MW API 全字段布局）。

use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::*;

use lexi_core::domain::types::{Candidate, MergedEntry, QueryResult, Sense, SourceTag};
use lexi_core::domain::wordbook::WordStatus;
use lexi_core::events::UiCommand;

use crate::app::AppView;
use crate::theme::{sense_accent, grad, hex, steel, white, with_alpha, Neon};
use crate::widgets::{mono_chip, panel, panel_title, round_action, section_label, solid_pill, tag_pill};

/// 词典视图入口（由 app.rs 在滚动容器内调用）。
pub fn render(view: &mut AppView, cx: &mut Context<AppView>) -> Div {
    let result = view.result.clone();
    match result {
        Some(QueryResult::Entry(entry)) => render_entry(&entry, view, cx),
        Some(QueryResult::Candidates(cands)) => render_candidates(&cands, cx),
        Some(QueryResult::NotFound { input, suggestions }) => {
            render_not_found(&input, &suggestions, cx)
        }
        None => render_placeholder(view, cx),
    }
}

// ============================================================
// 词条
// ============================================================

fn render_entry(entry: &MergedEntry, view: &mut AppView, cx: &mut Context<AppView>) -> Div {
    let in_wordbook = view
        .wordbook
        .iter()
        .any(|i| i.word_key == entry.normalized_key);
    let wb_item = view
        .wordbook
        .iter()
        .find(|i| i.word_key == entry.normalized_key)
        .cloned();

    div()
        .flex()
        .gap(px(26.))
        .max_w(px(1300.))
        // ── 主列 ──
        .child(
            div().flex_1().min_w_0().flex().flex_col().child(hero(entry, in_wordbook, &view.tts_voices, cx))
                .child(body(entry)),
        )
        // ── 右侧信息栏 ──
        .child(aside(entry, wb_item.as_ref()))
}

fn hero(entry: &MergedEntry, in_wordbook: bool, tts_voices: &[String], _cx: &mut Context<AppView>) -> Div {
    let mut head = div().flex().items_center().flex_wrap().gap(px(16.));

    // headword + homograph 上标
    let mut hw_wrap = div().flex().items_baseline().gap(px(6.));
    hw_wrap = hw_wrap.child(
        div()
            .text_size(px(52.))
            .font_weight(FontWeight::EXTRA_BOLD)
            .text_color(hex("#dff6ff"))
            .child(entry.headword_display.clone()),
    );
    if let Some(hom) = &entry.hom {
        hw_wrap = hw_wrap.child(
            div()
                .text_size(px(24.))
                .font_weight(FontWeight::BOLD)
                .font_family(crate::theme::MONO_FONT)
                .text_color(Neon::violet())
                .opacity(0.9)
                .child(superscript(hom)),
        );
    }
    head = head.child(hw_wrap);

    // 播放按钮（40px 圆形；首个音频码，其余已预取缓存）
    if let Some(code) = entry.audio_codes.first() {
        let code = code.clone();
        head = head.child(round_action("hero-play", IconName::Play, Neon::cyan(), move |_, _, _| {
            crate::send_global(UiCommand::PlayAudio { code: code.clone(), ref_: "c".into() });
        }));
    }

    // 收藏 / 已收藏
    {
        let key = entry.normalized_key.clone();
        let headword = entry.headword_display.clone();
        let (icon, color) = if in_wordbook {
            (IconName::StarFill, Neon::magenta())
        } else {
            (IconName::Star, Neon::magenta())
        };
        head = head.child(round_action("hero-star", icon, color, move |_, _, _| {
            if in_wordbook {
                crate::send_global(UiCommand::RemoveFromWordBook(key.clone()));
            } else {
                crate::send_global(UiCommand::AddToWordBook {
                    key: key.clone(),
                    headword: headword.clone(),
                });
            }
        }));
    }

    // 音标行：MW 音标 + 多人声小圆钮（MW 录音变体 + TTS 人声，共 ≤3）+ IPA + 备用
    let mut pr_row = div().flex().items_center().flex_wrap().gap(px(10.)).mt(px(14.));
    if !entry.phonetic.is_empty() || !entry.audio_codes.is_empty() || !tts_voices.is_empty() {
        if !entry.phonetic.is_empty() {
            pr_row = pr_row.child(phonetic_pill("MW", &entry.phonetic, false));
        }
        pr_row = pr_row.child(audio_variants(entry, tts_voices));
    }
    if let Some(ipa) = &entry.ipa {
        if !ipa.is_empty() && *ipa != entry.phonetic {
            pr_row = pr_row.child(phonetic_pill("IPA", ipa, true));
        }
    }
    for alt in entry.alt_phonetics.iter().take(2) {
        pr_row = pr_row.child(phonetic_pill("ALT", alt, true));
    }

    // 词性 + 等级 + 常用度 + offensive + 来源
    let mut meta = div().flex().items_center().flex_wrap().gap(px(10.)).mt(px(14.));
    if !entry.pos.is_empty() {
        meta = meta.child(solid_pill(entry.pos.clone(), Neon::cyan()));
    }
    if !entry.exam_tags.is_empty() {
        let joined = entry.exam_tags.iter().map(|t| t.to_uppercase()).collect::<Vec<_>>().join(" · ");
        meta = meta.child(tag_pill(
            joined,
            hex("#e3c6ff"),
            with_alpha(Neon::violet(), 0.4),
            with_alpha(Neon::violet(), 0.14),
        ));
    }
    if let Some(pct) = frequency_score(entry) {
        meta = meta.child(tag_pill(
            format!("常用度 {pct}"),
            hex("#ffc7dc"),
            with_alpha(Neon::magenta(), 0.35),
            with_alpha(Neon::magenta(), 0.10),
        ));
    }
    if entry.offensive {
        meta = meta.child(tag_pill(
            "offensive",
            hex("#ffb0c8"),
            with_alpha(Neon::magenta(), 0.4),
            with_alpha(Neon::magenta(), 0.14),
        ));
    }
    for tag in &entry.sources {
        let (label, color) = match tag {
            SourceTag::Mw => ("MW", Neon::cyan()),
            SourceTag::Ecdict => ("ECDICT", Neon::lime()),
            SourceTag::Cedict => ("CEDICT", Neon::lime()),
        };
        meta = meta.child(tag_pill(
            label,
            color,
            with_alpha(color, 0.35),
            with_alpha(color, 0.08),
        ));
    }

    // meta 信息条（SRC/SECTION/SORT/ID；无 meta 时仅显示 KEY）
    let mut strip = div()
        .flex()
        .flex_wrap()
        .gap(px(18.))
        .mt(px(16.))
        .px(px(14.))
        .py(px(10.))
        .rounded_md()
        .bg(white(0.02))
        .border_1()
        .border_color(steel(0.08))
        .text_xs()
        .font_family(crate::theme::MONO_FONT)
        .text_color(Neon::faint());
    if let Some(m) = &entry.meta {
        let items = [
            ("SRC", m.src.as_str()),
            ("SECTION", m.section.as_str()),
            ("SORT", m.sort.as_str()),
            ("ID", m.id.as_str()),
        ];
        for (k, v) in items {
            if !v.is_empty() {
                strip = strip.child(
                    div()
                        .flex()
                        .gap(px(5.))
                        .child(div().child(k.to_string()))
                        .child(div().text_color(Neon::meta_key()).child(v.to_string())),
                );
            }
        }
    }
    strip = strip.child(
        div()
            .flex()
            .gap(px(5.))
            .child(div().child("KEY"))
            .child(
                div()
                    .text_color(Neon::meta_key())
                    .child(entry.normalized_key.clone()),
            ),
    );

    div()
        .pb(px(26.))
        .mb_6()
        .border_b_1()
        .border_color(with_alpha(Neon::cyan(), 0.10))
        .flex()
        .flex_col()
        .child(head)
        .child(pr_row)
        .child(meta)
        .child(strip)
}

/// MW meta 缺失时不渲染信息条（占位逻辑见 hero）。
fn phonetic_pill(label: &str, text: &str, dim: bool) -> Div {
    let mut pill = div()
        .flex()
        .items_baseline()
        .gap(px(5.))
        .px(px(12.))
        .py(px(3.))
        .rounded_sm()
        .bg(with_alpha(Neon::cyan(), 0.06))
        .border_1()
        .border_color(with_alpha(Neon::cyan(), 0.18))
        .text_color(if dim { with_alpha(Neon::phonetic(), 0.6) } else { Neon::phonetic() });
    pill = pill.child(
        div()
            .text_xs()
            .text_color(Neon::faint())
            .child(label.to_string()),
    );
    pill.child(
        div()
            .text_sm()
            .font_family(crate::theme::MONO_FONT)
            .child(text.to_string()),
    )
}

/// 发音按钮组（最多 3 个）：MW 录音变体优先（青色，多个时显示序号），
/// 不足 3 个用本地 TTS 人声补足（紫罗兰，人声名首字母）。
fn audio_variants(entry: &MergedEntry, tts_voices: &[String]) -> Div {
    let mw: Vec<&String> = entry.audio_codes.iter().take(3).collect();
    let tts: Vec<&String> = tts_voices
        .iter()
        .take(3usize.saturating_sub(mw.len()))
        .collect();
    let total = mw.len() + tts.len();
    let mut row = div().flex().items_center().gap(px(6.));
    for (i, code) in mw.iter().enumerate() {
        let code = (*code).clone();
        let multiple = total > 1;
        row = row.child(
            div()
                .id(("audio", i))
                .cursor_pointer()
                .w(px(28.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(with_alpha(Neon::cyan(), 0.08))
                .border_1()
                .border_color(with_alpha(Neon::cyan(), 0.3))
                .text_color(Neon::cyan())
                .hover(|d| {
                    d.bg(with_alpha(Neon::cyan(), 0.2))
                        .shadow(vec![crate::theme::glow(Neon::cyan(), 0.45, 16.)])
                })
                .on_click(move |_, _, _| {
                    crate::send_global(UiCommand::PlayAudio {
                        code: code.clone(),
                        ref_: "c".into(),
                    });
                })
                .child(if multiple {
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child((i + 1).to_string())
                        .into_any_element()
                } else {
                    Icon::new(IconName::Volume2)
                        .with_size(px(11.))
                        .into_any_element()
                }),
        );
    }
    for (i, voice) in tts.iter().enumerate() {
        let voice = (*voice).clone();
        let text = entry.headword_display.clone();
        let badge = lexi_core::infra::tts::voice_badge(&voice);
        row = row.child(
            div()
                .id(("tts", i))
                .cursor_pointer()
                .w(px(28.))
                .h(px(28.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(with_alpha(Neon::violet(), 0.08))
                .border_1()
                .border_color(with_alpha(Neon::violet(), 0.3))
                .text_color(Neon::violet())
                .hover(|d| {
                    d.bg(with_alpha(Neon::violet(), 0.2))
                        .shadow(vec![crate::theme::glow(Neon::violet(), 0.45, 16.)])
                })
                .on_click(move |_, _, _| {
                    crate::send_global(UiCommand::PlayTts {
                        voice: voice.clone(),
                        text: text.clone(),
                    });
                })
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .font_family(crate::theme::MONO_FONT)
                        .child(badge),
                ),
        );
    }
    row
}

fn body(entry: &MergedEntry) -> Div {
    let mut col = div().flex().flex_col();

    // 释义卡片（current_div：字母义项的归属 division，如 "1 a" 后的
    // "b" 显示为大数字 1 + 小字母 b）
    if !entry.senses.is_empty() {
        col = col.child(section_label("Definition · 释义"));
        let mut list = div().flex().flex_col().gap(px(14.));
        let mut current_div: Option<String> = None;
        for (i, sense) in entry.senses.iter().enumerate() {
            let (num, sub, level) = classify_sn(sense.sn.as_deref(), current_div.as_deref());
            if let Some(d) = div_number(sense.sn.as_deref()) {
                current_div = Some(d);
            }
            list = list.child(sense_card(i, sense, entry, num, sub, level));
        }
        col = col.child(list);
    }

    // 相关短语（MW 附带的独立短语词条，如 remain to be seen）
    if !entry.phrases.is_empty() {
        let mut grid = div().flex().flex_col().gap(px(10.));
        for (pi, ph) in entry.phrases.iter().enumerate() {
            let target = ph.headword.clone();
            let mut card = div()
                .id(("phrase", pi))
                .cursor_pointer()
                .relative()
                .flex()
                .flex_col()
                .gap(px(6.))
                .px(px(16.))
                .py(px(13.))
                .rounded_xl()
                .bg(grad(
                    155.,
                    with_alpha(Neon::violet(), 0.06),
                    with_alpha(hex("#070b13"), 0.6),
                ))
                .border_1()
                .border_color(with_alpha(Neon::violet(), 0.2))
                .hover(|d| {
                    d.border_color(with_alpha(Neon::violet(), 0.5))
                        .shadow(vec![crate::theme::glow(Neon::violet(), 0.2, 24.)])
                })
                .on_click(move |_, _, _| {
                    crate::send_global(UiCommand::Query(target.clone()));
                })
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(10.))
                        .child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(hex("#e3c6ff"))
                                .child(ph.headword.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_family(crate::theme::MONO_FONT)
                                .text_color(Neon::violet())
                                .child(ph.pos.to_uppercase()),
                        ),
                );
            if !ph.definition.is_empty() {
                card = card.child(
                    div()
                        .text_sm()
                        .line_height(relative(1.6))
                        .text_color(Neon::text())
                        .child(ph.definition.clone()),
                );
            }
            if let Some(ex) = &ph.example {
                card = card.child(
                    div()
                        .pl(px(12.))
                        .border_l_2()
                        .border_color(with_alpha(Neon::violet(), 0.25))
                        .child(
                            div()
                                .text_sm()
                                .italic()
                                .line_height(relative(1.6))
                                .text_color(Neon::example())
                                .child(format!("“{ex}”")),
                        ),
                );
            }
            grid = grid.child(card);
        }
        col = col
            .child(div().mt(px(10.)).child(section_label("Phrases · 相关短语")))
            .child(grid);
    }

    // 交叉引用（cxs：近义/参见 chips）
    if !entry.cross_refs.is_empty() {
        col = col.child(
            div()
                .mt(px(18.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(section_label("Cross References · 关联词"))
                .child(cxs_row(entry)),
        );
    }

    // 词源（et）
    if let Some(et) = &entry.etymology {
        col = col.child(
            div()
                .mt(px(14.))
                .px(px(14.))
                .py(px(12.))
                .rounded_md()
                .bg(with_alpha(Neon::amber(), 0.04))
                .border_l_2()
                .border_color(with_alpha(Neon::amber(), 0.4))
                .flex()
                .flex_col()
                .gap(px(4.))
                .child(
                    div()
                        .text_xs()
                        .text_color(Neon::amber())
                        .child("词源 · ETYMOLOGY"),
                )
                .child(
                    div()
                        .text_sm()
                        .italic()
                        .line_height(relative(1.7))
                        .text_color(hex("#b8a88a"))
                        .child(et.clone()),
                ),
        );
    }

    // 屈折变体（hwi.ins）
    if !entry.inflections.is_empty() {
        let mut row = div().flex().flex_wrap().gap_2().mt(px(14.));
        for inf in &entry.inflections {
            row = row.child(mono_chip(inf.display.clone()));
        }
        col = col
            .child(div().mt(px(22.)).child(section_label("Inflections · 词形变化")))
            .child(row);
    }

    // 派生词条（uros）
    if !entry.run_ons.is_empty() {
        let mut grid = div().flex().flex_col().gap(px(2.));
        for (ui_, uro) in entry.run_ons.iter().enumerate() {
            let ure = uro.ure.clone();
            // 行内小喇叭（派生词自己的 MW 录音）
            let play = (!uro.audio_code.is_empty()).then(|| {
                let code = uro.audio_code.clone();
                div()
                    .id(("uro-play", ui_))
                    .cursor_pointer()
                    .w(px(22.))
                    .h(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(with_alpha(Neon::cyan(), 0.08))
                    .border_1()
                    .border_color(with_alpha(Neon::cyan(), 0.25))
                    .text_color(Neon::cyan())
                    .hover(|d| d.bg(with_alpha(Neon::cyan(), 0.18)))
                    .on_click(move |_, _, _| {
                        crate::send_global(UiCommand::PlayAudio {
                            code: code.clone(),
                            ref_: "c".into(),
                        });
                    })
                    .child(Icon::new(IconName::Volume2).with_size(px(10.)))
            });
            let mut row = div()
                .id(("uro", ui_))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(10.))
                .px(px(12.))
                .py(px(9.))
                .rounded_md()
                .bg(white(0.02))
                .border_1()
                .border_color(steel(0.08))
                .hover(|d| {
                    d.bg(with_alpha(Neon::cyan(), 0.04))
                        .border_color(with_alpha(Neon::cyan(), 0.2))
                })
                .on_click({
                    let ure = ure.clone();
                    move |_, _, _| {
                        crate::send_global(UiCommand::Query(ure.clone()));
                    }
                })
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(hex("#cfe6f5"))
                        .child(uro.ure.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(crate::theme::MONO_FONT)
                        .text_color(Neon::violet())
                        .child(uro.fl.clone()),
                )
                .child(div().flex_1())
                .child(
                    div()
                        .text_xs()
                        .font_family(crate::theme::MONO_FONT)
                        .text_color(hex("#5f7f99"))
                        .child(uro.phonetic.clone()),
                );
            if let Some(p) = play {
                row = row.child(p);
            }
            grid = grid.child(row);
        }
        col = col
            .child(div().mt(px(22.)).child(section_label("Run-ons · 派生词条")))
            .child(grid);
    }

    // 首见年代
    if let Some(date) = &entry.date {
        col = col.child(
            div()
                .mt(px(18.))
                .text_xs()
                .font_family(crate::theme::MONO_FONT)
                .text_color(Neon::faint())
                .child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .child(div().child("FIRST KNOWN USE".to_string()))
                        .child(div().text_color(Neon::meta_key()).child(date.clone())),
                ),
        );
    }
    col
}

/// MW sn → (大编号, 小字母/括号标, 层级)。规则（按 MW 实测 remain 报文）：
/// - "N" / "N a"：division 顶级（"1 a" 是 division 1 的首义项，不是二级）
/// - 纯字母 "b"：division 内子义项 → 补当前 division 编号（大数字 N + 小 b）
/// - "(x)"：字母义项下的再细分 → 二级缩进，编号列直接显示 "(x)"
/// - 无 sn：连续/内部义项 → "·" 点标 + 一级缩进（不显示列表序号假编号）
fn classify_sn(sn: Option<&str>, current_div: Option<&str>) -> (Option<String>, String, usize) {
    let Some(sn) = sn.map(str::trim).filter(|s| !s.is_empty()) else {
        return (None, "·".into(), 1);
    };
    if sn.starts_with('(') && sn.ends_with(')') {
        return (None, sn.to_string(), 2);
    }
    let mut tokens = sn.split_whitespace();
    match (tokens.next(), tokens.next()) {
        // "N" 或 "N a"
        (Some(n), sub) if n.chars().all(|c| c.is_ascii_digit()) => (
            Some(n.to_string()),
            sub.unwrap_or_default().to_string(),
            0,
        ),
        // 纯字母 "b"
        (Some(letter), None) if letter.chars().all(|c| c.is_ascii_alphabetic()) => (
            current_div.map(str::to_string),
            letter.to_string(),
            1,
        ),
        _ => (None, sn.to_string(), 0),
    }
}

/// sn 中的 division 编号（用于后续纯字母义项补号）。
fn div_number(sn: Option<&str>) -> Option<String> {
    let sn = sn?.trim();
    let first = sn.split_whitespace().next()?;
    first
        .chars()
        .all(|c| c.is_ascii_digit())
        .then(|| first.to_string())
}

fn sense_card(
    i: usize,
    sense: &Sense,
    entry: &MergedEntry,
    num: Option<String>,
    sub: String,
    level: usize,
) -> Div {
    let accent = sense_accent(i);
    let mut num_col = div()
        .w(if level == 0 { px(52.) } else { px(44.) })
        .flex_none()
        .flex()
        .flex_col()
        .items_end()
        .pt(px(2.))
        .text_color(accent);
    // 无编号（"·" / "(x)"）时小号显示，否则大号
    num_col = num_col.child(
        div()
            .text_size(match (num.is_some(), level) {
                (true, 0) => px(22.),
                (true, _) => px(17.),
                (false, 2) => px(13.),
                _ => px(14.),
            })
            .font_weight(if num.is_some() && level == 0 {
                FontWeight::EXTRA_BOLD
            } else {
                FontWeight::BOLD
            })
            .font_family(crate::theme::MONO_FONT)
            .line_height(relative(1.0))
            .child({
                let label = num.clone().unwrap_or_else(|| sub.clone());
                label
            }),
    );
    // 大编号存在且 sub 是字母（"1"+"a" / "1"+"b"）时小字显示字母；
    // "(x)" 已作为主标显示，不重复
    let sub_letter = (!sub.is_empty()
        && sub != "·"
        && !sub.starts_with('(')
        && num.is_some())
    .then_some(sub.clone());
    if let Some(letter) = sub_letter {
        num_col = num_col.child(
            div()
                .text_xs()
                .mt(px(4.))
                .text_color(Neon::sense_hint())
                .font_family(crate::theme::MONO_FONT)
                .child(letter),
        );
    }

    let mut body = div().flex().flex_col().min_w_0();

    // 顶部：词性徽标 —— sense 级（MW def[].fl，多词性词条各块不同）
    // 优先，缺省回退 entry 级
    let pos = sense
        .pos
        .clone()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| entry.pos.clone());
    if !pos.is_empty() {
        body = body.child(
            div()
                .flex()
                .items_baseline()
                .gap(px(10.))
                .mb(px(8.))
                .child(tag_pill(
                    pos,
                    accent,
                    with_alpha(accent, 0.45),
                    with_alpha(accent, 0.09),
                )),
        );
    }

    // 定义文本
    body = body.child(
        div()
            .text_base()
            .font_weight(FontWeight::MEDIUM)
            .line_height(relative(1.65))
            .text_color(Neon::text_bright())
            .child(sense.text.clone()),
    );

    // 例句
    for ex in &sense.examples {
        body = body.child(
            div()
                .mt(px(12.))
                .pl(px(14.))
                .border_l_2()
                .border_color(with_alpha(Neon::cyan(), 0.22))
                .child(
                    div()
                        .text_sm()
                        .italic()
                        .line_height(relative(1.65))
                        .text_color(Neon::example())
                        .child(format!("“{ex}”")),
                ),
        );
    }

    // 细分义（sdsense）
    for sd in &sense.subsenses {
        let prefix = if sd.label.is_empty() {
            String::new()
        } else {
            format!("{}: ", sd.label)
        };
        body = body.child(
            div()
                .mt(px(12.))
                .px(px(12.))
                .py(px(10.))
                .rounded_md()
                .bg(with_alpha(Neon::violet(), 0.04))
                .border_1()
                .border_dashed()
                .border_color(with_alpha(Neon::violet(), 0.2))
                .text_sm()
                .line_height(relative(1.6))
                .text_color(hex("#b8a8d4"))
                .child(format!("↳ {prefix}{}", sd.text)),
        );
    }

    div()
        .relative()
        .flex()
        .gap(px(18.))
        .pr(px(22.))
        .py(if level == 0 { px(20.) } else { px(14.) })
        // 层级缩进：一级 24px、二级 48px（左缘强调条随卡片整体移动）
        .ml(px(level as f32 * 24.))
        .rounded_xl()
        .bg(grad(
            155.,
            with_alpha(hex("#0f1828"), if level == 2 { 0.5 } else { 0.78 }),
            with_alpha(hex("#070b13"), if level == 2 { 0.45 } else { 0.72 }),
        ))
        .border_1()
        .border_color(if level == 2 {
            with_alpha(Neon::violet(), 0.18)
        } else {
            steel(0.11)
        })
        .shadow(vec![crate::theme::glow(accent, 0.05, 34.)])
        // 左缘强调条
        .child(
            div()
                .absolute()
                .left_0()
                .top(if level == 0 { px(16.) } else { px(10.) })
                .bottom(if level == 0 { px(16.) } else { px(10.) })
                .w(px(2.))
                .rounded_r_sm()
                .bg(if level == 2 { with_alpha(accent, 0.6) } else { accent })
                .shadow(vec![crate::theme::glow(accent, 0.7, 16.)]),
        )
        .child(num_col)
        .child(body)
}

/// MW sn 形式 → 视觉层级：纯数字=0；字母（a/b/…，含 "1 a" 的字母位）=1；
/// 括号 "(2)"=2（义项下的再细分）。

fn cxs_row(entry: &MergedEntry) -> Div {
    let mut row = div().flex().flex_wrap().gap(px(8.));
    for (ci, cxr) in entry.cross_refs.iter().enumerate() {
        let antonym = cxr.label.to_lowercase().contains("antonym");
        let color = if antonym { Neon::magenta() } else { Neon::cyan() };
        row = row.child(
            div()
                .text_xs()
                .text_color(Neon::sense_hint())
                .child(cxr.label.clone()),
        );
        for (ri, r) in cxr.refs.iter().enumerate() {
            let target = r.clone();
            row = row.child(
                div()
                    .id(("cx", ci * 64 + ri))
                    .cursor_pointer()
                    .px(px(11.))
                    .py(px(3.))
                    .rounded_full()
                    .text_xs()
                    .text_color(if antonym { hex("#ffb0c8") } else { hex("#8fe8ff") })
                    .bg(with_alpha(color, 0.06))
                    .border_1()
                    .border_color(with_alpha(color, if antonym { 0.2 } else { 0.22 }))
                    .hover(|d| {
                        d.bg(with_alpha(color, 0.16))
                            .border_color(with_alpha(color, 0.6))
                            .shadow(vec![crate::theme::glow(color, 0.35, 16.)])
                    })
                    .on_click(move |_, _, _| {
                        crate::send_global(UiCommand::Query(target.clone()));
                    })
                    .child(r.clone()),
            );
        }
    }
    row
}

fn aside(
    entry: &MergedEntry,
    wb_item: Option<&lexi_core::domain::wordbook::WordBookItem>,
) -> Div {
    let mut col = div().flex().flex_col().gap(px(15.)).w(px(280.)).flex_none();

    // Short Definitions
    if !entry.short_defs.is_empty() {
        let mut list = div().flex().flex_col().gap(px(10.));
        for (i, d) in entry.short_defs.iter().enumerate() {
            list = list.child(
                div()
                    .flex()
                    .gap(px(10.))
                    .text_sm()
                    .line_height(relative(1.55))
                    .text_color(hex("#9db4c9"))
                    .child(
                        div()
                            .flex_none()
                            .w(px(20.))
                            .text_right()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .font_family(crate::theme::MONO_FONT)
                            .text_color(with_alpha(Neon::cyan(), 0.8))
                            .child((i + 1).to_string()),
                    )
                    .child(
                        // flex_1 + min_w_0：允许收缩，长释义自动换行
                        div().flex_1().min_w_0().child(d.clone()),
                    ),
            );
        }
        col = col.child(panel().child(panel_title("Short Definitions")).child(list));
    }

    // Stems（可点击查询）
    if !entry.stems.is_empty() {
        let mut chips = div().flex().flex_wrap().gap(px(5.));
        for (si, s) in entry.stems.iter().take(16).enumerate() {
            let target = s.clone();
            chips = chips.child(
                div()
                    .id(("stem", si))
                    .cursor_pointer()
                    .px(px(9.))
                    .py(px(2.))
                    .rounded_sm()
                    .text_xs()
                    .font_family(crate::theme::MONO_FONT)
                    .text_color(Neon::muted())
                    .bg(white(0.03))
                    .border_1()
                    .border_color(steel(0.1))
                    .hover(|d| {
                        d.text_color(Neon::cyan())
                            .border_color(with_alpha(Neon::cyan(), 0.4))
                    })
                    .on_click(move |_, _, _| {
                        crate::send_global(UiCommand::Query(target.clone()));
                    })
                    .child(s.clone()),
            );
        }
        col = col.child(panel().child(panel_title("Stems · 词干")).child(chips));
    }

    // 中文释义（ECDICT 全量行；不截断）
    if let Some(chinese) = &entry.chinese {
        let mut lines = div().flex().flex_col().gap(px(6.));
        for line in chinese.split('\n').filter(|s| !s.trim().is_empty()) {
            lines = lines.child(
                div()
                    .text_sm()
                    .line_height(relative(1.6))
                    .text_color(Neon::text())
                    .child(line.trim().to_string()),
            );
        }
        col = col.child(panel().child(panel_title("中文释义 · ECDICT")).child(lines));
    }

    // 词频档案（BNC/COCA/Collins/牛津）
    if let Some((bnc, frq, collins, oxford)) = entry.freq {
        if bnc > 0 || frq > 0 || collins > 0 || oxford > 0 {
            let mut kvs = div().flex().flex_col();
            let kv = |k: &str, v: String| {
                div()
                    .flex()
                    .justify_between()
                    .py(px(7.))
                    .text_sm()
                    .border_b_1()
                    .border_color(white(0.06))
                    .child(div().text_color(hex("#6d8299")).child(k.to_string()))
                    .child(
                        div()
                            .text_xs()
                            .font_family(crate::theme::MONO_FONT)
                            .text_color(hex("#cfe6f5"))
                            .child(v),
                    )
            };
            if bnc > 0 {
                kvs = kvs.child(kv("BNC 排名", format!("#{bnc}")));
            }
            if frq > 0 {
                kvs = kvs.child(kv("COCA 排名", format!("#{frq}")));
            }
            if collins > 0 {
                kvs = kvs.child(kv("Collins 星级", format!("{collins}★")));
            }
            if oxford > 0 {
                kvs = kvs.child(kv("牛津 3000", "YES".into()));
            }
            col = col.child(panel().child(panel_title("词频档案")).child(kvs));
        }
    }

    // 学习档案（本地数据；复习算法为 Phase 2）
    if let Some(item) = wb_item {
        let (pct, status_label, status_color) = match item.status {
            WordStatus::New => (0.10, "生词", Neon::muted()),
            WordStatus::Learning => (0.38, "学习中", Neon::cyan()),
            WordStatus::Familiar => (0.70, "熟悉", Neon::amber()),
            WordStatus::Mastered => (1.00, "已掌握", Neon::lime()),
        };
        let key = item.word_key.clone();
        let next = item.status.next();
        col = col.child(
            panel()
                .child(panel_title("学习档案"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.))
                        .mb(px(12.))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .text_xs()
                                .text_color(hex("#6d8299"))
                                .child(div().child("掌握程度"))
                                .child(
                                    div()
                                        .font_family(crate::theme::MONO_FONT)
                                        .text_color(Neon::cyan_light())
                                        .child(format!("{}%", (pct * 100.0) as u32)),
                                ),
                        )
                        .child(crate::widgets::progress_bar(pct, relative(1.0))),
                )
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .py(px(7.))
                        .text_sm()
                        .border_b_1()
                        .border_color(white(0.06))
                        .child(div().text_color(hex("#6d8299")).child("首次收录"))
                        .child(
                            div()
                                .text_xs()
                                .font_family(crate::theme::MONO_FONT)
                                .text_color(hex("#cfe6f5"))
                                .child(date_str(item.added_at)),
                        ),
                )
                .child(
                    div()
                        .id("aside-status-cycle")
                        .cursor_pointer()
                        .flex()
                        .justify_between()
                        .py(px(7.))
                        .text_sm()
                        .hover(|d| d.bg(white(0.03)))
                        .on_click(move |_, _, _| {
                            crate::send_global(UiCommand::SetWordStatus {
                                key: key.clone(),
                                status: next,
                            });
                        })
                        .child(div().text_color(hex("#6d8299")).child("状态（点击流转）"))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(status_color)
                                .child(status_label.to_string()),
                        ),
                ),
        );
    }

    col
}

// ============================================================
// 候选 / 未找到 / 占位
// ============================================================

/// 中文/拼音候选列表（卡片化行；点击补全详情）。
fn render_candidates(cands: &[Candidate], cx: &mut Context<AppView>) -> Div {
    let mut col = div().flex().flex_col().gap_3();
    col = col.child(
        div()
            .text_sm()
            .text_color(Neon::muted())
            .child(format!("候选 {} 条 · 点击查看词条", cands.len())),
    );
    let mut list = div().flex().flex_col().gap(px(10.));
    for (i, c) in cands.iter().enumerate() {
        let accent = sense_accent(i);
        let word = c.word.clone();
        let mut row = div()
            .id(("cand", i))
            .cursor_pointer()
            .relative()
            .flex()
            .items_center()
            .gap_3()
            .px(px(16.))
            .py(px(12.))
            .rounded_xl()
            .bg(grad(
                155.,
                with_alpha(hex("#0f1828"), 0.78),
                with_alpha(hex("#070b13"), 0.72),
            ))
            .border_1()
            .border_color(steel(0.11))
            .hover(|d| {
                d.border_color(with_alpha(accent, 0.35))
                    .shadow(vec![crate::theme::glow(accent, 0.12, 26.)])
            })
            .on_click(cx.listener(move |v, _, _, cx| {
                v.dispatch_detail(word.clone(), cx);
            }))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(10.))
                    .bottom(px(10.))
                    .w(px(2.))
                    .rounded_r_sm()
                    .bg(accent)
                    .shadow(vec![crate::theme::glow(accent, 0.6, 12.)]),
            );
        row = row.child(
            div()
                .text_base()
                .text_color(Neon::text_bright())
                .child(c.word.clone()),
        );
        if let Some(py) = &c.pinyin {
            row = row.child(
                div()
                    .text_sm()
                    .font_family(crate::theme::MONO_FONT)
                    .text_color(Neon::phonetic())
                    .child(py.clone()),
            );
        }
        row = row.child(div().flex_1());
        row = row.child(
            div()
                .text_sm()
                .max_w(px(420.))
                .text_color(Neon::muted())
                .child(c.gloss.clone()),
        );
        list = list.child(row);
    }
    col.child(list)
}

/// 未找到：MW 拼写建议 chips。
fn render_not_found(input: &str, suggestions: &[String], cx: &mut Context<AppView>) -> Div {
    let mut col = div().flex().flex_col().gap_4().py_2();
    col = col.child(
        div()
            .text_2xl()
            .font_weight(FontWeight::BOLD)
            .text_color(Neon::magenta())
            .child(format!("未找到「{input}」")),
    );
    if !suggestions.is_empty() {
        col = col.child(
            div()
                .text_sm()
                .text_color(Neon::muted())
                .child("你是想查："),
        );
        let mut chips = div().flex().flex_wrap().gap_2();
        for (si, s) in suggestions.iter().enumerate() {
            let text = s.clone();
            chips = chips.child(
                div()
                    .id(("sugg", si))
                    .cursor_pointer()
                    .px_3()
                    .py_1()
                    .rounded_full()
                    .text_sm()
                    .text_color(hex("#8fe8ff"))
                    .bg(with_alpha(Neon::cyan(), 0.06))
                    .border_1()
                    .border_color(with_alpha(Neon::cyan(), 0.22))
                    .hover(|d| {
                        d.bg(with_alpha(Neon::cyan(), 0.16))
                            .border_color(with_alpha(Neon::cyan(), 0.6))
                            .shadow(vec![crate::theme::glow(Neon::cyan(), 0.35, 16.)])
                    })
                    .on_click(cx.listener(move |v, _, _, cx| {
                        v.dispatch_query(text.clone(), cx);
                    }))
                    .child(s.clone()),
            );
        }
        col = col.child(chips);
    }
    col
}

/// 空态：品牌位 + 使用提示 + 随机单词入口。
fn render_placeholder(_view: &mut AppView, _cx: &mut Context<AppView>) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(10.))
        .py(px(120.))
        .opacity(0.9)
        .child(
            div()
                .text_3xl()
                .font_weight(FontWeight::EXTRA_BOLD)
                .text_color(with_alpha(Neon::cyan_light(), 0.85))
                .child("CYBERLEXI"),
        )
        .child(
            div()
                .text_sm()
                .text_color(Neon::muted())
                .child("输入以查询 · Ctrl K 聚焦搜索 · Enter 立即执行"),
        )
        .child(
            div()
                .text_xs()
                .font_family(crate::theme::MONO_FONT)
                .text_color(Neon::faint())
                .child("voluminous · ephemeral · 你好 · nihao"),
        )
}

// ============================================================
// 工具
// ============================================================

/// 数字 → 上标字符（homograph 序号显示）。
fn superscript(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            other => other,
        })
        .collect()
}

/// 常用度 1~99（BNC/COCA 排名 → 百分位；排名越小越常用）。
fn frequency_score(entry: &MergedEntry) -> Option<u32> {
    let ranks: [i32; 2] = [
        entry.freq.map(|f| f.0).unwrap_or(0),
        entry.freq.map(|f| f.1).unwrap_or(0),
    ];
    let best = ranks.iter().copied().filter(|r| *r > 0).min()?;
    // 60k 词表内：pct = 99 - rank/60000*98
    let pct = 99.0 - (best as f32 / 60_000.0) * 98.0;
    Some(pct.clamp(1.0, 99.0) as u32)
}

/// unix 秒 → 'YYYY.MM.DD'（UTC，档案展示用）。
pub fn date_str(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}.{m:02}.{d:02}")
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
