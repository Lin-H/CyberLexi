//! 单词本视图：页头（计数 + 添加单词）、状态筛选行、卡片网格。
//! 设计稿 #view-book（四态筛选映射 全部/生词/学习中/熟悉/已掌握）。

use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;

use lexi_core::domain::types::WordSummary;
use lexi_core::domain::wordbook::{WordBookItem, WordStatus};
use lexi_core::events::UiCommand;

use crate::app::AppView;
use crate::dict_view::date_str;
use crate::theme::{sense_accent, grad, hex, steel, white, with_alpha, Neon};
use crate::widgets::progress_bar;

/// 状态筛选键。
#[derive(Clone, Copy, PartialEq)]
pub enum BookFilter {
    All,
    New,
    Learning,
    Familiar,
    Mastered,
}

impl BookFilter {
    fn label(&self) -> &'static str {
        match self {
            BookFilter::All => "全部",
            BookFilter::New => "生词",
            BookFilter::Learning => "学习中",
            BookFilter::Familiar => "熟悉",
            BookFilter::Mastered => "已掌握",
        }
    }

    fn matches(&self, s: WordStatus) -> bool {
        match self {
            BookFilter::All => true,
            BookFilter::New => s == WordStatus::New,
            BookFilter::Learning => s == WordStatus::Learning,
            BookFilter::Familiar => s == WordStatus::Familiar,
            BookFilter::Mastered => s == WordStatus::Mastered,
        }
    }
}

const FILTERS: [BookFilter; 5] = [
    BookFilter::All,
    BookFilter::New,
    BookFilter::Learning,
    BookFilter::Familiar,
    BookFilter::Mastered,
];

