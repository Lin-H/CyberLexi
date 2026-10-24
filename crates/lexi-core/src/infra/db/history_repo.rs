//! 查询历史（history 表）：只保留最近 50 条；下拉展示按时间倒序去重。

use super::DbHandle;

#[derive(Clone)]
pub struct HistoryRepo {
    db: DbHandle,
    max_rows: i64,
}

impl HistoryRepo {
    pub fn new(db: DbHandle) -> Self {
        HistoryRepo { db, max_rows: 50 }
    }

    /// 记录一次查询并截断到最近 max_rows 条。
    pub fn push(&self, query_text: &str, now: i64) -> anyhow::Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO history (query_text, queried_at) VALUES (?1, ?2)",
                rusqlite::params![query_text, now],
            )?;
            c.execute(
                "DELETE FROM history WHERE id NOT IN (
                   SELECT id FROM history ORDER BY queried_at DESC, id DESC LIMIT ?1)",
                [self.max_rows],
            )?;
            Ok(())
        })
    }

    /// 最近历史（时间倒序，应用层去重，最多 50 条）。
    pub fn recent(&self) -> anyhow::Result<Vec<String>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT query_text FROM history ORDER BY queried_at DESC, id DESC",
            )?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut seen = std::collections::HashSet::new();
            let mut out = Vec::new();
            for text in rows {
                let text = text?;
                if seen.insert(text.to_lowercase()) {
                    out.push(text);
                    if out.len() >= self.max_rows as usize {
                        break;
                    }
                }
            }
            Ok(out)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn repo() -> HistoryRepo {
        HistoryRepo::new(DbHandle::from(Arc::new(
            crate::infra::db::Db::open_in_memory().unwrap(),
        )))
    }

    #[test]
    fn push_truncates_to_50() {
        let r = repo();
        for i in 0..60 {
            r.push(&format!("w{i}"), i).unwrap();
        }
        // 表内只剩 50 行
        let all = r.db.with_conn(|c| {
            let n: i64 = c.query_row("SELECT COUNT(*) FROM history", [], |x| x.get(0))?;
            Ok(n)
        })
        .unwrap();
        assert_eq!(all, 50);
        // 最新在前
        let recent = r.recent().unwrap();
        assert_eq!(recent[0], "w59");
        assert_eq!(recent.len(), 50);
    }

    #[test]
    fn recent_dedups_case_insensitive() {
        let r = repo();
        r.push("Hello", 1).unwrap();
        r.push("hello", 2).unwrap();
        r.push("world", 3).unwrap();
        let recent = r.recent().unwrap();
        assert_eq!(recent, vec!["world", "hello"]);
    }
}
