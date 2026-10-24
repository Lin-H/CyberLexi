//! MW 结果缓存仓储（cache_entry 表）。
//!
//! 语义：命中且未过 TTL → 返回并 `hit_count++`（第二次查询同一词
//! 零网络请求，由 orchestrator 保证）；`sweep` 执行 TTL 过期删除 +
//! 超 `max_rows` 行的冷淘汰（hit_count ASC, fetched_at ASC）。

use async_trait::async_trait;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::DbHandle;
use crate::domain::types::{MwEntry, Sense};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    pub mw: MwEntry,
    pub fetched_at: i64,
    pub hit_count: u32,
}

#[derive(Debug, Default, PartialEq)]
pub struct SweepReport {
    pub ttl_expired: u64,
    pub evicted: u64,
}

#[async_trait]
pub trait CacheRepo: Send + Sync {
    /// TTL 内命中返回条目并递增 hit_count；未命中/过期返回 None（过期行顺带删除）。
    async fn get(&self, key: &str, now: i64) -> anyhow::Result<Option<CachedEntry>>;

    /// 写入/覆盖缓存（保留既有 hit_count）。
    async fn put(&self, key: &str, mw: &MwEntry, ttl_days: u32, now: i64) -> anyhow::Result<()>;

    /// 清理过期与超限行（TTL 用每行 put 时写入的 ttl_days 列判定）。
    async fn sweep(&self, max_rows: u32, now: i64) -> anyhow::Result<SweepReport>;
}

pub struct SqliteCacheRepo {
    db: DbHandle,
}

impl SqliteCacheRepo {
    pub fn new(db: DbHandle) -> Self {
        SqliteCacheRepo { db }
    }
}

#[async_trait]
impl CacheRepo for SqliteCacheRepo {
    async fn get(&self, key: &str, now: i64) -> anyhow::Result<Option<CachedEntry>> {
        let db = self.db.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let row = c
                    .query_row(
                        "SELECT raw_json, headword, phonetic, pos, short_defs, senses_json,
                                audio_codes, etymology, date_text, stems, labels,
                                fetched_at, ttl_days, hit_count
                         FROM cache_entry WHERE word_key = ?1",
                        [&key],
                        |r| {
                            Ok((
                                r.get::<_, String>(0)?,
                                r.get::<_, Option<String>>(1)?,
                                r.get::<_, Option<String>>(2)?,
                                r.get::<_, Option<String>>(3)?,
                                r.get::<_, Option<String>>(4)?,
                                r.get::<_, Option<String>>(5)?,
                                r.get::<_, Option<String>>(6)?,
                                r.get::<_, Option<String>>(7)?,
                                r.get::<_, Option<String>>(8)?,
                                r.get::<_, Option<String>>(9)?,
                                r.get::<_, Option<String>>(10)?,
                                r.get::<_, i64>(11)?,
                                r.get::<_, i64>(12)?,
                                r.get::<_, i32>(13)?,
                            ))
                        },
                    )
                    .optional()?;

                let Some((raw_json, headword, phonetic, pos, short_defs, senses_json,
                          audio_codes, etymology, date_text, stems, labels,
                          fetched_at, ttl_days, hit_count)) = row else {
                    return Ok(None);
                };

                // TTL 判定（秒）
                if fetched_at + ttl_days * 86_400 < now {
                    c.execute("DELETE FROM cache_entry WHERE word_key = ?1", [&key])?;
                    return Ok(None);
                }

                c.execute(
                    "UPDATE cache_entry SET hit_count = hit_count + 1 WHERE word_key = ?1",
                    [&key],
                )?;

                let mw = MwEntry {
                    headword: headword.unwrap_or_default(),
                    phonetic: phonetic.unwrap_or_default(),
                    pos: pos.unwrap_or_default(),
                    short_defs: decode_json_array(&short_defs.unwrap_or_default()),
                    senses: serde_json::from_str(&senses_json.unwrap_or_else(|| "[]".into()))
                        .unwrap_or_default(),
                    audio_codes: decode_json_array(&audio_codes.unwrap_or_default()),
                    etymology,
                    date: date_text,
                    stems: decode_json_array(&stems.unwrap_or_default()),
                    labels: decode_json_array(&labels.unwrap_or_default()),
                    raw_json,
                };
                Ok(Some(CachedEntry {
                    mw,
                    fetched_at,
                    // 返回递增后的值（UPDATE 已 +1）
                    hit_count: (hit_count.max(0) as u32).saturating_add(1),
                }))
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!("join: {e}"))?
    }

    async fn put(&self, key: &str, mw: &MwEntry, ttl_days: u32, now: i64) -> anyhow::Result<()> {
        let db = self.db.clone();
        let key = key.to_string();
        let mw = mw.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                c.execute(
                    "INSERT INTO cache_entry
                       (word_key, raw_json, headword, phonetic, pos, short_defs, senses_json,
                        audio_codes, etymology, date_text, stems, labels,
                        fetched_at, hit_count, ttl_days)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,0,?14)
                     ON CONFLICT(word_key) DO UPDATE SET
                       raw_json=excluded.raw_json, headword=excluded.headword,
                       phonetic=excluded.phonetic, pos=excluded.pos,
                       short_defs=excluded.short_defs, senses_json=excluded.senses_json,
                       audio_codes=excluded.audio_codes, etymology=excluded.etymology,
                       date_text=excluded.date_text, stems=excluded.stems,
                       labels=excluded.labels, fetched_at=excluded.fetched_at,
                       ttl_days=excluded.ttl_days",
                    params![
                        key,
                        mw.raw_json,
                        mw.headword,
                        mw.phonetic,
                        mw.pos,
                        serde_json::to_string(&mw.short_defs)?,
                        serde_json::to_string(&mw.senses)?,
                        serde_json::to_string(&mw.audio_codes)?,
                        mw.etymology,
                        mw.date,
                        serde_json::to_string(&mw.stems)?,
                        serde_json::to_string(&mw.labels)?,
                        now,
                        ttl_days,
                    ],
                )?;
                Ok(())
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!("join: {e}"))?
    }

    async fn sweep(&self, max_rows: u32, now: i64) -> anyhow::Result<SweepReport> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let ttl_expired = c.execute(
                    "DELETE FROM cache_entry WHERE fetched_at + ttl_days*86400 < ?1",
                    [now],
                )? as u64;
                let total: u32 = c.query_row("SELECT COUNT(*) FROM cache_entry", [], |r| r.get(0))?;
                let evicted = if total > max_rows {
                    c.execute(
                        "DELETE FROM cache_entry WHERE word_key IN (
                           SELECT word_key FROM cache_entry
                           ORDER BY hit_count ASC, fetched_at ASC
                           LIMIT ?1)",
                        [total - max_rows],
                    )? as u64
                } else {
                    0
                };
                Ok(SweepReport { ttl_expired, evicted })
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!("join: {e}"))?
    }
}

