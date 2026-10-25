//! 背景特效层（设计稿 body::before/::after + .scanlines）：
//! - 等距网格（46px 青线，自顶向下径向淡出）
//! - CRT 扫描线（3px 周期 1px 暗线，multiply ≈ 半透明黑）
//! - 氛围光斑（左上青 / 右上紫 / 底部粉；GPUI 无径向渐变，
//!   用 inset box-shadow 的 GPU 模糊近似）
//!
//! 全部为纯装饰 overlay：不拦截指针、不参与布局（absolute inset-0）。

use gpui_kit::*;

use crate::theme::{with_alpha, Neon};

/// 网格间距（设计稿 46px）。
const GRID: f32 = 46.0;
/// 网格线透明度（设计稿 rgba(0,234,255,.045)）。
const GRID_ALPHA: f32 = 0.045;

/// 等距网格背景（absolute 全覆盖；垂直方向按高度衰减近似径向 mask）。
pub fn grid_layer() -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _cx| {
            let x0 = bounds.origin.x;
            let y0 = bounds.origin.y;
            let w = bounds.size.width.as_f32();
            let h = bounds.size.height.as_f32();
            if w <= 0.0 || h <= 0.0 {
                return;
            }
            let color = with_alpha(Neon::cyan(), GRID_ALPHA);
            // 垂直衰减：顶部 1.0 → ~72% 高度处 0（近似 ellipse mask）
            let fade = |y: f32| -> f32 {
                let t = (y / (h * 0.72)).min(1.0);
                1.0 - t * t
            };
            // 水平线（整条，按 y 衰减）
            let mut y = GRID;
            while y < h {
                let a = fade(y);
                if a <= 0.02 {
                    break;
                }
                window.paint_quad(quad(
                    Bounds {
                        origin: point(x0, y0 + px(y)),
                        size: size(px(w), px(1.0)),
                    },
                    px(0.0),
                    with_alpha(color, color.a * a),
                    px(0.0),
                    Hsla::transparent_black(),
                    BorderStyle::default(),
                ));
                y += GRID;
            }
            // 垂直线（分 6 段各自衰减，避免整条同 alpha）
            let seg = h / 6.0;
            let mut x = GRID;
            while x < w {
                for s in 0..6 {
                    let y0s = s as f32 * seg;
                    let a = fade(y0s + seg * 0.5);
                    if a <= 0.02 {
                        continue;
                    }
                    window.paint_quad(quad(
                        Bounds {
                            origin: point(x0 + px(x), y0 + px(y0s)),
                            size: size(px(1.0), px(seg.ceil())),
                        },
                        px(0.0),
                        with_alpha(color, color.a * a),
                        px(0.0),
                        Hsla::transparent_black(),
                        BorderStyle::default(),
                    ));
                }
                x += GRID;
            }
        },
    )
    .absolute()
    .size_full()
}

/// CRT 扫描线（3px 周期 1px 暗线；设计稿 opacity .30 × rgba(0,0,0,.6)）。
pub fn scanline_layer() -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _cx| {
            let x0 = bounds.origin.x;
            let y0 = bounds.origin.y;
            let w = bounds.size.width.as_f32();
            let h = bounds.size.height.as_f32();
            if w <= 0.0 || h <= 0.0 {
                return;
            }
            let color = Hsla { a: 0.18, ..black() };
            let mut y = 0.0;
            while y < h {
                window.paint_quad(quad(
                    Bounds {
                        origin: point(x0, y0 + px(y)),
                        size: size(px(w), px(1.0)),
                    },
                    px(0.0),
                    color,
                    px(0.0),
                    Hsla::transparent_black(),
                    BorderStyle::default(),
                ));
                y += 3.0;
            }
        },
    )
    .absolute()
    .size_full()
}

/// 氛围光斑（三处 inset 模糊光晕；approx radial-gradient）。
pub fn glow_layer() -> Div {
    div()
        .absolute()
        .size_full()
        // 左上青（600×380 @ 18% -6%）
        .child(
            div()
                .absolute()
                .left_0()
                .top(px(-40.))
                .w(px(700.))
                .h(px(480.))
                .shadow(vec![BoxShadow {
                    color: with_alpha(Neon::cyan(), 0.08),
                    offset: Point::new(px(0.), px(0.)),
                    blur_radius: px(220.),
                    spread_radius: px(40.),
                    inset: true,
                }]),
        )
        // 右上紫
        .child(
            div()
                .absolute()
                .right_0()
                .top_0()
                .w(px(660.))
                .h(px(500.))
                .shadow(vec![BoxShadow {
                    color: with_alpha(Neon::violet(), 0.08),
                    offset: Point::new(px(0.), px(0.)),
                    blur_radius: px(220.),
                    spread_radius: px(40.),
                    inset: true,
                }]),
        )
        // 底部粉
        .child(
            div()
                .absolute()
                .left_1_2()
                .bottom_0()
                .w(px(820.))
                .h(px(420.))
                .shadow(vec![BoxShadow {
                    color: with_alpha(Neon::magenta(), 0.05),
                    offset: Point::new(px(0.), px(0.)),
                    blur_radius: px(240.),
                    spread_radius: px(40.),
                    inset: true,
                }]),
        )
}