pub fn render(view: &mut AppView, cx: &mut Context<AppView>) -> Div {
    // 语义：summary（音标/释义）按 word_key 索引
    let items = view.wordbook.clone();
    let summaries: std::collections::HashMap<String, WordSummary> = view
        .wb_summaries
        .iter()
        .map(|s| (s.word_key.clone(), s.clone()))
        .collect();

    let total = items.len();
    let count_of = |f: BookFilter| items.iter().filter(|i| f.matches(i.status)).count();
    let filtered: Vec<&WordBookItem> = items
        .iter()
        .filter(|i| view.book_filter.matches(i.status))
        .collect();

    let mut col = div().flex().flex_col().max_w(px(1240.));

    // 页头
    col = col.child(
        div()
            .flex()
            .items_end()
            .justify_between()
            .gap_5()
            .mb_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::EXTRA_BOLD)
                            .text_color(hex("#ffffff"))
                            .child("单词本"),
                    )
                    .child(
                        div()
                            .mt(px(7.))
                            .text_sm()
                            .text_color(hex("#6d8299"))
                            .child(format!(
                                "已收录 {total} 个单词 · 学习中 {} · 已掌握 {}",
                                count_of(BookFilter::Learning),
                                count_of(BookFilter::Mastered),
                            )),
                    ),
            )
            .child(
                div()
                    .id("wb-add-toggle")
                    .cursor_pointer()
                    .px(px(18.))
                    .py(px(9.))
                    .rounded_md()
                    .bg(grad(90., Neon::cyan(), hex("#7de9ff")))
                    .shadow(vec![crate::theme::glow(Neon::cyan(), 0.45, 24.)])
                    .hover(|d| d.shadow(vec![crate::theme::glow(Neon::cyan(), 0.75, 34.)]))
                    .on_click(cx.listener(|v, _, _, cx| {
                        v.add_word_open = !v.add_word_open;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(hex("#03121a"))
                            .child("+ 添加单词"),
                    ),
            ),
    );

    // 添加单词行（展开时）
    if view.add_word_open {
        let add_input = view.add_word_input.clone();
        col = col.child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mb_5()
                .px(px(14.))
                .py(px(10.))
                .rounded_lg()
                .bg(Neon::bg_input())
                .border_1()
                .border_color(with_alpha(Neon::cyan(), 0.25))
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(&add_input).appearance(false)),
                )
                .child(
                    div()
                        .id("wb-add-confirm")
                        .cursor_pointer()
                        .px(px(14.))
                        .py(px(6.))
                        .rounded_md()
                        .bg(with_alpha(Neon::cyan(), 0.12))
                        .border_1()
                        .border_color(with_alpha(Neon::cyan(), 0.5))
                        .text_sm()
                        .text_color(Neon::cyan())
                        .on_click({
                            let add_input = add_input.clone();
                            move |_, _, cx| {
                                let text = add_input.read(cx).value().trim().to_string();
                                if !text.is_empty() {
                                    crate::send_global(UiCommand::Query(text.clone()));
                                    // 查询结果回来后用户在词条页收藏；
                                    // 直接顺手入本（key 由 normalizer 决定，headword 用原词）
                                    crate::send_global(UiCommand::AddToWordBook {
                                        key: text.to_lowercase(),
                                        headword: text,
                                    });
                                }
                            }
                        })
                        .child("查询并加入"),
                ),
        );
    }

    // 筛选行
    let mut filter_row = div().flex().flex_wrap().gap(px(9.)).mb_5();
    for (fi, f) in FILTERS.into_iter().enumerate() {
        let active = view.book_filter == f;
        let n = count_of(f);
        let mut chip = div()
            .id(("wb-filter", fi))
            .cursor_pointer()
            .px(px(15.))
            .py(px(6.))
            .rounded_full()
            .text_sm()
            .text_color(if active { Neon::cyan() } else { hex("#8ba3ba") })
            .bg(if active {
                with_alpha(Neon::cyan(), 0.1)
            } else {
                white(0.03)
            })
            .border_1()
            .border_color(if active {
                with_alpha(Neon::cyan(), 0.55)
            } else {
                steel(0.14)
            });
        if active {
            chip = chip.shadow(vec![crate::theme::glow(Neon::cyan(), 0.22, 18.)]);
        }
        chip = chip.on_click(cx.listener(move |v, _, _, cx| {
            v.book_filter = f;
            cx.notify();
        }));
        filter_row = filter_row.child(chip.child(format!("{} {n}", f.label())));
    }
    col = col.child(filter_row);

    // 卡片网格
    if filtered.is_empty() {
        col = col.child(
            div()
                .py(px(60.))
                .flex()
                .justify_center()
                .text_sm()
                .text_color(Neon::faint())
                .child(if total == 0 {
                    "单词本为空 · 在词条页点击 ☆ 收藏，或点右上「+ 添加单词」".to_string()
                } else {
                    "该分类下暂无单词".to_string()
                }),
        );
    } else {
        let mut grid = div().flex().flex_wrap().gap(px(14.));
        for (i, item) in filtered.iter().enumerate() {
            let summary = summaries.get(&item.word_key);
            grid = grid.child(word_card(i, item, summary, cx));
        }
        col = col.child(grid);
    }
    col
}

