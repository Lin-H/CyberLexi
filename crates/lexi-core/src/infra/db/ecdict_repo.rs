//! ECDICT 离线词库仓储（ecdict 表 + ecdict_fts）。

use rusqlite::params;

use super::DbHandle;
use crate::domain::types::EcdictRecord;

pub struct EcdictRepo {
    db: DbHandle,
}

impl EcdictRepo {
    pub fn new(db: DbHandle) -> Self {
        EcdictRepo { db }
    }

    pub fn count(&self) -> anyhow::Result<i64> {
        self.db.with_conn(|c| Ok(c.query_row("SELECT COUNT(*) FROM ecdict", [], |r| r.get(0))?))
    }

    /// 随机高频词（frq 排名前 `pool` 内均匀抽取；3~12 字母）。
    /// 「随机单词」按钮数据源；无可用词返回 None。
    pub fn random_word(&self, pool: i64) -> anyhow::Result<Option<String>> {
        self.db.with_conn(|c| {
            let word: Option<String> = c
                .query_row(
                    "SELECT word FROM ecdict
                     WHERE frq > 0 AND length(word) BETWEEN 3 AND 12
                     ORDER BY frq ASC
                     LIMIT 1 OFFSET abs(random()) % ?1",
                    [pool],
                    |r| r.get(0),
                )
                .optional()?;
            Ok(word)
        })
    }

    /// 词精确匹配（word 列 = key）。ECDICT word 本身为小写。
    pub fn lookup(&self, word_key: &str) -> anyhow::Result<Option<EcdictRecord>> {
        self.db.with_conn(|c| {
            let row = c
                .query_row(
                    "SELECT word, phonetic, pos, translation, collins, oxford, tag, bnc, frq, exchange
                     FROM ecdict WHERE word = ?1",
                    [word_key],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, Option<String>>(2)?,
                            r.get::<_, Option<String>>(3)?,
                            r.get::<_, i32>(4)?,
                            r.get::<_, i32>(5)?,
                            r.get::<_, Option<String>>(6)?,
                            r.get::<_, i32>(7)?,
                            r.get::<_, i32>(8)?,
                            r.get::<_, Option<String>>(9)?,
                        ))
                    },
                )
                .optional()?;
            Ok(row.map(|(word, phonetic, pos, translation, collins, oxford, tag, bnc, frq, exchange)| {
                EcdictRecord {
                    word,
                    phonetic: phonetic.unwrap_or_default(),
                    pos: pos.unwrap_or_default(),
                    translation: translation.unwrap_or_default(),
                    collins,
                    oxford,
                    tag: tag.unwrap_or_default(),
                    bnc,
                    frq,
                    exchange: exchange.unwrap_or_default(),
                }
            }))
        })
    }

    /// 批量插入（importer 分批调用；每批一个事务）。
    pub fn insert_batch(&self, records: &[EcdictRecord]) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            let tx = c.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO ecdict
                       (word, phonetic, pos, translation, collins, oxford, tag, bnc, frq, exchange)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                )?;
                for r in records {
                    stmt.execute(params![
                        r.word,
                        r.phonetic,
                        r.pos,
                        r.translation,
                        r.collins,
                        r.oxford,
                        r.tag,
                        r.bnc,
                        r.frq,
                        r.exchange,
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// 重导前清空（连同 FTS 由 content 表触发器联动删除）。
    pub fn clear(&self) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM ecdict", [])?;
            Ok(())
        })
    }

    /// 中文链兜底：translation 全文匹配（FTS5 trigram，>=3 字符）；
    /// 短查询回退 LIKE 子串。返回 (word, translation 首行)。
    pub fn search_translation(&self, q: &str, limit: u32) -> anyhow::Result<Vec<(String, String)>> {
        let q = q.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        self.db.with_conn(|c| {
            let (sql, bind): (&str, String) = if q.chars().count() >= 3 {
                (
                    "SELECT e.word, e.translation FROM ecdict e
                     JOIN ecdict_fts f ON f.rowid = e.rowid
                     WHERE ecdict_fts MATCH ?1
                     ORDER BY e.frq DESC LIMIT ?2",
                    format!("\"{q}\""),
                )
            } else {
                (
                    "SELECT word, translation FROM ecdict
                     WHERE translation LIKE ?1 ESCAPE '\\'
                     ORDER BY frq DESC LIMIT ?2",
                    format!("%{}%", escape_like(q)),
                )
            };
            let mut stmt = c.prepare(sql)?;
            let rows = stmt.query_map(rusqlite::params![bind, limit], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                ))
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
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

    fn repo() -> EcdictRepo {
        EcdictRepo::new(DbHandle::from(Arc::new(
            crate::infra::db::Db::open_in_memory().unwrap(),
        )))
    }

    fn rec(word: &str, translation: &str, frq: i32) -> EcdictRecord {
        EcdictRecord {
            word: word.into(),
            phonetic: "f".into(),
            pos: "n".into(),
            translation: translation.into(),
            collins: 1,
            oxford: 1,
            tag: "cet4".into(),
            bnc: 1,
            frq,
            exchange: String::new(),
        }
    }

    #[test]
    fn insert_and_exact_lookup() {
        let r = repo();
        r.insert_batch(&[
            rec("apple", "n. 苹果\\nn. 苹果树", 1000),
            rec("banana", "n. 香蕉", 800),
        ])
        .unwrap();
        assert_eq!(r.count().unwrap(), 2);
        let got = r.lookup("apple").unwrap().unwrap();
        assert_eq!(got.word, "apple");
        assert!(got.translation.contains("苹果"));
        assert!(r.lookup("nope").unwrap().is_none());
    }

    #[test]
    fn translation_fts_and_like_fallback() {
        let r = repo();
        r.insert_batch(&[
            rec("apple", "n. 苹果\\nn. 一种水果", 100),
            rec("pineapple", "n. 菠萝；凤梨", 50),
            rec("juice", "n. 果汁", 80),
        ])
        .unwrap();
        // 3+ 字符走 FTS trigram：子串「一种水果」命中
        let hits = r.search_translation("一种水果", 20).unwrap();
        assert!(hits.iter().any(|(w, _)| w == "apple"));
        // 短查询（<3 字符）LIKE 回退
        let hits = r.search_translation("苹果", 20).unwrap();
        assert!(hits.iter().any(|(w, _)| w == "apple"));
        let hits = r.search_translation("菠萝", 20).unwrap();
        assert!(hits.iter().any(|(w, _)| w == "pineapple"));
        // 无命中
        assert!(r.search_translation("不存在词xyz", 20).unwrap().is_empty());
    }

    #[test]
    fn clear_for_reimport() {
        let r = repo();
        r.insert_batch(&[rec("a", "甲", 1)]).unwrap();
        r.clear().unwrap();
        assert_eq!(r.count().unwrap(), 0);
        assert!(r.search_translation("甲", 5).unwrap().is_empty());
    }

    #[test]
    fn random_word_picks_from_pool() {
        let r = repo();
        assert!(r.random_word(100).unwrap().is_none(), "空库无随机词");
        for i in 0..10 {
            r.insert_batch(&[rec(&format!("word{i:02}"), "词", 10 - i)]).unwrap();
        }
        for _ in 0..10 {
            let w = r.random_word(10).unwrap().expect("有词");
            assert!(w.starts_with("word"));
        }
    }
}
