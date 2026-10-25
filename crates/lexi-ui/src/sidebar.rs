//! 左侧边栏：品牌区、三视图导航（词典/单词本/设置）、底部统计
//!（今日查词目标 + 连续打卡）。设计稿 .sidebar。
//!
//! GPUI 限制：无 letter-spacing / transform:rotate —— 品牌字距与菱形
//! Logo 以字重/渐变近似（待确认：GPUI 后续版本补充）。

use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::*;

use crate::app::{AppView, Page};
use crate::theme::{grad, hex, white, with_alpha, Neon};
use crate::widgets::progress_bar;

/// 渲染侧边栏（232px）。右缘 1px 青紫渐隐描边由半透明青描边近似。
pub fn render(view: &AppView, cx: &mut Context<AppView>) -> Div {
    let accent = view.accent.color();
    div()
        .w(px(232.))
        .flex_none()
        .flex()
        .flex_col()
        .px(px(16.))
        .pt(px(20.))
        .pb(px(18.))
        .bg(Neon::bg_surf())
        .border_r_1()
        .border_color(with_alpha(Neon::cyan(), 0.10))
        .child(brand())
        .child(nav(view, accent, cx))
        .child(div().flex_1())
        .child(side_foot(view))
}

fn brand() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(11.))
        .px(px(6.))
        .pb(px(20.))
        .border_b_1()
        .border_color(white(0.06))
        // Logo：青→紫渐变方块 + 发光（菱形 clip-path 的近似）
        .child(
            div()
                .w(px(30.))
                .h(px(30.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(9.))
                .bg(grad(135., Neon::cyan(), Neon::violet()))
                .shadow(vec![crate::theme::glow(Neon::cyan(), 0.5, 22.)])
                .child(
                    div()
                        .text_base()
                        .font_weight(FontWeight::BLACK)
                        .text_color(hex("#03121a"))
                        .child("N"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .text_color(hex("#ffffff"))
                        .child("NEONLEX"),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(crate::theme::MONO_FONT)
                        .text_color(Neon::faint())
                        .child("v0.1.0 / MW-API"),
                ),
        )
}

fn nav(view: &AppView, accent: Hsla, cx: &mut Context<AppView>) -> Div {
    let mut col = div().flex().flex_col().gap(px(6.)).mt(px(22.));
    let items: [(Page, &str, IconName); 3] = [
        (Page::Dict, "词典", IconName::BookOpen),
        (Page::Book, "单词本", IconName::Bookmark),
        (Page::Settings, "设置", IconName::Settings),
    ];
    for (i, (page, label, icon)) in items.into_iter().enumerate() {
        let active = view.page == page;
        let fg = if active { accent } else { Neon::muted() };
        let mut item = div()
            .id(("nav", i))
            .cursor_pointer()
            .relative()
            .flex()
            .items_center()
            .gap(px(12.))
            .px(px(14.))
            .py(px(11.))
            .rounded_md()
            .text_sm()
            .text_color(fg);
        item = if active {
            item.bg(grad(
                90.,
                with_alpha(accent, 0.15),
                with_alpha(accent, 0.015),
            ))
            .border_1()
            .border_color(with_alpha(accent, 0.26))
            .shadow(vec![crate::theme::glow(accent, 0.10, 22.)])
            // 左缘指示条（.nav-item.active::before）
            .child(
                div()
                    .absolute()
                    .left(px(-16.))
                    .top_1_2()
                    .w(px(3.))
                    .h(px(20.))
                    .rounded_sm()
                    .bg(accent)
                    .shadow(vec![crate::theme::glow(accent, 0.8, 14.)]),
            )
        } else {
            item.border_1()
                .border_color(Hsla::transparent_black())
                .hover(|d| {
                    d.bg(with_alpha(Neon::cyan(), 0.05))
                        .text_color(hex("#cfe6f5"))
                })
        };
        // 切换视图（词典页保持既有查询结果）
        let target = page;
        item = item
            .on_click(cx.listener(move |v, _, _, cx| {
                v.page = target;
                cx.notify();
            }))
            .child(Icon::new(icon).with_size(px(18.)).text_color(fg))
            .child(div().child(label.to_string()));
        col = col.child(item);
    }
    col
}

fn side_foot(view: &AppView) -> Div {
    let (today, goal, streak) = view.stats;
    let today_pct = if goal > 0 { today as f32 / goal as f32 } else { 0.0 };
    let streak_pct = (streak as f32 / 30.0).min(1.0);
    div()
        .pt(px(14.))
        .px(px(6.))
        .border_t_1()
        .border_color(white(0.06))
        .flex()
        .flex_col()
        // 今日学习目标
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .text_xs()
                .text_color(hex("#6d8099"))
                .child(div().child("今日学习目标"))
                .child(
                    div()
                        .font_family(crate::theme::MONO_FONT)
                        .text_color(Neon::cyan_light())
                        .child(format!("{today} / {goal}")),
                ),
        )
        .child(div().mt(px(7.)).mb(px(14.)).child(progress_bar(today_pct, relative(1.0))))
        // 连续打卡
        .child(
            div()
                .flex()
                .justify_between()
                .items_center()
                .text_xs()
                .text_color(hex("#6d8099"))
                .child(div().child("连续打卡"))
                .child(
                    div()
                        .font_family(crate::theme::MONO_FONT)
                        .text_color(Neon::cyan_light())
                        .child(format!("{streak} DAYS")),
                ),
        )
        .child(div().mt(px(7.)).child(progress_bar(streak_pct, relative(1.0))))
}
