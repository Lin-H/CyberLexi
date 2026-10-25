//! 后台 worker：独立线程内跑 tokio 运行时，承接全部网络/DB/文件 IO。
//!
//! GPUI 主线程永不触碰这些资源：命令经 `UiCommand` 入队，结果经
//! std mpsc 回投 `BackendEvent`。同 key 查询在途去重（新请求 abort
//! 上一轮未完成任务）。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::domain::normalizer::Normalizer;
use crate::domain::wordbook::{WordBook, WordBookItem, WordStatus};
use crate::events::{BackendEvent, UiCommand};
use crate::infra::audio;
use crate::infra::config::Config;
use crate::infra::db::{Db, DbHandle, HistoryRepo, MetaRepo, SqliteWordBookStore};
use crate::infra::importer::{cedict, ecdict, data_fingerprint, mark_imported, needs_import};
use crate::infra::package;
use crate::infra::paths::AppPaths;
use crate::orchestrator::{build_orchestrator, QueryOrchestrator};

pub struct WorkerHandle {
    pub cmd_tx: mpsc::UnboundedSender<UiCommand>,
    pub event_rx: std::sync::mpsc::Receiver<BackendEvent>,
}

/// 启动后台线程（唯一入口；UI 侧持有 WorkerHandle）。
pub fn spawn(paths: AppPaths) -> anyhow::Result<WorkerHandle> {
    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    std::thread::Builder::new()
        .name("lexi-worker".into())
        .spawn(move || {
            if let Err(e) = run(paths, cmd_rx, event_tx.clone()) {
                tracing::error!(error = %e, "worker fatal");
                let _ = event_tx.send(BackendEvent::Fatal(e.to_string()));
            }
        })?;
    Ok(WorkerHandle { cmd_tx, event_rx })
}

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn run(
    paths: AppPaths,
    mut cmd_rx: mpsc::UnboundedReceiver<UiCommand>,
    event_tx: std::sync::mpsc::Sender<BackendEvent>,
) -> anyhow::Result<()> {
    init_tracing(&paths)?;

    // 上次导入留下的 staging：先落盘再开库
    if let Err(e) = apply_import_staging(&paths) {
        tracing::warn!(error = %e, "apply import staging failed");
    }

    let db: DbHandle = Arc::new(Db::open(&paths.db_path())?);
    let meta = Arc::new(MetaRepo::new(db.clone()));
    let cfg = Config::load(&paths.config_path());

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(serve(paths, db, meta, cfg, &mut cmd_rx, event_tx))
}

fn init_tracing(paths: &AppPaths) -> anyhow::Result<()> {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let appender = tracing_appender::rolling::daily(paths.logs_dir(), "cyberlexi.log");
    let (writer, _guard) = tracing_appender::non_blocking(appender);
    // writer 需要存活整个进程：leak guard（worker 线程生命周期 = 进程）
    std::mem::forget(_guard);
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false),
        )
        .try_init()
        .ok();
    Ok(())
}

