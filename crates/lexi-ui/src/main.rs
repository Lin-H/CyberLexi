//! NEONLEX 入口：图标资产注册 → 后台 worker → GPUI 窗口。
//!
//! GPUI 主线程零阻塞（硬约束）：所有 IO 在 lexi-core worker 的
//! tokio 运行时执行，UI 经 UiCommand / BackendEvent 通信。

mod app;
mod book_view;
mod dict_view;
mod effects;
mod settings_view;
mod sidebar;
mod theme;
mod topbar;
mod widgets;

use std::borrow::Cow;
use std::sync::{OnceLock, RwLock};

use gpui_kit::*;
use lexi_core::events::UiCommand;
use lexi_core::infra::config::{AppearanceCfg, AudioCfg, Config};
use lexi_core::infra::paths::AppPaths;
use lexi_core::worker;
use tokio::sync::mpsc::UnboundedSender;

/// 全局命令通道：gpui-kit 组件回调无 view 上下文时经此转发。
static CMD_TX: OnceLock<UnboundedSender<UiCommand>> = OnceLock::new();

/// 配置镜像：静态回调（Switch/循环选择器）读改写配置用。
/// 权威数据仍在 worker（config.toml）；镜像随 ConfigSaved 事件刷新。
static CFG: OnceLock<RwLock<Config>> = OnceLock::new();

pub fn send_global(cmd: UiCommand) {
    if let Some(tx) = CMD_TX.get() {
        let _ = tx.send(cmd);
    }
}

/// 刷新配置镜像（ConfigSaved / 启动广播时调用）。
pub fn update_cfg_mirror(cfg: Config) {
    let lock = CFG.get_or_init(|| RwLock::new(Config::default()));
    if let Ok(mut guard) = lock.write() {
        *guard = cfg;
    }
}

/// 读取配置镜像副本。
pub fn cfg_snapshot() -> Config {
    let lock = CFG.get_or_init(|| RwLock::new(Config::default()));
    lock.read().map(|g| g.clone()).unwrap_or_default()
}

/// 读改写外观配置并发送保存（即改即存）。
pub fn patch_appearance(f: impl FnOnce(&mut AppearanceCfg)) {
    let mut cfg = cfg_snapshot();
    f(&mut cfg.appearance);
    send_global(UiCommand::SaveConfig(cfg));
}

/// 读改写发音配置并发送保存。
pub fn patch_audio(f: impl FnOnce(&mut AudioCfg)) {
    let mut cfg = cfg_snapshot();
    f(&mut cfg.audio);
    send_global(UiCommand::SaveConfig(cfg));
}

// Ctrl+K 聚焦搜索。
actions!(neonlex, [FocusSearch]);

fn main() -> anyhow::Result<()> {
    let paths = AppPaths::resolve()?;
    tracing::info!(data_dir = paths.data_dir.display().to_string(), "starting");

    let handle = worker::spawn(paths.clone())?;
    let _ = CMD_TX.set(handle.cmd_tx.clone());

    application()
        // 全量 Lucide 图标目录（内嵌；含本 UI 所有的 SVG 图标路径）
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);

            // Ctrl+K → FocusSearch（全局绑定，无 key context 限定）
            cx.bind_keys([KeyBinding::new("ctrl-k", FocusSearch, None)]);

            // 随包字体：JetBrains Mono（等宽主字体）+ Noto Sans SC（CJK 兜底）。
            // 界面字体 "Segoe UI" 由系统字体解析；中文走 GPUI 的
            // DirectWrite fallback（Noto 已内嵌，待实测）。
            load_fonts(&paths, cx);

            theme::apply(cx);

            let _window = open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(1320.), px(880.)),
                    })),
                    titlebar: Some(TitlebarOptions {
                        title: Some("NEONLEX — 赛博英语词典".into()),
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