fn word_card(
    i: usize,
    item: &WordBookItem,
    summary: Option<&WordSummary>,
    cx: &mut Context<AppView>,
) -> Div {
    let accent = sense_accent(i);
    let (pct, status_label) = match item.status {
        WordStatus::New => (0.10, "生词"),
        WordStatus::Learning => (0.38, "学习中"),
        WordStatus::Familiar => (0.70, "熟悉"),
        WordStatus::Mastered => (1.00, "已掌握"),
    };
    let phonetic = summary
        .map(|s| s.phonetic.clone())
        .unwrap_or_default();
    let pos = summary.map(|s| s.pos.clone()).unwrap_or_default();
    let definition = summary
        .map(|s| s.definition.clone())
        .unwrap_or_default();
    let tag = summary
        .and_then(|s| s.exam_tags.first().map(|t| t.to_uppercase()))
        .unwrap_or_else(|| date_str(item.added_at));

    let word = item.headword.clone();
    let key = item.word_key.clone();
    let next_status = item.status.next();

    let mut subline = String::new();
    if !phonetic.is_empty() {
        subline.push_str(&format!("/{phonetic}"));
    }
    if !pos.is_empty() {
        if !subline.is_empty() {
            subline.push_str(" · ");
        }
        subline.push_str(&pos);
    }

    div()
        .relative()
        .w(px(268.))
        .p(px(17.))
        .rounded_xl()
        .bg(grad(
            155.,
            with_alpha(hex("#0e1726"), 0.8),
            with_alpha(hex("#070b12"), 0.75),
        ))
        .border_1()
        .border_color(steel(0.11))
        .hover(|d| {
            d.border_color(with_alpha(Neon::cyan(), 0.35))
                .shadow(vec![crate::theme::glow(Neon::cyan(), 0.25, 30.)])
        })
        // 左缘强调条
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(2.))
                .bg(accent)
                .shadow(vec![crate::theme::glow(accent, 0.6, 14.)]),
        )
        .flex()
        .flex_col()
        // 词形（点击跳词条）
        .child(
            div()
                .id(("wb-card-word", i))
                .cursor_pointer()
                .self_start()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .text_color(Neon::text_bright())
                .hover(move |d| d.text_color(Neon::cyan_light()))
                .on_click({
                    let word = word.clone();
                    cx.listener(move |v, _, _, cx| {
                        v.page = crate::app::Page::Dict;
                        v.dispatch_query(word.clone(), cx);
                    })
                })
                .child(item.headword.clone()),
        )
        .child(if subline.is_empty() {
            div().h(px(16.))
        } else {
            div()
                .mt(px(3.))
                .text_xs()
                .font_family(crate::theme::MONO_FONT)
                .text_color(hex("#5f7f99"))
                .child(subline)
        })
        .child(if definition.is_empty() {
            div().mt(px(9.)).h(px(18.))
        } else {
            div()
                .mt(px(9.))
                .text_sm()
                .line_height(relative(1.5))
                .text_color(hex("#8ba3ba"))
                .child(truncate_chars(&definition, 64))
        })
        .child(
            div()
                .flex()
                .items_center()
                .mt(px(12.))
                .gap(px(10.))
                .child(progress_bar(pct, relative(1.0)))
                .child(
                    div()
                        .text_xs()
                        .font_family(crate::theme::MONO_FONT)
                        .text_color(hex("#5f7f99"))
                        .child(tag),
                ),
        )
        // 底部操作：状态流转 + 移除（小尺寸，不抢卡片主体）
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .mt(px(10.))
                .pt(px(8.))
                .border_t_1()
                .border_color(white(0.05))
                .child(
                    div()
                        .id(("wb-card-status", i))
                        .cursor_pointer()
                        .text_xs()
                        .px(px(8.))
                        .py(px(2.))
                        .rounded_sm()
                        .border_1()
                        .border_color(accent)
                        .text_color(accent)
                        .hover(move |d| d.bg(with_alpha(accent, 0.12)))
                        .on_click(move |_, _, _| {
                            crate::send_global(UiCommand::SetWordStatus {
                                key: key.clone(),
                                status: next_status,
                            });
                        })
                        .child(format!("{status_label} ›")),
                )
                .child(
                    div()
                        .id(("wb-card-del", i))
                        .cursor_pointer()
                        .text_xs()
                        .text_color(Neon::faint())
                        .hover(|d| d.text_color(Neon::magenta()))
                        .on_click({
                            let key = item.word_key.clone();
                            move |_, _, _| {
                                crate::send_global(UiCommand::RemoveFromWordBook(key.clone()));
                            }
                        })
                        .child("移除"),
                ),
        )
}

/// 按字符截断（中文按字符数更均匀）。
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        s.chars().take(max).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}

/// 订阅添加单词输入的 Enter（查询并加入）。
pub fn subscribe_add_input(
    add_input: &gpui_kit::Entity<InputState>,
    window: &mut Window,
    cx: &mut Context<AppView>,
) -> Subscription {
    cx.subscribe_in(add_input, window, |v, state, event: &InputEvent, _window, cx| {
        if let InputEvent::PressEnter { .. } = event {
            let text = state.read(cx).value().trim().to_string();
            if !text.is_empty() {
                v.page = crate::app::Page::Dict;
                v.dispatch_query(text.clone(), cx);
                crate::send_global(UiCommand::AddToWordBook {
                    key: text.to_lowercase(),
                    headword: text,
                });
                v.add_word_open = false;
                cx.notify();
            }
        }
    })
}
