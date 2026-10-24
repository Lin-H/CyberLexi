//! UI ↔ 后台 worker 的事件协议（纯类型，无 GPUI 依赖）。
//!
//! 线程模型：GPUI 主线程通过 `cmd_tx` 投递 [`UiCommand`]；
//! worker（独立 tokio 运行时线程）通过 std mpsc 回投 [`BackendEvent`]。

use std::path::PathBuf;

use crate::domain::types::QueryResult;
use crate::domain::wordbook::{WordBookItem, WordStatus};
use crate::infra::config::Config;

#[derive(Debug)]
pub enum UiCommand {
    /// 查询（300ms 防抖后由 UI 发出）
    Query(String),
    /// 点击中文候选后的详情补全
    QueryDetail(String),
    /// 播放音频（MW audio code + sound.ref）
    PlayAudio { code: String, ref_: String },
    AddToWordBook { key: String, headword: String },
    RemoveFromWordBook(String),
    SetWordStatus { key: String, status: WordStatus },
    SaveConfig(Config),
    ResetMwQuota,
    OpenDataDir,
    ExportPackage(PathBuf),
    ImportPackage(PathBuf),
    Shutdown,
}

#[derive(Debug)]
pub enum BackendEvent {
    QueryStarted(String),
    QueryFinished {
        input: String,
        result: Box<QueryResult>,
    },
    /// 在线链可用性变化（无 Key / Key 失效 / 熔断）
    OfflineMode(bool),
    Quota { used: u32, limit: u32 },
    ImportProgress { pct: f32, stage: String },
    ImportDone { ecdict_rows: u64, cedict_rows: u64 },
    ImportFailed(String),
    AudioReady { code: String, path: PathBuf },
    AudioFailed { code: String, reason: String },
    ConfigSaved(Config),
    WordBookChanged(Vec<WordBookItem>),
    HistoryChanged(Vec<String>),
    PackageDone(Result<String, String>),
    /// 后台线程致命错误（UI 显示后可退出）
    Fatal(String),
}