fn decode_json_array(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

/// 从缓存 JSON 还原 Sense 列表（Sense 已 derive Serialize/Deserialize）。
#[allow(dead_code)]
fn decode_senses(s: &str) -> Vec<Sense> {
    serde_json::from_str(s).unwrap_or_default()
}

trait OptionalRow<T> {
    fn optional(self) -> anyhow::Result<Option<T>>;
}
impl<T> OptionalRow<T> for rusqlite::Result<T> {
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
    use std::sync::Arc;

    fn entry(hw: &str) -> MwEntry {
        MwEntry {
            headword: hw.into(),
            phonetic: "ˈtest".into(),
            pos: "noun".into(),
            short_defs: vec!["a test".into()],
            senses: vec![Sense {
                sn: Some("1".into()),
                text: "a test".into(),
                examples: vec!["a test entry".into()],
            }],
            audio_codes: vec!["test0001".into()],
            etymology: None,
            date: Some("1900".into()),
            stems: vec![hw.into()],
            labels: vec![],
            raw_json: "[{\"meta\":{\"id\":\"test\"}}]".into(),
        }
    }

    fn repo() -> SqliteCacheRepo {
        let db = DbHandle::from(Arc::new(
            crate::infra::db::Db::open_in_memory().unwrap(),
        ));
        SqliteCacheRepo::new(db)
    }

    #[tokio::test]
    async fn put_get_roundtrip_and_hit_count() {
        let r = repo();
        r.put("test", &entry("test"), 30, 1000).await.unwrap();
        let c = r.get("test", 1001).await.unwrap().unwrap();
        assert_eq!(c.mw.headword, "test");
        assert_eq!(c.mw.short_defs, vec!["a test"]);
        assert_eq!(c.hit_count, 1); // get 时递增
        let c2 = r.get("test", 1002).await.unwrap().unwrap();
        assert_eq!(c2.hit_count, 2);
    }

    #[tokio::test]
    async fn ttl_expiry_deletes_row() {
        let r = repo();
        r.put("test", &entry("test"), 30, 1000).await.unwrap();
        // 31 天后
        let c = r.get("test", 1000 + 31 * 86_400).await.unwrap();
        assert!(c.is_none());
        // 过期行已被删除：sweep 计数为 0
        let report = r.sweep(50_000, 1000 + 31 * 86_400).await.unwrap();
        assert_eq!(report.ttl_expired, 0);
    }

    #[tokio::test]
    async fn ttl_boundary_inclusive() {
        let r = repo();
        r.put("test", &entry("test"), 30, 1000).await.unwrap();
        // 恰好第 30 天仍有效（fetched_at + ttl*86400 == now 不算过期）
        let c = r.get("test", 1000 + 30 * 86_400).await.unwrap();
        assert!(c.is_some());
    }

    #[tokio::test]
    async fn put_overwrites_but_keeps_hit_count() {
        let r = repo();
        r.put("test", &entry("test"), 30, 1000).await.unwrap();
        r.get("test", 1001).await.unwrap().unwrap(); // hit 1
        let mut e2 = entry("test2");
        e2.headword = "test2".into();
        r.put("test", &e2, 30, 2000).await.unwrap();
        let c = r.get("test", 2001).await.unwrap().unwrap();
        assert_eq!(c.mw.headword, "test2");
        assert_eq!(c.hit_count, 2); // 旧 hit 1 + 本次 get 1
    }

    #[tokio::test]
    async fn sweep_evicts_coldest_beyond_limit() {
        let r = repo();
        for i in 0..5 {
            r.put(&format!("w{i}"), &entry("w"), 30, 1000 + i as i64)
                .await
                .unwrap();
        }
        // w0 最旧；给 w1 加热度
        r.get("w1", 1001).await.unwrap().unwrap();
        let report = r.sweep(3, 2000).await.unwrap();
        assert_eq!(report.evicted, 2);
        assert!(r.get("w0", 2001).await.unwrap().is_none());
        // w1 有 hit_count 不被淘汰（hit ASC, fetched ASC → w0, w2 淘汰）
        assert!(r.get("w1", 2001).await.unwrap().is_some());
        assert!(r.get("w2", 2001).await.unwrap().is_none());
        assert!(r.get("w3", 2001).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn missing_key_is_none() {
        let r = repo();
        assert!(r.get("nope", 0).await.unwrap().is_none());
    }
}
