//! MW 结果缓存仓储（cache_entry 表）。
//!
//! 语义（永不清除策略）：`ttl_days` 是**刷新周期**而非保留期——
//! 到期后 `get` 返回 None（查询链重新走 MW 并 `put` 覆盖同一行，
//! hit_count 保留），过期行**不删除**；`sweep` 仅在超过 `max_rows`
//! 时按 hit_count/fetched_at 冷淘汰。一个单词的释义几乎不会变化，
//! 缓存只刷新、不清理。

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
    /// 超过 max_rows 冷淘汰的行数（TTL 不再触发删除）
    pub evicted: u64,
}

#[async_trait]
pub trait CacheRepo: Send + Sync {
    /// 刷新周期内命中返回条目并递增 hit_count；到期返回 None 触发
    /// 重新拉取（过期行保留，刷新时被 put 覆盖；缓存永不清除）。
    async fn get(&self, key: &str, now: i64) -> anyhow::Result<Option<CachedEntry>>;

    /// 只读查看（不递增 hit_count）；用于单词本摘要聚合。
    async fn peek(&self, key: &str, now: i64) -> anyhow::Result<Option<CachedEntry>>;

    /// 写入/覆盖缓存（保留既有 hit_count）。
    async fn put(&self, key: &str, mw: &MwEntry, ttl_days: u32, now: i64) -> anyhow::Result<()>;

    /// 清理超限行（仅 max_rows 冷淘汰；TTL 不删行）。
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
                                hom, meta_json, altprs_json, ins_json, uros_json, cxs_json,
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
                                r.get::<_, Option<String>>(11)?,
                                r.get::<_, Option<String>>(12)?,
                                r.get::<_, Option<String>>(13)?,
                                r.get::<_, Option<String>>(14)?,
                                r.get::<_, Option<String>>(15)?,
                                r.get::<_, Option<String>>(16)?,
                                r.get::<_, i64>(17)?,
                                r.get::<_, i64>(18)?,
                                r.get::<_, i32>(19)?,
                            ))
                        },
                    )
                    .optional()?;

                let Some((raw_json, headword, phonetic, pos, short_defs, senses_json,
                          audio_codes, etymology, date_text, stems, labels,
                          hom, meta_json, altprs_json, ins_json, uros_json, cxs_json,
                          fetched_at, ttl_days, hit_count)) = row else {
                    return Ok(None);
                };

                // 刷新周期判定：到期返回 None（走 MW 重取覆盖），
                // 行本身保留 —— 缓存永不清除
                if fetched_at + ttl_days * 86_400 < now {
                    return Ok(None);
                }

                c.execute(
                    "UPDATE cache_entry SET hit_count = hit_count + 1 WHERE word_key = ?1",
                    [&key],
                )?;

                // 首选：raw_json 全 entry 重放 + 重新合并（含短语分离、
                // 免列迁移）；raw 损坏时回退提取列
                let mw = replay_or_columns(&raw_json, headword, phonetic, pos, short_defs,
                    senses_json, audio_codes, etymology, date_text, stems, labels,
                    hom, meta_json, altprs_json, ins_json, uros_json, cxs_json);
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

    /// 只读命中（TTL 内）：与 [`CacheRepo::get`] 同构但不递增 hit_count。
    async fn peek(&self, key: &str, now: i64) -> anyhow::Result<Option<CachedEntry>> {
        let db = self.db.clone();
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            db.with_conn(|c| {
                let row = c
                    .query_row(
                        "SELECT raw_json, headword, phonetic, pos, short_defs, senses_json,
                                audio_codes, etymology, date_text, stems, labels,
                                hom, meta_json, altprs_json, ins_json, uros_json, cxs_json,
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
                                r.get::<_, Option<String>>(11)?,
                                r.get::<_, Option<String>>(12)?,
                                r.get::<_, Option<String>>(13)?,
                                r.get::<_, Option<String>>(14)?,
                                r.get::<_, Option<String>>(15)?,
                                r.get::<_, Option<String>>(16)?,
                                r.get::<_, i64>(17)?,
                                r.get::<_, i64>(18)?,
                                r.get::<_, i32>(19)?,
                            ))
                        },
                    )
                    .optional()?;

                let Some((raw_json, headword, phonetic, pos, short_defs, senses_json,
                          audio_codes, etymology, date_text, stems, labels,
                          hom, meta_json, altprs_json, ins_json, uros_json, cxs_json,
                          fetched_at, ttl_days, hit_count)) = row else {
                    return Ok(None);
                };
                if fetched_at + ttl_days * 86_400 < now {
                    return Ok(None);
                }
                let mw = replay_or_columns(&raw_json, headword, phonetic, pos, short_defs,
                    senses_json, audio_codes, etymology, date_text, stems, labels,
                    hom, meta_json, altprs_json, ins_json, uros_json, cxs_json);
                Ok(Some(CachedEntry {
                    mw,
                    fetched_at,
                    hit_count: hit_count.max(0) as u32,
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
                        hom, meta_json, altprs_json, ins_json, uros_json, cxs_json,
                        fetched_at, hit_count, ttl_days)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,0,?20)
                     ON CONFLICT(word_key) DO UPDATE SET
                       raw_json=excluded.raw_json, headword=excluded.headword,
                       phonetic=excluded.phonetic, pos=excluded.pos,
                       short_defs=excluded.short_defs, senses_json=excluded.senses_json,
                       audio_codes=excluded.audio_codes, etymology=excluded.etymology,
                       date_text=excluded.date_text, stems=excluded.stems,
                       labels=excluded.labels, hom=excluded.hom,
                       meta_json=excluded.meta_json, altprs_json=excluded.altprs_json,
                       ins_json=excluded.ins_json, uros_json=excluded.uros_json,
                       cxs_json=excluded.cxs_json, fetched_at=excluded.fetched_at,
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
                        mw.hom,
                        mw.meta.as_ref().map(|m| serde_json::to_string(m)).transpose()?,
                        serde_json::to_string(&mw.alt_phonetics)?,
                        serde_json::to_string(&mw.inflections)?,
                        serde_json::to_string(&mw.run_ons)?,
                        serde_json::to_string(&mw.cross_refs)?,
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
                let _ = now; // TTL 不删行；仅超限冷淘汰
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
                Ok(SweepReport { evicted })
            })
        })
        .await
        .map_err(|e| anyhow::anyhow!("join: {e}"))?
    }
}

