//! SQLite 主库：连接管理、版本化 migration、meta 键值存储。
//!
//! 所有 SQL 写在仓库根 `migrations/` 下的版本化文件中，编译期
//! `include_str!` 嵌入，启动时按 `meta.schema_version` 顺序执行。
//! 连接包 `Mutex`（rusqlite Connection 是 Send 但非 Sync），
//! 供 tokio worker 通过 `spawn_blocking` 使用；GPUI 主线程永不触碰。

pub mod cache_repo;
pub mod cedict_repo;
pub mod ecdict_repo;
pub mod history_repo;
pub mod wordbook_repo;

pub use cache_repo::{CacheRepo, CachedEntry, SqliteCacheRepo, SweepReport};
pub use cedict_repo::CedictRepo;
pub use ecdict_repo::EcdictRepo;
pub use history_repo::HistoryRepo;
pub use wordbook_repo::SqliteWordBookStore;

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Context;
use rusqlite::Connection;

/// 版本化 migrations（有序）。文件名仅用于日志/排错。
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init.sql", include_str!("../../../../../migrations/0001_init.sql")),
    ("0002_fts.sql", include_str!("../../../../../migrations/0002_fts.sql")),
    (
        "0003_phase2_placeholder.sql",
        include_str!("../../../../../migrations/0003_phase2_placeholder.sql"),
    ),
];

/// 线程安全的数据库句柄。
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

pub type DbHandle = Arc<Db>;

impl Db {
    /// 打开（或创建）数据库文件并跑完 migrations。
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("open sqlite {}", path.display()))?;
        let db = Self::from_conn(conn)?;
        db.migrate().context("run migrations")?;
        Ok(db)
    }

    /// 内存库（单测用）。
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self::from_conn(conn)?;
        db.migrate()?;
        Ok(db)
    }

    fn from_conn(conn: Connection) -> anyhow::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// 在锁内执行同步数据库操作（调用方应位于 blocking 线程）。
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let guard = self.conn.lock().map_err(|e| anyhow::anyhow!("db lock poisoned: {e}"))?;
        f(&guard)
    }

    /// 按 meta.schema_version 顺序应用未执行的 migration。
    pub fn migrate(&self) -> anyhow::Result<()> {
        self.with_conn(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
            )?;
            let current: i64 = conn
                .query_row(
                    "SELECT value FROM meta WHERE key='schema_version'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            if current as usize >= MIGRATIONS.len() {
                return Ok(());
            }
            for (idx, (name, sql)) in MIGRATIONS.iter().enumerate() {
                let version = idx as i64 + 1;
                if version <= current {
                    continue;
                }
                // 单个 migration 包在事务里；FTS5 虚表语句 execute_batch 可处理
                conn.execute_batch(sql)
                    .with_context(|| format!("apply migration {name}"))?;
                conn.execute(
                    "INSERT INTO meta(key, value) VALUES('schema_version', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    [version.to_string()],
                )?;
                tracing::info!(migration = name, version, "applied");
            }
            Ok(())
        })
    }
}

/// meta 键值存储（schema_version / data_version / MW 配额计数）。
pub struct MetaRepo {
    db: DbHandle,
}

impl MetaRepo {
    pub fn new(db: DbHandle) -> Self {
        MetaRepo { db }
    }

    pub fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        self.db.with_conn(|c| {
            Ok(c.query_row(
                "SELECT value FROM meta WHERE key = ?1",
                [key],
                |r| r.get::<_, String>(0),
            )
            .optional()?)
        })
    }

    pub fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO meta(key, value) VALUES(?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        })
    }

    /// MW 当日配额（date 'YYYY-MM-DD', used）；跨天返回空日期。
    pub fn quota(&self, today: &str) -> anyhow::Result<(String, u32)> {
        if self.get("mw_quota_date")?.as_deref() == Some(today) {
            let used = self
                .get("mw_quota_used")?
                .and_then(|u| u.parse().ok())
                .unwrap_or(0);
            Ok((today.to_string(), used))
        } else {
            Ok((String::new(), 0))
        }
    }

    /// 配额 +1（跨天自动重置）。返回自增后的计数。
    pub fn quota_incr(&self, today: &str) -> anyhow::Result<u32> {
        let (date, used) = self.quota(today)?;
        let next = if date == today { used + 1 } else { 1 };
        self.set("mw_quota_date", today)?;
        self.set("mw_quota_used", &next.to_string())?;
        Ok(next)
    }

    /// 手动清零配额（设置页「重置今日计数」）。
    pub fn quota_reset(&self) -> anyhow::Result<()> {
        self.set("mw_quota_used", "0")
    }
}

trait ConnOptional<T> {
    fn optional(self) -> anyhow::Result<Option<T>>;
}

impl<T> ConnOptional<T> for rusqlite::Result<T> {
    fn optional(self) -> anyhow::Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        // 重跑两次：schema_version 不变、不报错
        db.migrate().unwrap();
        db.migrate().unwrap();
        let v = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT value FROM meta WHERE key='schema_version'",
                    [],
                    |r| r.get::<_, String>(0),
                )?)
            })
            .unwrap();
        assert_eq!(v, MIGRATIONS.len().to_string());
    }

    #[test]
    fn fts5_tables_exist() {
        let db = Db::open_in_memory().unwrap();
        db.with_conn(|c| {
            let n: i64 = c.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name IN
                 ('ecdict_fts','cedict_fts','ecdict','cedict','cache_entry','wordbook','history')",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(n, 7);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn meta_roundtrip() {
        let db = Db::open_in_memory().unwrap();
        let meta = MetaRepo::new(Arc::new(db));
        assert_eq!(meta.get("nope").unwrap(), None);
        meta.set("k", "v1").unwrap();
        meta.set("k", "v2").unwrap();
        assert_eq!(meta.get("k").unwrap().as_deref(), Some("v2"));
    }

    #[test]
    fn quota_increments_and_resets_across_days() {
        let db = Db::open_in_memory().unwrap();
        let meta = MetaRepo::new(Arc::new(db));
        assert_eq!(meta.quota("2026-10-25").unwrap().1, 0);
        assert_eq!(meta.quota_incr("2026-10-25").unwrap(), 1);
        assert_eq!(meta.quota_incr("2026-10-25").unwrap(), 2);
        assert_eq!(meta.quota("2026-10-25").unwrap().1, 2);
        // 次日：自动重置
        assert_eq!(meta.quota("2026-10-26").unwrap().1, 0);
        assert_eq!(meta.quota_incr("2026-10-26").unwrap(), 1);
        // 手动重置
        meta.quota_reset().unwrap();
        assert_eq!(meta.quota("2026-10-26").unwrap().1, 0);
    }

    #[test]
    fn open_creates_file_db() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/dir/lexi.sqlite");
        let db = Db::open(&path).unwrap();
        db.with_conn(|c| {
            let n: i64 = c.query_row("SELECT COUNT(*) FROM meta", [], |r| r.get(0))?;
            assert!(n >= 1);
            Ok(())
        })
        .unwrap();
        assert!(path.exists());
    }
}
