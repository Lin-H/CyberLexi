//! 离线词库导入：ECDICT mini CSV + CC-CEDICT TSV → SQLite（含 FTS5）。
//!
//! 首次启动（或数据文件变更）时在 tokio worker 执行，UI 只收进度事件。
//! data_version = 两文件内容 sha256 的组合，存 meta；相同则跳过重导。

pub mod cedict;
pub mod ecdict;

use anyhow::Context;
use sha2::{Digest, Sha256};

use crate::infra::db::MetaRepo;

/// 计算数据文件的指纹（sha256 前 16 hex，两文件拼接）。
pub fn data_fingerprint(ecdict_path: &std::path::Path, cedict_path: &std::path::Path) -> anyhow::Result<String> {
    let a = sha256_hex(ecdict_path)?;
    let b = sha256_hex(cedict_path)?;
    Ok(format!("{a}:{b}"))
}

fn sha256_hex(path: &std::path::Path) -> anyhow::Result<String> {
    let data = std::fs::read(path)
        .with_context(|| format!("read {}", path.display()))?;
    let mut h = Sha256::new();
    h.update(&data);
    let out = h.finalize();
    Ok(out.iter().map(|b| format!("{b:02x}")).take(16).collect())
}

/// 是否需要（重）导入：指纹与 meta.data_version 比较。
pub fn needs_import(meta: &MetaRepo, fingerprint: &str) -> bool {
    match meta.get("data_version") {
        Ok(Some(v)) => v != fingerprint,
        _ => true,
    }
}

/// 导入完成后记录指纹。
pub fn mark_imported(meta: &MetaRepo, fingerprint: &str) -> anyhow::Result<()> {
    meta.set("data_version", fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn fingerprint_changes_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.csv");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "hello").unwrap();
        std::fs::write(&b, "world").unwrap();
        let f1 = data_fingerprint(&a, &b).unwrap();
        std::fs::write(&a, "hello2").unwrap();
        let f2 = data_fingerprint(&a, &b).unwrap();
        assert_ne!(f1, f2);
        assert_eq!(f1.len(), 65); // 32hex + ':' + 32hex
    }

    #[test]
    fn needs_import_until_marked() {
        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(crate::infra::db::Db::open(&dir.path().join("t.sqlite")).unwrap());
        let meta = MetaRepo::new(db);
        assert!(needs_import(&meta, "x"));
        mark_imported(&meta, "x").unwrap();
        assert!(!needs_import(&meta, "x"));
        assert!(needs_import(&meta, "y"));
    }
}
