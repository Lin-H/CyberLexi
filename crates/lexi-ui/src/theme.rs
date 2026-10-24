//! Cyberpunk 主题 tokens（spec 第九节）与 gpui-kit Theme 接入。
//!
//! 手法：深色底 + 1px 描边卡片；焦点/选中 cyan 描边 + 同色半透明
//! 阴影模拟 glow；霓虹色仅限「搜索框焦点、当前选中、按钮 hover」三处。

use gpui_kit::*;
use gpui_kit::component::Theme;

/// 主题色 token（Hsla）。命名与 spec 一一对应。
pub struct CyberpunkTheme;

impl CyberpunkTheme {
    pub fn bg_base() -> Hsla { hex("#0a0a0f") }
    pub fn bg_surf() -> Hsla { hex("#12121a") }
    pub fn bg_elev() -> Hsla { hex("#1a1a26") }
    pub fn border() -> Hsla { hex("#2a2a3a") }
    /// 主强调：搜索框焦点 / 当前选中 / 按钮 hover（唯一主发光色）
    pub fn cyan() -> Hsla { hex("#00f0ff") }
    pub fn magenta() -> Hsla { hex("#ff003c") }
    pub fn yellow() -> Hsla { hex("#fcee0a") }
    pub fn green() -> Hsla { hex("#00ff9f") }
    pub fn text() -> Hsla { hex("#e0e0e8") }
    pub fn dim() -> Hsla { hex("#6b6b80") }
}

/// #rrggbb → Hsla（不透明）。
pub fn hex(s: &str) -> Hsla {
    let h = s.trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).expect("valid hex color");
    rgba(
        (v >> 16) as u8,
        ((v >> 8) & 0xff) as u8,
        (v & 0xff) as u8,
        1.0,
    )
}

/// rgba → Hsla。
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let (h, s) = if (max - min).abs() < f32::EPSILON {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
        let h = if (max - r).abs() < f32::EPSILON {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if (max - g).abs() < f32::EPSILON {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };
        (h * 60.0, s)
    };
    Hsla { h: h / 360.0, s, l, a }
}

/// 半透明变体（glow 阴影用）。
pub fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

/// 通用卡片：surf 底 + 1px 描边（供后续组件复用）。
#[allow(dead_code)]
pub fn card() -> Div {
    div()
        .bg(CyberpunkTheme::bg_surf())
        .border_1()
        .border_color(CyberpunkTheme::border())
        .rounded_lg()
}

/// 应用到 gpui-kit 全局主题（字体与基础暗色）。
/// 注意：颜色在 Theme.colors（ThemeColor），字体在 Theme 根字段。
pub fn apply(cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.font_family = "JetBrains Mono".into();
        theme.colors.background = CyberpunkTheme::bg_base();
        theme.colors.foreground = CyberpunkTheme::text();
        theme.colors.border = CyberpunkTheme::border();
        theme.colors.accent = CyberpunkTheme::cyan();
        theme.colors.secondary = CyberpunkTheme::bg_elev();
    });
}
