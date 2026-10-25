//! 顶栏：搜索框（焦点青光圈 + Ctrl K 提示）、MW 配额/离线状态、
//! 随机单词与历史弹层按钮、导入进度细条。设计稿 .topbar。

use gpui_kit::assets::IconName;
use gpui_kit::component::input::Input;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::*;

use crate::app::AppView;
use crate::theme::{grad, hex, steel, white, with_alpha, Neon};
use crate::widgets::{icon_button, kbd};

pub fn render(view: &mut AppView, window: &mut Window, cx: &mut Context<AppView>) -> Div {
    let focused = view
        .input
        .read(cx)
        .focus_handle(cx)
        .is_focused(window);

    let mut bar = div()
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .gap_3()
        .h(px(68.))
        .px(px(26.))
        .bg(with_alpha(Neon::bg(), 0.55))
        .border_b_1()
        .border_color(with_alpha(Neon::cyan(), 0.09))
        .child(search_box(view, focused, cx))
        // 返回上一词（导航栈空时置灰）
        .child(back_button(view, cx));

    // 窗口拖动区（自定义标题栏）：搜索框与右侧控件间的空白条。
    // HTCAPTION 命中：系统级拖动 + 双击最大化 + 右键系统菜单
    bar = bar.child(
        div()
            .flex_1()
            .h_full()
            .window_control_area(WindowControlArea::Drag),
    );

    // MW 配额 chip（有 Key 时）
    if let Some((used, limit)) = view.quota {
        if limit > 0 {
            bar = bar.child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(Neon::border())
                    .text_xs()
                    .font_family(crate::theme::MONO_FONT)
                    .text_color(Neon::muted())
                    .child(format!("MW {used}/{limit}")),
            );
        }
    }

    // 离线模式指示
    if view.offline {
        bar = bar.child(
            div()
                .flex()
                .items_center()
                .gap(px(5.))
                .px_2()
                .py_1()
                .rounded_sm()
                .border_1()
                .border_color(with_alpha(Neon::amber(), 0.5))
                .text_xs()
                .text_color(Neon::amber())
                .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(Neon::amber()))
                .child(div().child("OFFLINE")),
        );
    }

    // 右侧动作
    bar = bar
        .child(
            icon_button("btn-random", IconName::Shuffle, "随机单词", |_, _, _| {
                crate::send_global(lexi_core::events::UiCommand::RandomWord);
            }),
        )
        .child(
            icon_button("btn-history", IconName::Clock, "历史记录", {
                cx.listener(|v, _, _, cx| {
                    v.history_open = !v.history_open;
                    cx.notify();
                })
            }),
        )
        // 自绘窗口控制（原生标题栏按钮已隐藏）
        .child(window_controls(view, window, cx));

    // 首启导入进度：底缘 2px 渐变细条（不遮挡布局）
    if let Some((pct, _stage)) = view.import_progress.clone() {
        bar = bar.child(
            div()
                .absolute()
                .bottom_0()
                .left_0()
                .h(px(2.))
                .w(relative(pct / 100.0))
                .bg(grad(90., Neon::cyan(), Neon::violet()))
                .shadow(vec![crate::theme::glow(Neon::cyan(), 0.6, 8.)]),
        );
    }
    // 历史弹层由 app.rs 在根层渲染（避免被后绘制的内容视图遮挡）
    bar
}

/// 返回上一词按钮（搜索框右侧）：栈空置灰不可点；有历史时高亮。
fn back_button(view: &AppView, cx: &mut Context<AppView>) -> Stateful<Div> {
    let enabled = !view.nav_history.is_empty();
    let accent = view.accent.color();
    let mut btn = div()
        .id("btn-back")
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .w(px(34.))
        .h(px(34.))
        .rounded_full()
        .border_1()
        .text_color(if enabled {
            accent
        } else {
            Neon::faint()
        })
        .border_color(if enabled {
            with_alpha(accent, 0.35)
        } else {
            steel(0.12)
        });
    if enabled {
        btn = btn
            .cursor_pointer()
            .bg(with_alpha(accent, 0.06))
            .hover(|d| {
                d.bg(with_alpha(accent, 0.16))
                    .shadow(vec![crate::theme::glow(accent, 0.4, 14.)])
            })
            .on_click(cx.listener(|v, _, window, cx| {
                v.go_back(window, cx);
            }))
            .child(Icon::new(IconName::ArrowLeft).with_size(px(15.)));
    } else {
        btn = btn.child(Icon::new(IconName::ArrowLeft).with_size(px(15.)));
    }
    btn
}