fn decode_json_array(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

/// raw_json 全 entry 重放 + 重新合并；raw 为空/损坏时用提取列拼装兜底。
#[allow(clippy::too_many_arguments)]
fn replay_or_columns(
    raw_json: &str,
    headword: Option<String>,
    phonetic: Option<String>,
    pos: Option<String>,
    short_defs: Option<String>,
    senses_json: Option<String>,
    audio_codes: Option<String>,
    etymology: Option<String>,
    date_text: Option<String>,
    stems: Option<String>,
    labels: Option<String>,
    hom: Option<String>,
    meta_json: Option<String>,
    altprs_json: Option<String>,
    ins_json: Option<String>,
    uros_json: Option<String>,
    cxs_json: Option<String>,
) -> MwEntry {
    let entries = crate::infra::mw::parse_cached_entries(raw_json);
    if !entries.is_empty() {
        return crate::domain::merger::combine_entries(entries);
    }
    let mut entry = MwEntry {
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
        hom: hom.filter(|s| !s.is_empty()),
        meta: meta_json.as_deref().and_then(|s| serde_json::from_str(s).ok()),
        alt_phonetics: decode_json_array(&altprs_json.unwrap_or_default()),
        inflections: decode_json_or_default(&ins_json),
        run_ons: decode_json_or_default(&uros_json),
        cross_refs: decode_json_or_default(&cxs_json),
        // 列兜底路径（raw 解析失败才会走到）：新增字段无列可回填，
        // 留空即可——正常缓存走 raw 重放获得全部新字段
        ..MwEntry::default()
    };
    entry.raw_json = raw_json.to_string();
    entry
}

fn decode_json_or_default<T: serde::de::DeserializeOwned + Default>(s: &Option<String>) -> T {
    s.as_deref()
        .and_then(|json| serde_json::from_str(json).ok())
        .unwrap_or_default()
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
                ..Default::default()
            }],
            audio_codes: vec!["test0001".into()],
            etymology: None,
            date: Some("1900".into()),
            stems: vec![hw.into()],
            labels: vec![],
            raw_json: "[{\"meta\":{\"id\":\"test\"}}]".into(),
            ..Default::default()
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
    async fn ttl_expiry_keeps_row_but_returns_none() {
        let r = repo();
        r.put("test", &entry("test"), 30, 1000).await.unwrap();
        // 31 天后：触发重取（None），但行保留（缓存永不清除）
        let c = r.get("test", 1000 + 31 * 86_400).await.unwrap();
        assert!(c.is_none());
        let rows: i64 = r
            .db
            .with_conn(|c| Ok(c.query_row("SELECT COUNT(*) FROM cache_entry", [], |x| x.get(0))?))
            .unwrap();
        assert_eq!(rows, 1);
        // 刷新周期内重取覆盖后可再次命中
        r.put("test", &entry("test"), 30, 1000 + 31 * 86_400)
            .await
            .unwrap();
        let c = r.get("test", 1000 + 31 * 86_400 + 10).await.unwrap();
        assert!(c.is_some());
        // sweep 永不因 TTL 删除
        let report = r.sweep(50_000, 1000 + 300 * 86_400).await.unwrap();
        assert_eq!(report.evicted, 0);
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

    #[tokio::test]
    async fn rich_fields_roundtrip() {
        let r = repo();
        let mut e = entry("voluminous");
        e.hom = Some("1".into());
        e.meta = Some(crate::domain::types::MwMeta {
            id: "voluminous".into(),
            src: "collegiate".into(),
            section: "alpha".into(),
            sort: "220130400".into(),
            stems: vec!["voluminous".into(), "voluminously".into()],
            offensive: false,
        });
        e.run_ons = vec![crate::domain::types::RunOn {
            ure: "voluminously".into(),
            fl: "adverb".into(),
            phonetic: String::new(),
            audio_code: String::new(),
            ..Default::default()
        }];
        e.inflections = vec![crate::domain::types::Inflection {
            display: "voluminously".into(),
            ..Default::default()
        }];
        e.cross_refs = vec![crate::domain::types::CrossRef {
            label: "synonym of".into(),
            refs: vec!["ample".into()],
        }];
        e.senses[0].subsenses = vec![crate::domain::types::SubSense {
            label: "also".into(),
            text: "full".into(),
            examples: vec![],
        }];
        r.put("voluminous", &e, 30, 1000).await.unwrap();
        let got = r.get("voluminous", 1001).await.unwrap().unwrap();
        assert_eq!(got.mw.hom.as_deref(), Some("1"));
        let meta = got.mw.meta.expect("meta");
        assert_eq!(meta.src, "collegiate");
        assert_eq!(meta.stems.len(), 2);
        assert_eq!(got.mw.run_ons.len(), 1);
        assert_eq!(got.mw.run_ons[0].ure, "voluminously");
        assert_eq!(got.mw.inflections[0].display, "voluminously");
        assert_eq!(got.mw.cross_refs[0].refs, vec!["ample"]);
        assert_eq!(got.mw.senses[0].subsenses[0].label, "also");
        assert_eq!(got.mw.senses[0].subsenses[0].text, "full");
    }

    /// 0004 之前的旧行（富化列 NULL）读取回落空值。
    #[tokio::test]
    async fn legacy_row_without_rich_columns_parses() {
        let r = repo();
        let e = entry("old");
        r.db.with_conn(|c| {
            c.execute(
                "INSERT INTO cache_entry
                   (word_key, raw_json, headword, phonetic, pos, short_defs, senses_json,
                    audio_codes, etymology, date_text, stems, labels,
                    fetched_at, hit_count, ttl_days)
                 VALUES ('old',?1,?2,?3,?4,?5,?6,?7,NULL,?8,?9,'[]',1000,0,30)",
                rusqlite::params![
                    e.raw_json,
                    e.headword,
                    e.phonetic,
                    e.pos,
                    serde_json::to_string(&e.short_defs)?,
                    serde_json::to_string(&e.senses)?,
                    serde_json::to_string(&e.audio_codes)?,
                    e.date,
                    serde_json::to_string(&e.stems)?,
                ],
            )?;
            Ok(())
        })
        .unwrap();
        let got = r.get("old", 1001).await.unwrap().unwrap();
        assert_eq!(got.mw.headword, "old");
        assert_eq!(got.mw.hom, None);
        assert!(got.mw.meta.is_none());
        assert!(got.mw.run_ons.is_empty());
    }
}
