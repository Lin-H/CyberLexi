//! CC-CEDICT 导入。官方行格式（空格分隔，非严格 TSV）：
//! `# comment` 或 `trad simp [pin1 yin1] /gloss1/gloss2/`

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::domain::types::CedictRecord;
use crate::infra::db::{CedictRepo, DbHandle};

const BATCH: usize = 5000;

/// 解析一行 CEDICT；注释/ malformed 行返回 None。
pub fn parse_cedict_line(line: &str) -> Option<CedictRecord> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let open = line.find('[')?;
    let close = line.find(']')?;
    if close <= open {
        return None;
    }
    let head = &line[..open];
    let pinyin = &line[open + 1..close];
    let english = line[close + 1..].trim();
    if english.is_empty() || !english.starts_with('/') {
        return None;
    }
    let mut it = head.split_whitespace();
    let traditional = it.next()?.to_string();
    let simplified = it.next()?.to_string();
    Some(CedictRecord {
        traditional,
        simplified,
        pinyin: pinyin.to_string(),
        pinyin_plain: strip_tone_digits(pinyin),
        english: english.to_string(),
    })
}

/// "ni3 hao3" → "ni hao"（去掉声调数字；ü 保留原样——用户以 v 输入的
/// 匹配差异为已知限制，见 README）。
fn strip_tone_digits(pinyin: &str) -> String {
    pinyin
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect::<String>()
        .trim()
        .to_string()
}

/// 导入 CEDICT txt。返回导入行数。
pub async fn import_cedict(
    db: DbHandle,
    txt_path: PathBuf,
    progress: mpsc::UnboundedSender<(f32, String)>,
) -> anyhow::Result<u64> {
    tokio::task::spawn_blocking(move || {
        let _ = progress.send((0.0, "cedict: open".into()));
        let repo = CedictRepo::new(db.clone());
        repo.clear()?;
        let data = std::fs::read(&txt_path)?;
        let file_len = data.len().max(1);
        let text = String::from_utf8_lossy(&data);
        let mut batch: Vec<CedictRecord> = Vec::with_capacity(BATCH);
        let mut total: u64 = 0;
        let mut consumed = 0usize;
        for line in text.lines() {
            consumed += line.len() + 1;
            if let Some(rec) = parse_cedict_line(line) {
                batch.push(rec);
            }
            if batch.len() >= BATCH {
                repo.insert_batch(&batch)?;
                total += batch.len() as u64;
                batch.clear();
                let pct = consumed as f32 / file_len as f32 * 100.0;
                let _ = progress.send((pct.min(99.0), format!("cedict: {total} rows")));
            }
        }
        if !batch.is_empty() {
            repo.insert_batch(&batch)?;
            total += batch.len() as u64;
        }
        let _ = progress.send((100.0, format!("cedict: done {total}")));
        Ok(total)
    })
    .await
    .map_err(|e| anyhow::anyhow!("join: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn parse_line_formats() {
        let r = parse_cedict_line("中國 中国 [Zhong1 guo2] /China/Middle Kingdom/").unwrap();
        assert_eq!(r.traditional, "中國");
        assert_eq!(r.simplified, "中国");
        assert_eq!(r.pinyin, "Zhong1 guo2");
        assert_eq!(r.pinyin_plain, "Zhong guo");
        assert_eq!(r.english, "/China/Middle Kingdom/");

        assert!(parse_cedict_line("# CC-CEDICT comment").is_none());
        assert!(parse_cedict_line("").is_none());
        assert!(parse_cedict_line("malformed no brackets").is_none());
        // 无英文部分
        assert!(parse_cedict_line("中 中 [zhong1] ").is_none());
    }

    #[tokio::test]
    async fn imports_real_format_file() {
        let dir = tempfile::tempdir().unwrap();
        let txt = dir.path().join("cedict.txt");
        std::fs::write(
            &txt,
            "# CC-CEDICT Simple Format\ntaiwan 台湾 [Tai2 wan1] /Taiwan/FORMERLY FORMOSA/\n",
        )
        .unwrap();
        let db = DbHandle::from(Arc::new(crate::infra::db::Db::open_in_memory().unwrap()));
        let (tx, _rx) = mpsc::unbounded_channel();
        let n = import_cedict(db.clone(), txt, tx).await.unwrap();
        assert_eq!(n, 1);
        let repo = CedictRepo::new(db);
        let hits = repo.lookup_exact("台湾").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].english_glosses()[0], "Taiwan");
        // FTS 联动
        let found = repo.search_english("Taiwan", 5).unwrap();
        assert_eq!(found.len(), 1);
    }
}
