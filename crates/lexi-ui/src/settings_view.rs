//! 设置视图：外观（色板/扫描线/网格）、发音（口音/自动播放/语速）、
//! 数据（MW Key/缓存/词库/数据包）。设计稿 #view-settings + Phase 1
//! 必备数据管理项。开关即改即存：读全局配置镜像 → 改单字段 →
//! SaveConfig（worker 落盘并回广播刷新镜像）。

use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::*;

use crate::app::AppView;
use lexi_core::events::UiCommand;

use crate::theme::{grad, hex, white, with_alpha, Neon};
use crate::widgets::{cycle_select, panel};

pub fn render(view: &mut AppView, _cx: &mut Context<AppView>) -> Div {
    let mut wrap = div().flex().flex_col().gap_4().max_w(px(860.));

    // ── 外观 ──
    {
        let appearance = view.cfg.appearance.clone();
        let accent = view.accent;
        wrap = wrap.child(
            group("外观 · APPEARANCE", "调整界面的视觉风格与显示效果")
                .child(row(
                    "主题色板",
                    "选择霓虹高亮的配色方案",
                    cycle_select("set-accent", accent.label(), move |_, _, _| {
                        crate::patch_appearance(|a| a.accent = accent.next().key().to_string());
                    })
                    .into_any_element(),
                ))
                .child(row(
                    "扫描线特效",
                    "在界面层叠加 CRT 扫描线纹理",
                    Switch::new("set-scanlines")
                        .checked(appearance.scanlines)
                        .color(Neon::cyan())
                        .on_click(move |checked, _, _| {
                            crate::patch_appearance(|a| a.scanlines = *checked);
                        })
                        .into_any_element(),
                ))
                .child(row(
                    "背景网格",
                    "显示等距科技网格背景",
                    Switch::new("set-grid")
                        .checked(appearance.grid)
                        .color(Neon::cyan())
                        .on_click(move |checked, _, _| {
                            crate::patch_appearance(|a| a.grid = *checked);
                        })
                        .into_any_element(),
                )),
        );
    }

    // ── 发音 ──
    {
        let audio = view.cfg.audio.clone();
        let speed_label = speed_display(audio.speed);
        wrap = wrap.child(
            group("发音 · PRONUNCIATION", "控制单词的语音播放方式")
                .child(row(
                    "口音偏好",
                    "MW 音频源仅提供美式发音（en/us）",
                    // 诚实起见不提供无效选项；英音/TTS 源待 Phase 2
                    div()
                        .px(px(12.))
                        .py(px(7.))
                        .rounded_md()
                        .bg(Neon::bg_input())
                        .border_1()
                        .border_color(Neon::border())
                        .text_xs()
                        .text_color(Neon::muted())
                        .child("美式 · MW Audio")
                        .into_any_element(),
                ))
                .child(row(
                    "自动发音",
                    "查询单词后自动播放读音",
                    Switch::new("set-autoplay")
                        .checked(audio.auto_play)
                        .color(Neon::cyan())
                        .on_click(move |checked, _, _| {
                            crate::patch_audio(|a| a.auto_play = *checked);
                        })
                        .into_any_element(),
                ))
                .child(row(
                    "发音语速",
                    "调节语音播放的速度（0.75x / 1.0x / 1.25x 循环）",
                    cycle_select("set-speed", speed_label, move |_, _, _| {
                        crate::patch_audio(|a| a.speed = next_speed(a.speed));
                    })
                    .into_any_element(),
                )),
        );
    }

    // ── 数据 ──
    {
        let key_input = view.key_input.clone();
        let ttl_input = view.ttl_input.clone();
        let max_input = view.max_input.clone();
        let import_path_input = view.import_path_input.clone();
        let (ecdict_n, cedict_n) = view.dict_counts;

        wrap = wrap.child(
            group("数据 · DATA", "管理本地词库、缓存与学习记录")
                // MW API Key
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .py_3()
                        .border_b_1()
                        .border_color(white(0.06))
                        .child(
                            div()
                                .text_sm()
                                .text_color(hex("#cfe6f5"))
                                .child("Merriam-Webster API Key"),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(Neon::sense_hint())
                                .child(
                                    "仅存本机 config.toml（已 gitignore），绝不编译进二进制；\
                                     留空 = 离线模式。非商用额度 1000 次/天。",
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(div().flex_1().child(Input::new(&key_input)))
                                .child(
                                    div()
                                        .id("set-save-key")
                                        .cursor_pointer()
                                        .flex()
                                        .items_center()
                                        .px(px(14.))
                                        .py(px(7.))
                                        .rounded_md()
                                        .bg(grad(90., Neon::cyan(), hex("#7de9ff")))
                                        .shadow(vec![crate::theme::glow(Neon::cyan(), 0.45, 24.)])
                                        .hover(|d| {
                                            d.shadow(vec![crate::theme::glow(Neon::cyan(), 0.75, 34.)])
                                        })
                                        .on_click({
                                            let key_input = key_input.clone();
                                            let ttl_input = ttl_input.clone();
                                            let max_input = max_input.clone();
                                            move |_, _, cx| {
                                                let key = key_input.read(cx).value().to_string();
                                                let ttl = ttl_input
                                                    .read(cx)
                                                    .value()
                                                    .trim()
                                                    .parse()
                                                    .unwrap_or(30);
                                                let max = max_input
                                                    .read(cx)
                                                    .value()
                                                    .trim()
                                                    .parse()
                                                    .unwrap_or(50_000);
                                                // 以镜像为基础（保留外观/发音设置）
                                                let mut cfg = crate::cfg_snapshot();
                                                cfg.mw_api_key = key;
                                                cfg.cache_ttl_days = ttl;
                                                cfg.cache_max_rows = max;
                                                crate::send_global(UiCommand::SaveConfig(cfg));
                                            }
                                        })
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(hex("#03121a"))
                                                .child("保存（改完即用）"),
                                        ),
                                ),
                        ),
                )
                // 缓存
                .child(row(
                    "缓存刷新周期（天）",
                    "到期后查询重新拉取 MW 并覆盖；缓存只刷新、永不清除",
                    div()
                        .w(px(160.))
                        .child(Input::new(&ttl_input))
                        .into_any_element(),
                ))
                .child(row(
                    "缓存行数上限",
                    "超过上限才按 hit_count / fetched_at 冷淘汰（默认几乎不触发）",
                    div()
                        .w(px(160.))
                        .child(Input::new(&max_input))
                        .into_any_element(),
                ))
                // 离线词库
                .child(row(
                    "离线词库",
                    format!("ECDICT {ecdict_n} 条 · CEDICT {cedict_n} 条"),
                    div()
                        .id("set-reimport")
                        .cursor_pointer()
                        .px(px(14.))
                        .py(px(7.))
                        .rounded_md()
                        .bg(with_alpha(Neon::cyan(), 0.1))
                        .border_1()
                        .border_color(with_alpha(Neon::cyan(), 0.5))
                        .text_sm()
                        .text_color(Neon::cyan())
                        .hover(|d| {
                            d.bg(with_alpha(Neon::cyan(), 0.2))
                                .shadow(vec![crate::theme::glow(Neon::cyan(), 0.3, 18.)])
                        })
                        .on_click(|_, _, _| {
                            crate::send_global(UiCommand::ReimportDicts);
                        })
                        .child("重新导入")
                        .into_any_element(),
                ))
                // 数据目录 / 导出 / 配额
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_2()
                        .py_3()
                        .child(action_chip("set-open-dir", "打开数据目录", |_, _, _| {
                            crate::send_global(UiCommand::OpenDataDir);
                        }))
                        .child(action_chip("set-export", "导出数据包", |_, _, _| {
                            let dir = lexi_core::infra::paths::AppPaths::resolve()
                                .map(|p| p.export_dir())
                                .unwrap_or_default();
                            let _ = std::fs::create_dir_all(&dir);
                            let out = dir.join(format!("cyberlexi-{}.zip", day_stamp()));
                            crate::send_global(UiCommand::ExportPackage(out));
                        }))
                        .child(action_chip("set-reset-quota", "重置 MW 今日计数", |_, _, _| {
                            crate::send_global(UiCommand::ResetMwQuota);
                        })),
                )
                // 导入
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .pb_1()
                        .child(
                            div()
                                .flex_1()
                                .child(Input::new(&import_path_input).appearance(false)),
                        )
                        .child(action_chip("set-import", "导入数据包", {
                            let import_path_input = import_path_input.clone();
                            move |_, _, cx| {
                                let path =
                                    import_path_input.read(cx).value().trim().to_string();
                                if !path.is_empty() {
                                    crate::send_global(UiCommand::ImportPackage(
                                        std::path::PathBuf::from(path),
                                    ));
                                }
                            }
                        })),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(Neon::sense_hint())
                        .child("数据包 = db + config + 音频缓存（zip）；导入完成后重启应用生效"),
                ),
        );
    }

    wrap
}

