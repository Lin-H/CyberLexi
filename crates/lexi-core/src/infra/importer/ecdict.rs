//! ECDICT CSV 导入（按列名映射，兼容完整版/mini 版同构数据）。

use std::path::PathBuf;

use anyhow::Context as _;
use tokio::sync::mpsc;

use crate::domain::types::EcdictRecord;
use crate::infra::db::{DbHandle, EcdictRepo};

const BATCH: usize = 5000;

/// 导入 ECDICT csv。返回导入行数。进度经 channel 发 (pct, stage)。
pub async fn import_ecdict(
    db: DbHandle,
    csv_path: PathBuf,
    progress: mpsc::UnboundedSender<(f32, String)>,
) -> anyhow::Result<u64> {
    tokio::task::spawn_blocking(move || {
        let _ = progress.send((0.0, "ecdict: open".into()));
        let repo = EcdictRepo::new(db.clone());
        repo.clear()?;
        let file_len = std::fs::metadata(&csv_path)?.len().max(1);
        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_path(&csv_path)?;
        let headers = rdr.headers()?.clone();
        let col = |name: &str| -> anyhow::Result<Option<usize>> {
            Ok(headers.iter().position(|h| h == name))
        };
        let (c_word, c_phon, c_pos, c_trans, c_collins, c_oxford, c_tag, c_bnc, c_frq, c_exc) = (
            col("word")?.context("ecdict csv missing 'word' column")?,
            col("phonetic")?,
            col("pos")?,
            col("translation")?,
            col("collins")?,
            col("oxford")?,
            col("tag")?,
            col("bnc")?,
            col("frq")?,
            col("exchange")?,
        );

        let mut batch: Vec<EcdictRecord> = Vec::with_capacity(BATCH);
        let mut total: u64 = 0;
        for rec in rdr.records() {
            let rec = rec?;
            let get = |i: Option<usize>| -> String {
                i.and_then(|i| rec.get(i)).unwrap_or("").trim().to_string()
            };
            let geti = |i: Option<usize>| -> i32 {
                i.and_then(|i| rec.get(i))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0)
            };
            let word = get(Some(c_word));
            if word.is_empty() {
                continue;
            }
            batch.push(EcdictRecord {
                word,
                phonetic: get(c_phon),
                pos: get(c_pos),
                translation: get(c_trans),
                collins: geti(c_collins),
                oxford: geti(c_oxford),
                tag: get(c_tag),
                bnc: geti(c_bnc),
                frq: geti(c_frq),
                exchange: get(c_exc),
            });
            if batch.len() >= BATCH {
                repo.insert_batch(&batch)?;
                total += batch.len() as u64;
                batch.clear();
                // records() 持有 &mut rdr，不能同时读 position()：
                // 以已处理行数估算进度（~180B/行 平均）
                let est_bytes = total.saturating_mul(180);
                let pct = (est_bytes as f32 / file_len as f32 * 100.0).min(99.0);
                let _ = progress.send((pct, format!("ecdict: {total} rows")));
            }
        }
        if !batch.is_empty() {
            repo.insert_batch(&batch)?;
            total += batch.len() as u64;
        }
        let _ = progress.send((100.0, format!("ecdict: done {total}")));
        Ok(total)
    })
    .await
    .map_err(|e| anyhow::anyhow!("join: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn imports_csv_with_named_columns() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("mini.csv");
        // 列顺序故意与 repo 结构不同：验证按列名映射
        std::fs::write(
            &csv,
            "word,phonetic,definition,translation,pos,collins,oxford,tag,bnc,frq,exchange\n\
             apple,ˈæpl,,n. 苹果\\nn. 苹果树,n,3,1,cet4,1000,2000,d:apples\n\
             test,test,,n. 测试,n,0,0,zk,1,2,\n",
        )
        .unwrap();
        let db = DbHandle::from(Arc::new(crate::infra::db::Db::open_in_memory().unwrap()));
        let (tx, _rx) = mpsc::unbounded_channel();
        let n = import_ecdict(db.clone(), csv, tx).await.unwrap();
        assert_eq!(n, 2);
        let repo = EcdictRepo::new(db);
        let got = repo.lookup("apple").unwrap().unwrap();
        assert_eq!(got.phonetic, "ˈæpl");
        assert!(got.translation.contains("苹果"));
        assert_eq!(got.collins, 3);
        assert_eq!(got.oxford, 1);
        assert_eq!(got.tag, "cet4");
        assert_eq!(got.bnc, 1000);
        assert_eq!(got.frq, 2000);
        assert_eq!(got.exchange, "d:apples");
        // FTS 已联动（trigger）
        let hits = repo.search_translation("苹果树", 10).unwrap();
        assert!(hits.iter().any(|(w, _)| w == "apple"));
    }

    #[tokio::test]
    async fn skips_empty_word_rows() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("mini.csv");
        std::fs::write(
            &csv,
            "word,phonetic,translation\n,phon,n. x\nok,,n. y\n",
        )
        .unwrap();
        let db = DbHandle::from(Arc::new(crate::infra::db::Db::open_in_memory().unwrap()));
        let (tx, _rx) = mpsc::unbounded_channel();
        let n = import_ecdict(db, csv, tx).await.unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    async fn reimport_replaces() {
        let dir = tempfile::tempdir().unwrap();
        let csv = dir.path().join("mini.csv");
        std::fs::write(&csv, "word,translation\naa,n. 甲\n").unwrap();
        let db = DbHandle::from(Arc::new(crate::infra::db::Db::open_in_memory().unwrap()));
        let (tx, _rx) = mpsc::unbounded_channel();
        import_ecdict(db.clone(), csv.clone(), tx).await.unwrap();
        std::fs::write(&csv, "word,translation\nbb,n. 乙\n").unwrap();
        let (tx, _rx) = mpsc::unbounded_channel();
        import_ecdict(db.clone(), csv, tx).await.unwrap();
        let repo = EcdictRepo::new(db);
        assert_eq!(repo.count().unwrap(), 1);
        assert!(repo.lookup("bb").unwrap().is_some());
        assert!(repo.lookup("aa").unwrap().is_none());
    }
}
