//! CC-CEDICT 仓储（cedict 表 + cedict_fts）。
//!
//! 中文链：simp/trad 精确 → pinyin 前缀（无声调形式）→ english FTS。

use rusqlite::params;

use super::DbHandle;
use crate::domain::types::CedictRecord;

pub struct CedictRepo {
    db: DbHandle,
}

impl CedictRepo {
    pub fn new(db: DbHandle) -> Self {
        CedictRepo { db }
    }

    pub fn count(&self) -> anyhow::Result<i64> {
        self.db.with_conn(|c| Ok(c.query_row("SELECT COUNT(*) FROM cedict", [], |r| r.get(0))?))
    }

    /// 简体/繁体精确匹配。
    pub fn lookup_exact(&self, zh: &str) -> anyhow::Result<Vec<CedictRecord>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT trad, simp, pinyin, pinyin_plain, english FROM cedict
                 WHERE simp = ?1 OR trad = ?1 LIMIT 20",
            )?;
            let rows = stmt.query_map([zh], map_record)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }

    /// 拼音前缀匹配（无声调）：如 "ni h"/"nihao" → 候选。
    pub fn search_pinyin(&self, prefix: &str, limit: u32) -> anyhow::Result<Vec<CedictRecord>> {
        let p = prefix.trim().to_lowercase();
        if p.is_empty() {
            return Ok(Vec::new());
        }
        self.db.with_conn(|c| {
            // 空格分隔形式（"ni h*"）与连写形式（"nih*"）双探；
            // 连写走 REPLACE 表达式（无法用索引，CEDICT 体量可接受）
            let mut stmt = c.prepare(
                "SELECT trad, simp, pinyin, pinyin_plain, english FROM cedict
                 WHERE pinyin_plain LIKE ?1 ESCAPE '\\'
                    OR REPLACE(pinyin_plain, ' ', '') LIKE ?2 ESCAPE '\\'
                 LIMIT ?3",
            )?;
            let sp = format!("{}%", escape_like(&p));
            let joined = format!("{}%", escape_like(&p.replace(' ', "")));
            let rows = stmt.query_map(params![sp, joined, limit], map_record)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }

    /// 英文释义 FTS 匹配（词组前缀查询）。
    pub fn search_english(&self, q: &str, limit: u32) -> anyhow::Result<Vec<CedictRecord>> {
        let q = q.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT c.trad, c.simp, c.pinyin, c.pinyin_plain, c.english
                 FROM cedict c JOIN cedict_fts f ON f.rowid = c.rowid
                 WHERE cedict_fts MATCH ?1 LIMIT ?2",
            )?;
            let match_expr = build_prefix_expr(q);
            let rows = stmt.query_map(params![match_expr, limit], map_record)?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }

    /// 批量插入。
    pub fn insert_batch(&self, records: &[CedictRecord]) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            let tx = c.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR REPLACE INTO cedict (trad, simp, pinyin, pinyin_plain, english)
                     VALUES (?1,?2,?3,?4,?5)",
                )?;
                for r in records {
                    stmt.execute(params![
                        r.traditional,
                        r.simplified,
                        r.pinyin,
                        r.pinyin_plain,
                        r.english,
                    ])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn clear(&self) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM cedict", [])?;
            Ok(())
        })
    }
}

fn map_record(r: &rusqlite::Row<'_>) -> rusqlite::Result<CedictRecord> {
    Ok(CedictRecord {
        traditional: r.get(0)?,
        simplified: r.get(1)?,
        pinyin: r.get(2)?,
        pinyin_plain: r.get(3)?,
        english: r.get(4)?,
    })
}

/// FTS5 前缀词组表达式："hello world" → '"hello"* "world"*'。
fn build_prefix_expr(q: &str) -> String {
    q.split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|w| format!("\"{}\"*", w.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn repo() -> CedictRepo {
        CedictRepo::new(DbHandle::from(Arc::new(
            crate::infra::db::Db::open_in_memory().unwrap(),
        )))
    }

    fn rec(simp: &str, trad: &str, pinyin: &str, english: &str) -> CedictRecord {
        CedictRecord {
            traditional: trad.into(),
            simplified: simp.into(),
            pinyin: pinyin.into(),
            pinyin_plain: pinyin.replace(char::is_numeric, "").trim().to_string(),
            english: english.into(),
        }
    }

    fn seed(r: &CedictRepo) {
        r.insert_batch(&[
            rec("你好", "你好", "ni3 hao3", "/hello/hi/"),
            rec("中国", "中國", "zhong1 guo2", "/China/Middle Kingdom/"),
            rec("电脑", "電腦", "dian4 nao3", "/computer/CL:臺|台[tai2]/"),
            rec("汉语", "漢語", "han4 yu3", "/Chinese language/"),
        ])
        .unwrap();
    }

    #[test]
    fn exact_lookup_simp_and_trad() {
        let r = repo();
        seed(&r);
        let hits = r.lookup_exact("中国").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].pinyin, "zhong1 guo2");
        let hits = r.lookup_exact("電腦").unwrap(); // 繁体
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].simplified, "电脑");
        assert!(r.lookup_exact("不存在").unwrap().is_empty());
    }

    #[test]
    fn pinyin_prefix_search() {
        let r = repo();
        seed(&r);
        // 无声调前缀（空格分隔）
        let hits = r.search_pinyin("ni ha", 10).unwrap();
        assert!(hits.iter().any(|h| h.simplified == "你好"));
        // 连写形式
        let hits = r.search_pinyin("nihao", 10).unwrap();
        assert!(hits.iter().any(|h| h.simplified == "你好"));
        // 声调形式直接匹配 toned 列不适用：输入 "han4" 按 plain 查不到
        //（plain 已去数字），但连写无调 "hanyu" 应命中
        let hits = r.search_pinyin("hanyu", 10).unwrap();
        assert!(hits.iter().any(|h| h.simplified == "汉语"));
    }

    #[test]
    fn english_fts_search() {
        let r = repo();
        seed(&r);
        let hits = r.search_english("computer", 10).unwrap();
        assert!(hits.iter().any(|h| h.simplified == "电脑"));
        let hits = r.search_english("chinese lang", 10).unwrap();
        assert!(hits.iter().any(|h| h.simplified == "汉语"));
        assert!(r.search_english("zzzznotfound", 10).unwrap().is_empty());
    }

    #[test]
    fn glosses_parse() {
        let r = repo();
        seed(&r);
        let hits = r.lookup_exact("你好").unwrap();
        assert_eq!(hits[0].english_glosses(), vec!["hello", "hi"]);
        assert_eq!(hits[0].first_gloss(10), "hello");
    }

    #[test]
    fn count_and_clear() {
        let r = repo();
        seed(&r);
        assert_eq!(r.count().unwrap(), 4);
        r.clear().unwrap();
        assert_eq!(r.count().unwrap(), 0);
    }
}
