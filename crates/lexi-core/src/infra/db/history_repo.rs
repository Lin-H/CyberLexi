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

    /// 侧边栏统计：今日去重查词数 + 连续打卡天数（UTC 日界，与配额一致）。
    /// streak：从今天（今天无记录则从昨天）起往回数连续有查询的天数。
    pub fn stats(&self, now: i64) -> anyhow::Result<(u32, u32)> {
        self.db.with_conn(|c| {
            let today_count: i64 = c.query_row(
                "SELECT COUNT(DISTINCT lower(query_text)) FROM history
                 WHERE date(queried_at, 'unixepoch') = date(?1, 'unixepoch')",
                [now],
                |r| r.get(0),
            )?;
            let mut stmt =
                c.prepare("SELECT DISTINCT date(queried_at, 'unixepoch') FROM history ORDER BY 1")?;
            let days: Vec<String> = stmt
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<_, _>>()?;
            if days.is_empty() {
                return Ok((today_count.max(0) as u32, 0));
            }
            let today = day_number(now);

            // days 升序去重；转成日序号后从末端连续递减计数
            let nums: Vec<i64> = days
                .iter()
                .map(|d| {
                    // 'YYYY-MM-DD' → 粗略日序号（年*372 + 月*31 + 日）
                    let mut it = d.split('-');
                    let y: i64 = it.next().unwrap_or("0").parse().unwrap_or(0);
                    let m: i64 = it.next().unwrap_or("0").parse().unwrap_or(0);
                    let d: i64 = it.next().unwrap_or("0").parse().unwrap_or(0);
                    y * 372 + m * 31 + d
                })
                .collect();
            let last = *nums.last().unwrap();
            // 今日无记录时允许从昨天起算（打卡不断更真实的定义）
            let anchor = if last == today { today } else { today - 1 };
            let mut streak = 0u32;
            let mut expect = anchor;
            for n in nums.iter().rev() {
                if *n == expect {
                    streak += 1;
                    expect -= 1;
                } else if *n < expect {
                    break;
                }
            }
            Ok((today_count.max(0) as u32, streak))
        })
    }
}

/// UTC 日序号（粗略，仅供连续性比较；与 date() 输出一致即可）。
fn day_number(now: i64) -> i64 {
    let days = now.div_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    y * 372 + m * 31 + d
}

/// days since epoch → (year, month, day)（Howard Hinnant civil_from_days）。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
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

    #[test]
    fn stats_today_count_and_streak() {
        let r = repo();
        // 2026-10-25 当天（取整点）与前三天的记录
        let day = |y: i64, m: i64, d: i64, h: i64| {
            // 简易 epoch：用格里历日序号（civil_from_days 的逆）×86400
            days_from_civil(y, m, d) * 86_400 + h * 3600
        };
        r.push("apple", day(2026, 10, 22, 1)).unwrap();
        r.push("banana", day(2026, 10, 23, 1)).unwrap();
        r.push("cherry", day(2026, 10, 24, 1)).unwrap();
        r.push("apple", day(2026, 10, 25, 2)).unwrap();
        r.push("durian", day(2026, 10, 25, 3)).unwrap();
        let (today, streak) = r.stats(day(2026, 10, 25, 12)).unwrap();
        assert_eq!(today, 2, "今日去重 2 词");
        assert_eq!(streak, 4, "22→25 连续 4 天");

        // 今天还没查：streak 允许从昨天起算
        let (today, streak) = r.stats(day(2026, 10, 26, 0)).unwrap();
        assert_eq!(today, 0);
        assert_eq!(streak, 4, "26 日 0 点（UTC 日界未过 26 全天）— 22~25 连 4 天");

        // 断更两天后
        let (today, streak) = r.stats(day(2026, 10, 28, 12)).unwrap();
        assert_eq!(today, 0);
        assert_eq!(streak, 0);
    }

    /// civil_from_days 逆运算（仅测试用）。
    fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y.rem_euclid(400);
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }
}
