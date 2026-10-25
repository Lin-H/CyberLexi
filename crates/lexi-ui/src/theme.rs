//! NEONLEX 主题 tokens（docs/cyberpunk ui.html 设计稿 :root 变量）。
//!
//! 色板：深空底 + 霓虹青主强调 + 霓虹粉/等离紫/荧光绿/琥珀辅助。
//! 「主题色板」设置可在 cyan/magenta/violet 三者间切换强调色
//! （AccentPalette），其余色固定。

use gpui_kit::component::Theme;
use gpui_kit::*;

/// UI 正文等宽字体（设计稿 --mono 的可用子集；CJK 走 GPUI fallback）。
pub const MONO_FONT: &str = "JetBrains Mono";
/// UI 界面字体（设计稿 "Segoe UI Variable"；Windows 系统字体解析）。
pub const UI_FONT: &str = "Segoe UI";

/// 主题色 token（Hsla）。命名与设计稿一一对应。
pub struct Neon;

impl Neon {
    /// --bg
    pub fn bg() -> Hsla { hex("#05070d") }
    /// 侧栏/卡片面（设计稿渐变底的两端取近似均值）
    pub fn bg_surf() -> Hsla { hex("#0a1018") }
    pub fn bg_elev() -> Hsla { hex("#101a29") }
    /// 输入框底（rgba(8,14,24,.85)）
    pub fn bg_input() -> Hsla { hex("#080e18") }
    pub fn border() -> Hsla { hex("#1e2c40") }
    /// --cyan #00eaff
    pub fn cyan() -> Hsla { hex("#00eaff") }
    /// --magenta #ff2d78
    pub fn magenta() -> Hsla { hex("#ff2d78") }
    /// --violet #a855f7
    pub fn violet() -> Hsla { hex("#a855f7") }
    /// --lime #7cffb2
    pub fn lime() -> Hsla { hex("#7cffb2") }
    /// --amber #ffb020
    pub fn amber() -> Hsla { hex("#ffb020") }
    /// --text #dbe7f3
    pub fn text() -> Hsla { hex("#dbe7f3") }
    /// 亮文本（词条定义 #eaf5ff）
    pub fn text_bright() -> Hsla { hex("#eaf5ff") }
    /// --muted #7c8ba1
    pub fn muted() -> Hsla { hex("#7c8ba1") }
    /// 次级标签（#4e6a80 一档）
    pub fn faint() -> Hsla { hex("#4e6a80") }
    /// headword 渐变末端/高亮青白（#a9ecff）
    pub fn cyan_light() -> Hsla { hex("#a9ecff") }
    /// 音标文本（#8fd4ee）
    pub fn phonetic() -> Hsla { hex("#8fd4ee") }
    /// 例句文本（#a9c1d6）
    pub fn example() -> Hsla { hex("#a9c1d6") }
    /// sense 编号小标（#54708a）
    pub fn sense_hint() -> Hsla { hex("#54708a") }
    /// meta-strip 文本（#7fa5c4 加权值）
    pub fn meta_key() -> Hsla { hex("#7fa5c4") }
}

/// 可切换强调色板（设置页「主题色板」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentPalette {
    Cyan,
    Magenta,
    Violet,
}

impl AccentPalette {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cyan" => Some(AccentPalette::Cyan),
            "magenta" => Some(AccentPalette::Magenta),
            "violet" => Some(AccentPalette::Violet),
            _ => None,
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            AccentPalette::Cyan => "cyan",
            AccentPalette::Magenta => "magenta",
            AccentPalette::Violet => "violet",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            AccentPalette::Cyan => "赛博青 · Cyan Pulse",
            AccentPalette::Magenta => "霓虹粉 · Magenta Drive",
            AccentPalette::Violet => "等离紫 · Violet Core",
        }
    }

    /// 主强调色（活动导航/搜索焦点/选中）。
    pub fn color(&self) -> Hsla {
        match self {
            AccentPalette::Cyan => Neon::cyan(),
            AccentPalette::Magenta => Neon::magenta(),
            AccentPalette::Violet => Neon::violet(),
        }
    }

    pub fn next(&self) -> AccentPalette {
        match self {
            AccentPalette::Cyan => AccentPalette::Magenta,
            AccentPalette::Magenta => AccentPalette::Violet,
            AccentPalette::Violet => AccentPalette::Cyan,
        }
    }
}

/// sense 卡片左侧强调色轮转（设计稿 --accent 循环）。
pub fn sense_accent(i: usize) -> Hsla {
    [Neon::cyan(), Neon::magenta(), Neon::violet(), Neon::lime()][i % 4]
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

/// 半透明变体（glow 阴影/底色用）。
pub fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

/// 白色半透明（设计稿大量 rgba(255,255,255,x) 底色）。
pub fn white(a: f32) -> Hsla {
    rgba(255, 255, 255, a)
}

/// 钢青灰半透明（设计稿 rgba(120,160,200,x) 描边）。
pub fn steel(a: f32) -> Hsla {
    rgba(120, 160, 200, a)
}

/// 任意角度线性渐变（两色端点 0%→100%）。
pub fn grad(angle: f32, from: Hsla, to: Hsla) -> Background {
    linear_gradient(angle, linear_color_stop(from, 0.0), linear_color_stop(to, 1.0))
}

/// 发光阴影（box-shadow: 0 0 blur color）。
pub fn glow(color: Hsla, alpha: f32, blur: f32) -> BoxShadow {
    BoxShadow {
        color: with_alpha(color, alpha),
        offset: Point::new(px(0.), px(0.)),
        blur_radius: px(blur),
        spread_radius: px(0.),
        inset: false,
    }
}

/// 应用到 gpui-kit 全局主题（UI 字体 + 基础暗色；Mono 由各处局部指定）。
pub fn apply(cx: &mut App) {
    Theme::update(cx, |theme| {
        theme.font_family = UI_FONT.into();
        theme.colors.background = Neon::bg();
        theme.colors.foreground = Neon::text();
        theme.colors.border = Neon::border();
        theme.colors.accent = Neon::cyan();
        theme.colors.secondary = Neon::bg_elev();
    });
}
