//! 音频：MW mp3 下载缓存（sha256 命名）+ rodio 本地播放。
//!
//! 查询命中 MW 后 worker 预取首个音频码；点击发音时优先本地文件，
//! 离线可播。播放用独立线程持有 rodio OutputStream 播完即弃。

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::mw::audio_url::build_audio_url;

/// 音频文件缓存路径：<audio_dir>/<sha256(code) 前 20>.mp3
pub fn audio_cache_path(audio_dir: &Path, code: &str) -> PathBuf {
    let mut h = Sha256::new();
    h.update(code.as_bytes());
    let hex: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    audio_dir.join(format!("{}.mp3", &hex[..20]))
}

/// 确保音频已下载到本地；返回本地路径。
/// 已缓存直接返回（离线可用）；否则按 build_audio_url 下载。
pub async fn ensure_downloaded(
    http: &reqwest::Client,
    audio_dir: &Path,
    code: &str,
    ref_: &str,
) -> anyhow::Result<PathBuf> {
    let path = audio_cache_path(audio_dir, code);
    if path.exists() {
        return Ok(path);
    }
    let Some(url) = build_audio_url(code, ref_) else {
        anyhow::bail!("empty audio code");
    };
    let resp = http.get(&url).send().await?.error_for_status()?;
    let bytes = resp.bytes().await?;
    if bytes.is_empty() {
        anyhow::bail!("empty audio body for {code}");
    }
    std::fs::create_dir_all(audio_dir)?;
    let tmp = path.with_extension("mp3.part");
    tokio::fs::write(&tmp, &bytes).await?;
    tokio::fs::rename(&tmp, &path).await?;
    tracing::info!(code, bytes = bytes.len(), "audio downloaded");
    Ok(path)
}

/// 同步播放本地 mp3（阻塞线程直至播完；调用方用后台线程）。
/// `speed`：rodio Source::speed 倍速（0.5~2.0，1.0 = 原速）。
pub fn play_blocking(path: &Path, speed: f32) -> anyhow::Result<()> {
    let (_stream, handle) = rodio::OutputStream::try_default()
        .map_err(|e| anyhow::anyhow!("audio device: {e}"))?;
    let sink = rodio::Sink::try_new(&handle).map_err(|e| anyhow::anyhow!("sink: {e}"))?;
    let file = std::fs::File::open(path)?;
    let decoder = rodio::Decoder::new(std::io::BufReader::new(file))?;
    let speed = speed.clamp(0.5, 2.0);
    if (speed - 1.0).abs() < f32::EPSILON {
        sink.append(decoder);
    } else {
        use rodio::Source as _;
        sink.append(decoder.speed(speed));
    }
    sink.sleep_until_end();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_path_is_stable_and_sanitized() {
        let dir = Path::new("/audio");
        let p1 = audio_cache_path(dir, "volumi02");
        let p2 = audio_cache_path(dir, "volumi02");
        assert_eq!(p1, p2);
        assert!(p1.to_str().unwrap().ends_with(".mp3"));
        // 不同 code 不同文件名
        assert_ne!(p1, audio_cache_path(dir, "other01"));
    }

    #[tokio::test]
    async fn ensure_downloaded_skips_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = audio_cache_path(dir.path(), "volumi02");
        std::fs::write(&path, b"fake-mp3").unwrap();
        // 无网络的 client 也应直接命中缓存
        let http = reqwest::Client::new();
        let got = ensure_downloaded(&http, dir.path(), "volumi02", "c").await.unwrap();
        assert_eq!(got, path);
    }

    #[tokio::test]
    async fn ensure_downloaded_bad_code_errors() {
        let dir = tempfile::tempdir().unwrap();
        let http = reqwest::Client::new();
        // 空码：本地无缓存 → build_audio_url None → 报错（不发请求）
        let err = ensure_downloaded(&http, dir.path(), "", "c").await;
        assert!(err.is_err());
    }
}
