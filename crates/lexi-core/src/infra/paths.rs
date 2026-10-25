//! 数据目录与资源目录解析。
//!
//! 可写数据（spec 目录策略）：先尝试 exe 同级 `data/`（便携模式），
//! 探针写入失败（如 Program Files 无权限）则回退
//! `dirs::data_local_dir()/com.cyberlexi/`。
//! 只读资源：exe 同级 `assets/`；dev（cargo run）下 exe 位于
//! `target/…`，回退编译期 `CARGO_MANIFEST_DIR/../../assets` 与 cwd。

use std::path::{Path, PathBuf};

use anyhow::Context;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub assets_dir: PathBuf,
}

impl AppPaths {
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("db").join("lexi.sqlite")
    }
    pub fn audio_dir(&self) -> PathBuf {
        self.data_dir.join("audio")
    }
    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }
    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }
    pub fn export_dir(&self) -> PathBuf {
        self.data_dir.join("export")
    }

    /// 解析路径并创建全部可写子目录。
    pub fn resolve() -> anyhow::Result<AppPaths> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf));

        let data_dir = match exe_dir.as_deref() {
            Some(exe) if dir_is_writable(&exe.join("data")) => exe.join("data"),
            _ => fallback_data_dir()?,
        };

        let assets_dir = locate_assets(exe_dir.as_deref())
            .context("assets/ 目录未找到（应与 exe 同级分发；dev 下须在仓库根构建）")?;

        for sub in ["db", "audio", "export"] {
            std::fs::create_dir_all(data_dir.join(sub))
                .with_context(|| format!("create {}", data_dir.join(sub).display()))?;
        }
        // 日志目录仅调试构建需要（发布版零日志，见 worker::init_tracing）
        #[cfg(debug_assertions)]
        std::fs::create_dir_all(data_dir.join("logs"))
            .with_context(|| format!("create {}", data_dir.join("logs").display()))?;
        Ok(AppPaths { data_dir, assets_dir })
    }

    pub fn ecdict_csv(&self) -> PathBuf {
        self.assets_dir.join("dict").join("ecdict.mini.csv")
    }
    pub fn cedict_txt(&self) -> PathBuf {
        self.assets_dir.join("dict").join("cedict.txt")
    }
    pub fn fonts(&self) -> PathBuf {
        self.assets_dir.join("fonts")
    }
}

/// 便携目录探针：能创建目录并写入+删除探针文件才算可写。
fn dir_is_writable(dir: &Path) -> bool {
    std::fs::create_dir_all(dir).is_ok() && {
        let probe = dir.join(".write_probe");
        let ok = std::fs::write(&probe, b"1").is_ok();
        let _ = std::fs::remove_file(&probe);
        ok
    }
}

fn fallback_data_dir() -> anyhow::Result<PathBuf> {
    let base = dirs::data_local_dir().context("data_local_dir unavailable")?;
    Ok(base.join("com.cyberlexi"))
}

fn locate_assets(exe_dir: Option<&Path>) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(exe) = exe_dir {
        candidates.push(exe.join("assets"));
    }
    // dev（cargo run）：lexi-core 的 manifest 是 crates/lexi-core
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(Path::new(manifest).join("../../assets"));
    }
    candidates.push(PathBuf::from("assets"));
    candidates
        .into_iter()
        .find(|p| p.join("dict").is_dir() && p.join("fonts").is_dir())
        .map(|p| p.canonicalize().unwrap_or(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_dir_wins_when_writable() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("data");
        assert!(dir_is_writable(&dir));
        assert!(dir.join(".write_probe").starts_with(&dir));
    }

    #[test]
    fn subdir_layout_is_stable() {
        let p = AppPaths {
            data_dir: PathBuf::from("/x"),
            assets_dir: PathBuf::from("/a"),
        };
        assert_eq!(p.db_path(), PathBuf::from("/x/db/lexi.sqlite"));
        assert_eq!(p.config_path(), PathBuf::from("/x/config.toml"));
        assert_eq!(p.audio_dir(), PathBuf::from("/x/audio"));
        assert_eq!(p.ecdict_csv(), PathBuf::from("/a/dict/ecdict.mini.csv"));
        assert_eq!(p.cedict_txt(), PathBuf::from("/a/dict/cedict.txt"));
    }
}
