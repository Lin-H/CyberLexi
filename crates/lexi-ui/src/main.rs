//! CyberLexi 入口：解析路径 → 启动后台 worker → 打开 GPUI 窗口。
//!
//! GPUI 主线程零阻塞（硬约束）：所有 IO 在 lexi-core worker 的
//! tokio 运行时执行，UI 经 UiCommand / BackendEvent 通信。

mod app;
mod components;
mod theme;

use std::borrow::Cow;
use std::sync::OnceLock;

use gpui_kit::*;
use lexi_core::events::UiCommand;
use lexi_core::infra::paths::AppPaths;
use lexi_core::worker;
use tokio::sync::mpsc::UnboundedSender;

/// 全局命令通道：gpui-kit Button 回调无 view 上下文时经此转发。
static CMD_TX: OnceLock<UnboundedSender<UiCommand>> = OnceLock::new();

pub fn send_global(cmd: UiCommand) {
    if let Some(tx) = CMD_TX.get() {
        let _ = tx.send(cmd);
    }
}

fn main() -> anyhow::Result<()> {
    let paths = AppPaths::resolve()?;
    tracing::info!(data_dir = paths.data_dir.display().to_string(), "starting");

    let handle = worker::spawn(paths.clone())?;
    let _ = CMD_TX.set(handle.cmd_tx.clone());

    application().run(move |cx| {
        gpui_kit::init(cx);

        // 随包字体：JetBrains Mono（拉丁/等宽主字体）+ Noto Sans SC（CJK）。
        // 依赖 GPUI 的 script fallback：中文以 Noto Sans SC 渲染——
        // 待 Windows 实测；若出现豆腐块，将 theme.font_family 改为
        // "Noto Sans SC"（降级方案，等宽性损失）。
        load_fonts(&paths, cx);

        theme::apply(cx);

        let _window = open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Default::default(),
                    size: size(px(1180.), px(800.)),
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("CyberLexi".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    app::AppView::new(window, cx, handle.cmd_tx.clone(), handle.event_rx)
                })
            },
        )
        .expect("open window");
    });
    Ok(())
}

fn load_fonts(paths: &AppPaths, cx: &mut App) {
    let dir = paths.fonts();
    let mut fonts: Vec<Cow<'static, [u8]>> = Vec::new();
    for name in [
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono-Bold.ttf",
        "NotoSansSC-Regular.otf",
        "NotoSansSC-Bold.otf",
    ] {
        match std::fs::read(dir.join(name)) {
            Ok(bytes) => fonts.push(Cow::Owned(bytes)),
            Err(e) => {
                tracing::warn!(font = name, error = %e, "font missing, falling back to system font")
            }
        }
    }
    if !fonts.is_empty() {
        if let Err(e) = cx.text_system().add_fonts(fonts) {
            tracing::error!(error = %e, "add_fonts failed");
        }
    }
}
