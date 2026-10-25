//! 用户配置（config.toml，位于可写数据目录，gitignored）。
//!
//! MW API Key 绝不硬编码/入库：仅存在于本文件；未配置（空）时
//! 整条在线链跳过，软件保持离线模式完整可用。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 外观偏好（NEONLEX UI）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppearanceCfg {
    /// 强调色板："cyan" | "magenta" | "violet"
    #[serde(default = "default_accent")]
    pub accent: String,
    /// CRT 扫描线特效
    #[serde(default = "default_true")]
    pub scanlines: bool,
    /// 等距背景网格
    #[serde(default = "default_true")]
    pub grid: bool,
}

impl Default for AppearanceCfg {
    fn default() -> Self {
        AppearanceCfg {
            accent: default_accent(),
            scanlines: true,
            grid: true,
        }
    }
}

/// 发音偏好。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioCfg {
    /// 查询命中后自动播放发音
    #[serde(default)]
    pub auto_play: bool,
    /// 播放语速（0.5 ~ 2.0；rodio Source::speed）
    #[serde(default = "one")]
    pub speed: f32,
    /// 口音偏好（MW 音频源仅提供美音；字段预留给后续 TTS/英音源）
    #[serde(default = "default_us")]
    pub accent: String,
}

impl Default for AudioCfg {
    fn default() -> Self {
        AudioCfg {
            auto_play: false,
            speed: 1.0,
            accent: default_us(),
        }
    }
}

fn default_accent() -> String {
    "cyan".into()
}
fn default_true() -> bool {
    true
}
fn one() -> f32 {
    1.0
}
fn default_us() -> String {
    "us".into()
}

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
    /// 外观偏好
    #[serde(default)]
    pub appearance: AppearanceCfg,
    /// 发音偏好
    #[serde(default)]
    pub audio: AudioCfg,
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
            appearance: AppearanceCfg::default(),
            audio: AudioCfg::default(),
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
        assert_eq!(c.appearance.accent, "cyan");
        assert!(c.appearance.scanlines);
        assert!(!c.audio.auto_play);
        assert_eq!(c.audio.speed, 1.0);
    }

    #[test]
    fn roundtrip_and_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let c = Config {
            mw_api_key: "test-key".into(),
            cache_ttl_days: 7,
            cache_max_rows: 1000,
            appearance: AppearanceCfg {
                accent: "violet".into(),
                scanlines: false,
                grid: true,
            },
            audio: AudioCfg {
                auto_play: true,
                speed: 1.25,
                accent: "us".into(),
            },
        };
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path), c);

        // 旧版配置文件（无 appearance/audio 节）回退默认
        std::fs::write(&path, "mw_api_key = \"k\"\n").unwrap();
        let partial = Config::load(&path);
        assert_eq!(partial.mw_api_key, "k");
        assert!(partial.online_enabled());
        assert_eq!(partial.cache_ttl_days, 30);
        assert_eq!(partial.appearance.accent, "cyan");
        assert_eq!(partial.audio.speed, 1.0);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let c = Config::load(Path::new("Z:/definitely/not/here/config.toml"));
        assert_eq!(c, Config::default());
    }
}
