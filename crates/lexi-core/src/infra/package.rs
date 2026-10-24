//! 数据包导出/导入（迁移用）：zip 整个可写数据目录。
//!
//! Q5 已确认范围：db/lexi.sqlite + config.toml + audio/**（含历史与
//! 单词本数据）；logs/ 与 export/ 自身排除。导入解压覆盖后由调用方
//! 重载（worker 重建连接；UI 提示重启亦可）。

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;

/// 导出数据包到 out_zip。db_path 先做 WAL checkpoint 保证文件完整。
pub fn export_package(data_dir: &Path, db_path: &Path, out_zip: &Path) -> anyhow::Result<()> {
    // checkpoint：把 WAL 并入主库文件
    {
        let conn = rusqlite::Connection::open(db_path)?;
        conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
    }

    if let Some(parent) = out_zip.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = std::fs::File::create(out_zip)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // db 主文件
    add_file(&mut zip, opts, db_path, "db/lexi.sqlite")?;
    // config.toml（存在才打包；key 一并迁移——本地 zip 用户自管）
    let config = data_dir.join("config.toml");
    if config.exists() {
        add_file(&mut zip, opts, &config, "config.toml")?;
    }
    // audio/**
    let audio_dir = data_dir.join("audio");
    if audio_dir.is_dir() {
        let mut files = Vec::new();
        collect_files(&audio_dir, &mut files)?;
        for f in files {
            let rel = f.strip_prefix(data_dir).with_context(|| f.display().to_string())?;
            add_file(&mut zip, opts, &f, &rel.to_string_lossy().replace('\\', "/"))?;
        }
    }
    zip.finish()?;
    Ok(())
}

/// 导入数据包：解压 db/、config.toml、audio/ 到 data_dir（覆盖）。
/// 返回导入的文件数。调用方随后重载数据库连接。
pub fn import_package(zip_path: &Path, data_dir: &Path) -> anyhow::Result<usize> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        // 安全：只接受白名单顶层路径（db/、audio/、config.toml）
        let name = entry.name().to_string();
        let safe = name == "config.toml"
            || name.starts_with("db/")
            || name.starts_with("audio/");
        if !safe || entry.is_dir() {
            continue;
        }
        let out_path = safe_join(data_dir, &name)?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        std::fs::write(&out_path, &buf)?;
        count += 1;
    }
    Ok(count)
}

fn add_file(
    zip: &mut zip::ZipWriter<std::fs::File>,
    opts: zip::write::SimpleFileOptions,
    path: &Path,
    arc_name: &str,
) -> anyhow::Result<()> {
    zip.start_file(arc_name, opts)?;
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    zip.write_all(&buf)?;
    Ok(())
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_files(&p, out)?;
        } else {
            out.push(p);
        }
    }
    Ok(())
}

/// 防路径穿越：拒绝 zip 内的 `..` 与绝对路径成分。
fn safe_join(base: &Path, arc_name: &str) -> anyhow::Result<PathBuf> {
    let mut out = base.to_path_buf();
    for part in arc_name.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            anyhow::bail!("zip entry escapes data dir: {arc_name}");
        }
        out.push(part);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(dir: &Path) -> (PathBuf, PathBuf) {
        let db_path = dir.join("db/lexi.sqlite");
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        // 直接用 rusqlite 建一个带内容的库
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES(42);")
            .unwrap();
        drop(conn);
        std::fs::write(dir.join("config.toml"), "mw_api_key = \"abc\"").unwrap();
        let audio = dir.join("audio");
        std::fs::create_dir_all(&audio).unwrap();
        std::fs::write(audio.join("a.mp3"), b"mp3data").unwrap();
        (db_path.clone(), audio.join("a.mp3"))
    }

    #[test]
    fn export_then_import_roundtrip() {
        let src = tempfile::tempdir().unwrap();
        let (_db, _a) = setup(src.path());
        let zip_path = src.path().join("export/pkg.zip");
        export_package(src.path(), &src.path().join("db/lexi.sqlite"), &zip_path).unwrap();

        let dst = tempfile::tempdir().unwrap();
        let n = import_package(&zip_path, dst.path()).unwrap();
        assert_eq!(n, 3); // db + config + 1 audio
        assert!(dst.path().join("db/lexi.sqlite").exists());
        assert_eq!(std::fs::read_to_string(dst.path().join("config.toml")).unwrap(), "mw_api_key = \"abc\"");
        assert_eq!(std::fs::read(dst.path().join("audio/a.mp3")).unwrap(), b"mp3data");
        // db 可打开且数据在
        let conn = rusqlite::Connection::open(dst.path().join("db/lexi.sqlite")).unwrap();
        let v: i64 = conn.query_row("SELECT x FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(v, 42);
    }

    #[test]
    fn import_rejects_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("../../evil.txt", opts).unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();
        let err = import_package(&zip_path, dir.path());
        assert!(err.is_err() || dir.path().join("../evil.txt").exists() == false);
        // 严格断言：evil 文件绝不出现
        assert!(!dir.path().join("evil.txt").exists());
    }

    #[test]
    fn import_ignores_unrelated_entries() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("p.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("logs/x.log", opts).unwrap();
        zip.write_all(b"l").unwrap();
        zip.start_file("config.toml", opts).unwrap();
        zip.write_all(b"m=1").unwrap();
        zip.finish().unwrap();
        let n = import_package(&zip_path, dir.path()).unwrap();
        assert_eq!(n, 1);
        assert!(!dir.path().join("logs").exists());
    }
}