/// 自绘窗口控制按钮组（最小化/关闭）：原生标题栏已通过
/// appears_transparent 隐藏，按钮样式与 icon_button 一致（关闭 hover
/// 为品红警示色）。最大化按钮暂缺：gpui-pre Windows 的 zoom() 只
/// 最大化不还原（上游 bug）；需要最大化时双击顶栏拖动区（系统
/// HTCAPTION 行为，再双击或下拖还原）。
fn window_controls(_view: &AppView, _window: &Window, cx: &Context<AppView>) -> Div {
    let mut group = div().flex().items_center().gap(px(2.)).ml(px(6.));

    group = group.child(
        div()
            .id("win-min")
            .cursor_pointer()
            .w(px(30.))
            .h(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(white(0.03))
            .border_1()
            .border_color(steel(0.14))
            .text_color(hex("#7e97ad"))
            .hover(|d| {
                d.text_color(Neon::cyan())
                    .border_color(with_alpha(Neon::cyan(), 0.45))
                    .shadow(vec![crate::theme::glow(Neon::cyan(), 0.18, 18.)])
            })
            .on_click(cx.listener(|_, _, window, _| window.minimize_window()))
            .child(Icon::new(IconName::Minus).with_size(px(15.))),
    );

    group = group.child(
        div()
            .id("win-close")
            .cursor_pointer()
            .w(px(30.))
            .h(px(30.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(white(0.03))
            .border_1()
            .border_color(steel(0.14))
            .text_color(hex("#7e97ad"))
            .hover(|d| {
                d.text_color(hex("#ffb0c8"))
                    .border_color(with_alpha(Neon::magenta(), 0.55))
                    .shadow(vec![crate::theme::glow(Neon::magenta(), 0.25, 18.)])
            })
            .on_click(cx.listener(|_, _, window, _| window.remove_window()))
            .child(Icon::new(IconName::X).with_size(px(15.))),
    );

    group
}

fn search_box(view: &mut AppView, focused: bool, _cx: &mut Context<AppView>) -> Div {
    let accent = view.accent.color();
    let input_entity = view.input.clone();
    let mut box_div = div()
        .flex()
        .flex_1()
        .max_w(px(640.))
        .items_center()
        .gap(px(10.))
        .h(px(42.))
        .px(px(14.))
        .rounded_md()
        .bg(Neon::bg_input())
        .border_1()
        .border_color(if focused {
            with_alpha(accent, 0.6)
        } else {
            with_alpha(Neon::cyan(), 0.16)
        });
    if focused {
        box_div = box_div.shadow(vec![
            crate::theme::glow(accent, 0.18, 26.),
        ]);
    }
    box_div
        .child(
            Icon::new(IconName::Search)
                .with_size(px(17.))
                .text_color(hex("#4e8aa8")),
        )
        .child(div().flex_1().flex().items_center().child(
            Input::new(&input_entity).appearance(false),
        ))
        // 焦点提示键位 / 查询中状态
        .child(if view.loading {
            div()
                .text_xs()
                .font_family(crate::theme::MONO_FONT)
                .text_color(accent)
                .child("QUERYING…")
        } else {
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .child(kbd("Ctrl"))
                .child(kbd("K"))
        })
}

pub fn history_popover(view: &mut AppView, cx: &mut Context<AppView>) -> Stateful<Div> {
    let mut pop = div()
        .absolute()
        .top(px(76.))
        .right(px(26.))
        .w(px(320.))
        .id("history-popover")
        .max_h(px(420.))
        .overflow_y_scroll()
        .p(px(10.))
        .rounded_lg()
        .bg(Neon::bg_surf())
        .border_1()
        .border_color(with_alpha(Neon::cyan(), 0.25))
        .shadow(vec![crate::theme::glow(Neon::cyan(), 0.15, 30.)])
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .px(px(6.))
                .pb(px(8.))
                .text_xs()
                .text_color(Neon::muted())
                .child(div().child(format!("最近查询 · {}", view.history.len())))
                .child(
                    div()
                        .id("history-close")
                        .cursor_pointer()
                        .text_color(Neon::muted())
                        .hover(|d| d.text_color(Neon::magenta()))
                        .on_click(cx.listener(|v, _, _, cx| {
                            v.history_open = false;
                            cx.notify();
                        }))
                        .child("✕"),
                ),
        );
    if view.history.is_empty() {
        pop = pop.child(
            div()
                .px(px(6.))
                .py_2()
                .text_xs()
                .text_color(Neon::faint())
                .child("暂无查询记录"),
        );
    }
    for (i, h) in view.history.iter().take(50).enumerate() {
        let text = h.clone();
        pop = pop.child(
            div()
                .id(("hist-pop", i))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap_2()
                .px(px(8.))
                .py(px(6.))
                .rounded_md()
                .hover(|d| d.bg(with_alpha(Neon::cyan(), 0.07)))
                .on_click(cx.listener(move |v, _, _, cx| {
                    v.history_open = false;
                    v.dispatch_query(text.clone(), cx);
                }))
                .child(
                    Icon::new(IconName::Clock)
                        .with_size(px(13.))
                        .text_color(Neon::faint()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(hex("#cfe6f5"))
                        .child(h.clone()),
                ),
        );
    }
    pop
}
