//! 用户配置（config.toml，位于可写数据目录，gitignored）。
//!
//! MW API Key 绝不硬编码/入库：仅存在于本文件；未配置（空）时
//! 整条在线链跳过，软件保持离线模式完整可用。

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    /// Merriam-Webster Developer API Key；空 = 离线模式
    #[serde(default)]
    pub mw_api_key: String,
    /// 缓存 TTL（天）
    #[serde(default = "default_ttl_days")]
    pub cache_ttl_days: u32,
    /// 缓存行数上限（冷淘汰阈值）
    #[serde(default = "default_cache_max_rows")]
    pub cache_max_rows: u32,
}

fn default_ttl_days() -> u32 {
    30
}
fn default_cache_max_rows() -> u32 {
    50_000
}

impl Default for Config {
    fn default() -> Self {
        Config {
            mw_api_key: String::new(),
            cache_ttl_days: default_ttl_days(),
            cache_max_rows: default_cache_max_rows(),
        }
    }
}

impl Config {
    pub fn online_enabled(&self) -> bool {
        !self.mw_api_key.trim().is_empty()
    }

    /// 读取配置；文件不存在或损坏时回退默认值（不阻塞启动）。
    pub fn load(path: &Path) -> Config {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                tracing::warn!(path = path.display().to_string(), error = %e, "config parse failed, using defaults");
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_offline() {
        let c = Config::default();
        assert!(!c.online_enabled());
        assert_eq!(c.cache_ttl_days, 30);
        assert_eq!(c.cache_max_rows, 50_000);
    }

    #[test]
    fn roundtrip_and_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let c = Config {
            mw_api_key: "test-key".into(),
            cache_ttl_days: 7,
            cache_max_rows: 1000,
        };
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path), c);

        // 部分字段缺失时取默认
        std::fs::write(&path, "mw_api_key = \"k\"\n").unwrap();
        let partial = Config::load(&path);
        assert_eq!(partial.mw_api_key, "k");
        assert!(partial.online_enabled());
        assert_eq!(partial.cache_ttl_days, 30);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let c = Config::load(Path::new("Z:/definitely/not/here/config.toml"));
        assert_eq!(c, Config::default());
    }
}
