//! 通用装饰部件：面板（角标描边）、分隔节标题、chip/tag、kbd、
//! 进度条、图标按钮、循环选择器。样式逐条对应设计稿。

use gpui_kit::component::Icon;
use gpui_kit::assets::IconName;
use gpui_kit::*;

use crate::theme::{glow, grad, hex, steel, white, with_alpha, Neon};
use gpui_kit::component::Sizable;

/// 面板卡片：暗面渐变底 + 1px 描边 + 左上青/右下紫角标（设计稿 .panel）。
pub fn panel() -> Div {
    div()
        .relative()
        .p_4()
        .rounded_lg()
        .bg(Neon::bg_surf())
        .border_1()
        .border_color(Neon::border())
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .w(px(16.))
                .h(px(16.))
                .border_t_1()
                .border_l_1()
                .border_color(with_alpha(Neon::cyan(), 0.8))
                .rounded_tl_lg(),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .right_0()
                .w(px(16.))
                .h(px(16.))
                .border_b_1()
                .border_r_1()
                .border_color(with_alpha(Neon::violet(), 0.8))
                .rounded_br_lg(),
        )
}

/// 面板标题（大写字距标题 + 渐隐横线；设计稿 .panel-title）。
pub fn panel_title(text: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .mb_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(Neon::cyan_light())
                .child(text.to_uppercase()),
        )
        .child(div().flex_1().h(px(1.)).bg(grad(
            90.,
            with_alpha(Neon::cyan(), 0.4),
            Hsla::transparent_black(),
        )))
}

/// 主区分隔节标题（设计稿 .section-label）。
pub fn section_label(text: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .mb_4()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(Neon::cyan_light())
                .child(text.to_uppercase()),
        )
        .child(div().flex_1().h(px(1.)).bg(grad(
            90.,
            with_alpha(Neon::cyan(), 0.4),
            Hsla::transparent_black(),
        )))
}

/// 药丸标签（词性/等级/来源；设计稿 .tag / .pos-badge / .filter）。
pub fn tag_pill(label: impl Into<SharedString>, fg: Hsla, border: Hsla, bg: Hsla) -> Div {
    div()
        .px_2()
        .py(px(3.))
        .rounded_sm()
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_color(fg)
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.into())
}

/// 实心强调药丸（词性主标签：青底深字；设计稿 .tag.pos）。
pub fn solid_pill(label: impl Into<SharedString>, color: Hsla) -> Div {
    div()
        .px(px(11.))
        .py_1()
        .rounded_full()
        .bg(color)
        .text_color(Neon::bg())
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.into())
}

/// 等宽小 chip（stems/ins；设计稿 .stem-chip / .ins-item）。
pub fn mono_chip(label: impl Into<SharedString>) -> Div {
    div()
        .px(px(9.))
        .py(px(2.))
        .rounded_sm()
        .bg(white(0.03))
        .border_1()
        .border_color(Neon::border())
        .text_color(Neon::muted())
        .text_xs()
        .font_family(crate::theme::MONO_FONT)
        .child(label.into())
}

/// 键位提示（设计稿 kbd）。
pub fn kbd(text: &str) -> Div {
    div()
        .px(px(6.))
        .py(px(2.))
        .rounded_sm()
        .bg(white(0.03))
        .border_1()
        .border_color(steel(0.2))
        .text_color(hex("#5e7f99"))
        .text_xs()
        .font_family(crate::theme::MONO_FONT)
        .child(text.to_string())
}

/// 细进度条（设计稿 .bar / .mini-progress）。
pub fn progress_bar(pct: f32, w: DefiniteLength) -> Div {
    let pct = pct.clamp(0.0, 1.0);
    div()
        .w(w)
        .h(px(4.))
        .rounded_full()
        .bg(white(0.07))
        .overflow_hidden()
        .child(
            div()
                .h(px(4.))
                .w(relative(pct))
                .rounded_full()
                .bg(grad(90., Neon::cyan(), Neon::violet())),
        )
}

/// 顶栏图标按钮（36px；设计稿 .icon-btn）。
pub fn icon_button(
    id: impl Into<ElementId>,
    icon: IconName,
    _title: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .cursor_pointer()
        .w_9()
        .h_9()
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
                .shadow(vec![glow(Neon::cyan(), 0.18, 18.)])
        })
        .on_click(on_click)
        .child(Icon::new(icon).with_size(px(17.)))
}

/// 圆形动作按钮（发音/收藏；设计稿 .play-btn / .star-btn）。
pub fn round_action(
    id: impl Into<ElementId>,
    icon: IconName,
    color: Hsla,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .cursor_pointer()
        .w(px(40.))
        .h(px(40.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(with_alpha(color, 0.07))
        .border_1()
        .border_color(with_alpha(color, 0.4))
        .text_color(color)
        .hover(|d| {
            d.bg(with_alpha(color, 0.18))
                .shadow(vec![glow(color, 0.5, 24.)])
        })
        .on_click(on_click)
        .child(Icon::new(icon).with_size(px(16.)))
}

/// 循环选择器（点击切到下一项；select 的等价交互，避免自制下拉弹层的
/// 复杂度）。显示当前值 + chevron。
pub fn cycle_select(
    id: impl Into<ElementId>,
    current: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(6.))
        .px(px(12.))
        .py(px(7.))
        .rounded_md()
        .bg(Neon::bg_input())
        .border_1()
        .border_color(steel(0.2))
        .hover(|d| {
            d.border_color(with_alpha(Neon::cyan(), 0.55))
                .shadow(vec![glow(Neon::cyan(), 0.2, 16.)])
        })
        .on_click(on_click)
        .child(
            div()
                .text_xs()
                .text_color(hex("#cfe6f5"))
                .child(current.to_string()),
        )
        .child(
            Icon::new(IconName::ChevronDown)
                .with_size(px(13.))
                .text_color(Neon::faint()),
        )
}
