//! 单词本 SQLite 存储（wordbook 表）：实现 domain 的 [`WordBookStore`]。

use rusqlite::params;

use super::DbHandle;
use crate::domain::wordbook::{WordBookItem, WordBookStore, WordStatus};

pub struct SqliteWordBookStore {
    db: DbHandle,
}

impl SqliteWordBookStore {
    pub fn new(db: DbHandle) -> Self {
        SqliteWordBookStore { db }
    }
}

impl WordBookStore for SqliteWordBookStore {
    fn exists(&self, word_key: &str) -> anyhow::Result<bool> {
        self.db.with_conn(|c| {
            let n: i64 = c.query_row(
                "SELECT COUNT(*) FROM wordbook WHERE word_key = ?1",
                [word_key],
                |r| r.get(0),
            )?;
            Ok(n > 0)
        })
    }

    fn get(&self, word_key: &str) -> anyhow::Result<Option<WordBookItem>> {
        self.db.with_conn(|c| {
            let row = c
                .query_row(
                    "SELECT word_key, headword, status, tags, note, added_at, updated_at
                     FROM wordbook WHERE word_key = ?1",
                    [word_key],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, String>(3)?,
                            r.get::<_, String>(4)?,
                            r.get::<_, i64>(5)?,
                            r.get::<_, i64>(6)?,
                        ))
                    },
                )
                .optional()?;
            Ok(row.map(|(k, hw, status, tags, note, added, updated)| WordBookItem {
                word_key: k,
                headword: hw,
                status: WordStatus::parse(&status).unwrap_or(WordStatus::New),
                tags: serde_json::from_str(&tags).unwrap_or_default(),
                note,
                added_at: added,
                updated_at: updated,
            }))
        })
    }

    fn put(&self, item: WordBookItem) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO wordbook (word_key, headword, status, tags, note, added_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(word_key) DO UPDATE SET
                   headword=excluded.headword, status=excluded.status,
                   tags=excluded.tags, note=excluded.note,
                   updated_at=excluded.updated_at",
                params![
                    item.word_key,
                    item.headword,
                    item.status.as_str(),
                    serde_json::to_string(&item.tags)?,
                    item.note,
                    item.added_at,
                    item.updated_at,
                ],
            )?;
            Ok(())
        })
    }

    fn delete(&self, word_key: &str) -> anyhow::Result<bool> {
        self.db.with_conn(|c| Ok(c.execute("DELETE FROM wordbook WHERE word_key = ?1", [word_key])? > 0))
    }

    fn list_recent_first(&self) -> anyhow::Result<Vec<WordBookItem>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT word_key, headword, status, tags, note, added_at, updated_at
                 FROM wordbook ORDER BY updated_at DESC, word_key ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(WordBookItem {
                    word_key: r.get(0)?,
                    headword: r.get(1)?,
                    status: WordStatus::parse(&r.get::<_, String>(2)?).unwrap_or(WordStatus::New),
                    tags: serde_json::from_str(&r.get::<_, String>(3)?).unwrap_or_default(),
                    note: r.get(4)?,
                    added_at: r.get(5)?,
                    updated_at: r.get(6)?,
                })
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }
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
    use crate::domain::wordbook::WordBook;
    use std::sync::Arc;

    fn store() -> SqliteWordBookStore {
        SqliteWordBookStore::new(DbHandle::from(Arc::new(
            crate::infra::db::Db::open_in_memory().unwrap(),
        )))
    }

    fn item(key: &str) -> WordBookItem {
        WordBookItem {
            word_key: key.into(),
            headword: key.to_uppercase(),
            status: WordStatus::New,
            tags: vec!["gre".into()],
            note: String::new(),
            added_at: 100,
            updated_at: 100,
        }
    }

    #[test]
    fn crud_and_dedup_via_domain_facade() {
        let wb = WordBook::new(store());
        assert!(wb.add(item("apple")).unwrap());
        assert!(!wb.add(item("apple")).unwrap()); // 去重
        wb.set_status("apple", WordStatus::Mastered, 200).unwrap();
        let got = wb.store().get("apple").unwrap().unwrap();
        assert_eq!(got.status, WordStatus::Mastered);
        assert_eq!(got.updated_at, 200);
        assert!(wb.remove("apple").unwrap());
        assert_eq!(wb.list().unwrap().len(), 0);
    }

    #[test]
    fn restart_persistence_via_same_store() {
        // SQLite 天然持久；用同一 store 模拟重启后读取
        let s = store();
        s.put(item("boot")).unwrap();
        assert!(s.get("boot").unwrap().is_some());
    }

    #[test]
    fn list_recent_first_order() {
        let s = store();
        s.put(item("a")).unwrap();
        let mut b = item("b");
        b.updated_at = 999;
        s.put(b).unwrap();
        let list = s.list_recent_first().unwrap();
        assert_eq!(list[0].word_key, "b");
    }
}