async fn serve(
    paths: AppPaths,
    db: DbHandle,
    meta: Arc<MetaRepo>,
    cfg: Config,
    cmd_rx: &mut mpsc::UnboundedReceiver<UiCommand>,
    event_tx: std::sync::mpsc::Sender<BackendEvent>,
) -> anyhow::Result<()> {
    tracing::info!(data_dir = paths.data_dir.display().to_string(), "worker starting");

    // 首启导入：data_version 指纹不同才重导
    run_initial_import(&paths, &db, &meta, &event_tx).await;

    let orch = build_orchestrator(db.clone(), cfg.clone(), Some(meta.clone()), now_secs());
    let wordbook = WordBook::new(SqliteWordBookStore::new(db.clone()));
    let history = HistoryRepo::new(db.clone());
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    // 初始状态广播
    let _ = event_tx.send(BackendEvent::ConfigSaved(cfg.clone()));
    let _ = event_tx.send(BackendEvent::OfflineMode(!orch.online_enabled().await));
    if let Some((used, limit)) = current_quota(&orch).await {
        let _ = event_tx.send(BackendEvent::Quota { used, limit });
    }
    let _ = event_tx.send(BackendEvent::WordBookChanged(wordbook.list()?));
    let _ = event_tx.send(BackendEvent::HistoryChanged(history.recent()?));
    push_stats(&history, &event_tx, now_secs());
    {
        let ecdict_n = crate::infra::db::EcdictRepo::new(db.clone()).count().unwrap_or(0);
        let cedict_n = crate::infra::db::CedictRepo::new(db.clone()).count().unwrap_or(0);
        let _ = event_tx.send(BackendEvent::DictStats { ecdict: ecdict_n, cedict: cedict_n });
    }

    let inflight: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let normalizer = Normalizer::l1();

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            UiCommand::Query(raw) => {
                let key = normalizer.normalize(&raw).key;
                let _ = event_tx.send(BackendEvent::QueryStarted(raw.clone()));
                spawn_query(
                    &inflight,
                    &event_tx,
                    &orch,
                    &history,
                    raw,
                    key,
                    false,
                );
            }
            UiCommand::QueryDetail(raw) => {
                let key = normalizer.normalize(&raw).key;
                let _ = event_tx.send(BackendEvent::QueryStarted(raw.clone()));
                spawn_query(&inflight, &event_tx, &orch, &history, raw, key, true);
            }
            UiCommand::PlayAudio { code, ref_ } => {
                let http = http.clone();
                let dir = paths.audio_dir();
                let etx = event_tx.clone();
                let speed = orch.config().await.audio.speed;
                tokio::spawn(async move {
                    match audio::ensure_downloaded(&http, &dir, &code, &ref_).await {
                        Ok(path) => {
                            let _ = etx.send(BackendEvent::AudioReady {
                                code: code.clone(),
                                path: path.clone(),
                            });
                            // 播放放独立阻塞线程（OutputStream 生命周期=播完）
                            let res = std::thread::spawn(move || {
                                audio::play_blocking(&path, speed)
                            })
                            .join()
                            .map_err(|_| anyhow::anyhow!("audio thread panicked"))
                            .and_then(|r| r);
                            if let Err(e) = res {
                                let _ = etx.send(BackendEvent::AudioFailed {
                                    code,
                                    reason: e.to_string(),
                                });
                            }
                        }
                        Err(e) => {
                            let _ = etx.send(BackendEvent::AudioFailed { code, reason: e.to_string() });
                        }
                    }
                });
            }
            UiCommand::AddToWordBook { key, headword } => {
                let now = now_secs();
                let item = WordBookItem {
                    word_key: key,
                    headword,
                    status: WordStatus::New,
                    tags: Vec::new(),
                    note: String::new(),
                    added_at: now,
                    updated_at: now,
                };
                let _ = wordbook.add(item);
                let _ = event_tx.send(BackendEvent::WordBookChanged(wordbook.list()?));
            }
            UiCommand::RemoveFromWordBook(key) => {
                let _ = wordbook.remove(&key);
                let _ = event_tx.send(BackendEvent::WordBookChanged(wordbook.list()?));
            }
            UiCommand::SetWordStatus { key, status } => {
                let _ = wordbook.set_status(&key, status, now_secs());
                let _ = event_tx.send(BackendEvent::WordBookChanged(wordbook.list()?));
            }
            UiCommand::SaveConfig(new_cfg) => {
                if let Err(e) = new_cfg.save(&paths.config_path()) {
                    let _ = event_tx.send(BackendEvent::PackageDone(Err(format!("save config: {e}"))));
                    continue;
                }
                orch.update_config(new_cfg.clone(), meta.clone(), now_secs()).await?;
                let _ = event_tx.send(BackendEvent::ConfigSaved(new_cfg));
                let _ = event_tx.send(BackendEvent::OfflineMode(!orch.online_enabled().await));
                if let Some((used, limit)) = current_quota(&orch).await {
                    let _ = event_tx.send(BackendEvent::Quota { used, limit });
                }
            }
            UiCommand::ResetMwQuota => {
                if let Some(client) = orch.mw_client().await {
                    client.reset_quota()?;
                }
                if let Some((used, limit)) = current_quota(&orch).await {
                    let _ = event_tx.send(BackendEvent::Quota { used, limit });
                }
            }
            UiCommand::OpenDataDir => {
                let _ = std::process::Command::new("explorer").arg(&paths.data_dir).spawn();
            }
            UiCommand::ExportPackage(out_path) => {
                let data_dir = paths.data_dir.clone();
                let db_path = paths.db_path();
                let etx = event_tx.clone();
                tokio::task::spawn_blocking(move || {
                    let res = package::export_package(&data_dir, &db_path, &out_path)
                        .map(|_| out_path.display().to_string());
                    let _ = etx.send(BackendEvent::PackageDone(
                        res.map(|p| format!("已导出：{p}"))
                            .map_err(|e| e.to_string()),
                    ));
                })
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            }
            UiCommand::ImportPackage(zip_path) => {
                // 热文件被占用：解压到 staging，提示重启后由启动序列应用
                let staging = paths.data_dir.join("import_staging");
                let etx = event_tx.clone();
                let res = tokio::task::spawn_blocking(move || {
                    let n = package::import_package(&zip_path, &staging)?;
                    Ok::<usize, anyhow::Error>(n)
                })
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                let _ = etx.send(BackendEvent::PackageDone(
                    res.map(|n| format!("已导入 {n} 个文件，重启应用后生效"))
                        .map_err(|e| e.to_string()),
                ));
            }
            UiCommand::RandomWord => {
                // ECDICT 高频词随机抽取 → 走完整查询链（含历史记录/统计）
                let pick = tokio::task::spawn_blocking({
                    let db = db.clone();
                    move || crate::infra::db::EcdictRepo::new(db).random_word(20_000)
                })
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
                match pick {
                    Ok(Some(word)) => {
                        tracing::debug!(word, "random word picked");
                        let key = normalizer.normalize(&word).key;
                        let _ = event_tx.send(BackendEvent::QueryStarted(word.clone()));
                        spawn_query(&inflight, &event_tx, &orch, &history, word, key, false);
                    }
                    Ok(None) => {
                        let _ = event_tx.send(BackendEvent::PackageDone(Err(
                            "离线词库为空，无法随机抽词".into(),
                        )));
                    }
                    Err(e) => {
                        let _ = event_tx.send(BackendEvent::PackageDone(Err(e.to_string())));
                    }
                }
            }
            UiCommand::RequestWordBookDetails => {
                let items = wordbook.list()?;
                let summaries = orch.wordbook_summaries(&items, now_secs()).await;
                let _ = event_tx.send(BackendEvent::WordBookDetails(summaries));
            }
            UiCommand::ReimportDicts => {
                // 抹掉指纹标记触发全量重导（importer 自身会先 clear）
                if let Err(e) = meta.set("data_version", "force") {
                    let _ = event_tx.send(BackendEvent::PackageDone(Err(e.to_string())));
                    continue;
                }
                let paths2 = paths.clone();
                let db2 = db.clone();
                let meta2 = meta.clone();
                let etx = event_tx.clone();
                tokio::spawn(async move {
                    run_initial_import(&paths2, &db2, &meta2, &etx).await;
                });
            }
            UiCommand::Shutdown => {
                tracing::info!("worker shutdown");
                break;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_query(
    inflight: &Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
    orch: &Arc<QueryOrchestrator>,
    history: &HistoryRepo,
    raw: String,
    key: String,
    detail: bool,
) {
    // 在途去重：同 key 新请求取消上一轮
    if let Some(old) = inflight.lock().unwrap().remove(&key) {
        old.abort();
        tracing::debug!(key, "aborted previous in-flight query");
    }
    let orch = orch.clone();
    let etx = event_tx.clone();
    let history = history.clone();
    let task_key = key.clone();
    let handle = tokio::spawn(async move {
        let result = if detail {
            orch.query_detail(&raw, now_secs()).await
        } else {
            orch.query(&raw, now_secs()).await
        };
        match result {
            Ok(r) => {
                let _ = etx.send(BackendEvent::QueryFinished {
                    input: raw.clone(),
                    result: Box::new(r),
                });
                let _ = etx.send(BackendEvent::HistoryChanged(history.recent().unwrap_or_default()));
                push_stats(&history, &etx, now_secs());
            }
            Err(e) => {
                tracing::error!(error = %e, "query failed");
                let _ = etx.send(BackendEvent::QueryFinished {
                    input: raw.clone(),
                    result: Box::new(crate::domain::types::QueryResult::NotFound {
                        input: raw,
                        suggestions: Vec::new(),
                    }),
                });
            }
        }
    });
    inflight.lock().unwrap().insert(task_key, handle);
}

async fn current_quota(orch: &Arc<QueryOrchestrator>) -> Option<(u32, u32)> {
    let client = orch.mw_client().await?;
    Some(client.quota())
}

/// 每日查词目标（侧边栏进度条分母；Phase 2 交由学习计划接管）。
const DAILY_GOAL: u32 = 20;

fn push_stats(history: &HistoryRepo, event_tx: &std::sync::mpsc::Sender<BackendEvent>, now: i64) {
    match history.stats(now) {
        Ok((today, streak)) => {
            let _ = event_tx.send(BackendEvent::StatsUpdated {
                today_count: today,
                goal: DAILY_GOAL,
                streak_days: streak,
            });
        }
        Err(e) => tracing::warn!(error = %e, "stats compute failed"),
    }
}

/// 首启导入：数据文件指纹变化时全量重导（进度经事件上报）。
async fn run_initial_import(
    paths: &AppPaths,
    db: &DbHandle,
    meta: &Arc<MetaRepo>,
    event_tx: &std::sync::mpsc::Sender<BackendEvent>,
) {
    let (ecdict_csv, cedict_txt) = (paths.ecdict_csv(), paths.cedict_txt());
    if !ecdict_csv.exists() || !cedict_txt.exists() {
        tracing::warn!("assets dict files missing; running with empty offline dicts");
        let _ = event_tx.send(BackendEvent::ImportFailed(
            "assets/dict 数据文件缺失，离线词库为空".into(),
        ));
        return;
    }
    let fp = match data_fingerprint(&ecdict_csv, &cedict_txt) {
        Ok(f) => f,
        Err(e) => {
            let _ = event_tx.send(BackendEvent::ImportFailed(e.to_string()));
            return;
        }
    };
    if !needs_import(meta, &fp) {
        tracing::info!("offline dicts up to date");
        return;
    }
    tracing::info!("importing offline dicts");
    let (tx, mut rx) = mpsc::unbounded_channel::<(f32, String)>();
    let etx = event_tx.clone();
    let fwd = tokio::spawn(async move {
        while let Some((pct, stage)) = rx.recv().await {
            let _ = etx.send(BackendEvent::ImportProgress { pct, stage });
        }
    });

    let ecdict_rows = ecdict::import_ecdict(db.clone(), ecdict_csv, tx.clone()).await;
    let cedict_rows = cedict::import_cedict(db.clone(), cedict_txt, tx).await;
    drop(fwd);

    match (ecdict_rows, cedict_rows) {
        (Ok(a), Ok(b)) => {
            if let Err(e) = mark_imported(meta, &fp) {
                tracing::warn!(error = %e, "mark data_version failed");
            }
            tracing::info!(ecdict_rows = a, cedict_rows = b, "offline dicts imported");
            let _ = event_tx.send(BackendEvent::ImportDone {
                ecdict_rows: a,
                cedict_rows: b,
            });
            // 重导后刷新设置页词库计数
            let ecdict_n = crate::infra::db::EcdictRepo::new(db.clone()).count().unwrap_or(0);
            let cedict_n = crate::infra::db::CedictRepo::new(db.clone()).count().unwrap_or(0);
            let _ = event_tx.send(BackendEvent::DictStats { ecdict: ecdict_n, cedict: cedict_n });
        }
        (Err(e), _) | (_, Err(e)) => {
            let _ = event_tx.send(BackendEvent::ImportFailed(e.to_string()));
        }
    }
}

/// 应用上次会话 staging 的导入包（在打开数据库之前调用）。
fn apply_import_staging(paths: &AppPaths) -> anyhow::Result<()> {
    let staging = paths.data_dir.join("import_staging");
    if !staging.is_dir() {
        return Ok(());
    }
    for rel in ["db/lexi.sqlite", "db/lexi.sqlite-wal", "db/lexi.sqlite-shm", "config.toml"] {
        let src = staging.join(rel);
        if src.exists() {
            let dst = paths.data_dir.join(rel);
            if let Some(p) = dst.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&src, &dst)?;
        }
    }
    let src_audio = staging.join("audio");
    if src_audio.is_dir() {
        copy_dir_recursive(&src_audio, &paths.audio_dir())?;
    }
    std::fs::remove_dir_all(&staging)?;
    tracing::info!("import staging applied");
    Ok(())
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let p = entry.path();
        let target = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir_recursive(&p, &target)?;
        } else {
            std::fs::copy(&p, &target)?;
        }
    }
    Ok(())
}