fn group(title: &str, desc: &str) -> Div {
    panel().p(px(22.)).child(
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .mb(px(16.))
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(Neon::cyan_light())
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(Neon::sense_hint())
                    .child(desc.to_string()),
            ),
    )
}

fn row(label: &str, hint: impl Into<SharedString>, control: AnyElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_5()
        .py_3()
        .border_b_1()
        .border_color(white(0.06))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(3.))
                .child(
                    div()
                        .text_sm()
                        .text_color(hex("#cfe6f5"))
                        .child(label.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(Neon::sense_hint())
                        .child(hint.into()),
                ),
        )
        .child(control)
}

/// 次级操作 chip（描边按钮）。
fn action_chip(
    id: &str,
    label: &str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    div()
        .id(id.to_string())
        .cursor_pointer()
        .px(px(14.))
        .py(px(7.))
        .rounded_md()
        .bg(white(0.03))
        .border_1()
        .border_color(Neon::border())
        .text_sm()
        .text_color(hex("#cfe6f5"))
        .hover(|d| {
            d.border_color(with_alpha(Neon::cyan(), 0.45)).text_color(Neon::cyan())
        })
        .on_click(on_click)
        .child(label.to_string())
}

fn speed_display(speed: f32) -> &'static str {
    if (speed - 0.75).abs() < 0.01 {
        "慢速 0.75x"
    } else if (speed - 1.25).abs() < 0.01 {
        "快速 1.25x"
    } else {
        "正常 1.0x"
    }
}

fn next_speed(speed: f32) -> f32 {
    if (speed - 0.75).abs() < 0.01 {
        1.0
    } else if (speed - 1.0).abs() < 0.01 {
        1.25
    } else {
        0.75
    }
}

fn day_stamp() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    (secs / 86_400 + 19_000) as i64
}
