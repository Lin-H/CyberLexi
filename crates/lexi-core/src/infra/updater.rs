//! GitHub Releases 自动更新（纯 IO 逻辑，零 GPUI；全部在 worker 内执行）。
//!
//! 流程：`fetch_latest`（/releases/latest，不含预发布）→ `is_newer`
//! semver 比较 → `platform_asset` 选平台包 → `download` 流式下载
//! （.part 落盘后改名，尺寸一致则复用）→ `prepare` 解包并生成安装
//! 入口 → `apply` 拉起安装程序/替换脚本（detached，应用随即退出）。
//!
//! 平台策略（与 release.yml 产物命名对齐）：
//! - Windows：优先 NSIS `*-setup.exe` 覆盖安装（.onInit 等待旧进程
//!   退出）；便携 zip 则 robocopy 覆盖 + 重启（apply.cmd，data/ 保留）
//! - macOS：ditto 解包 .app，apply.sh 整包替换后 open（仅 .app 内运行）
//! - Linux：tar 解包，apply.sh cp -a 覆盖 + 重启（data/ 保留）
//!
//! 版本号：CI 经 `CYBERLEXI_VERSION` 环境变量在编译期注入（与 exe
//! 版本资源同源）；dev 构建回退 CARGO_PKG_VERSION。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

const API_LATEST: &str = "https://api.github.com/repos/Lin-H/CyberLexi/releases/latest";

/// 当前版本（编译期确定）。CI 打 tag 构建时注入 CYBERLEXI_VERSION。
pub fn current_version() -> &'static str {
    option_env!("CYBERLEXI_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    #[serde(default)]
    pub body: String,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub size: u64,
}

/// 供 UI 展示的更新摘要（纯类型，经 BackendEvent 传递）。
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub version: String,
    pub notes: String,
    pub url: String,
    /// 空 = 当前平台无自动更新包（UI 仅展示发布页链接）
    pub asset_name: String,
    pub asset_size: u64,
}

/// 拉取最新稳定 Release（GitHub 要求 User-Agent；未认证限流 60 次/时）。
pub async fn fetch_latest(http: &reqwest::Client) -> anyhow::Result<ReleaseInfo> {
    let resp = http
        .get(API_LATEST)
        .header("User-Agent", format!("CyberLexi/{} (auto-updater)", current_version()))
        .header("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .context("请求 GitHub Releases API")?;
    let status = resp.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("尚无已发布的稳定版本");
    }
    if !status.is_success() {
        anyhow::bail!("GitHub API 返回 {status}（可能限流或网络异常）");
    }
    Ok(resp.json().await.context("解析 Release JSON")?)
}

/// latest 是否比 current 新（semver，容忍 tag 的 v 前缀）。
/// 任一侧解析失败返回 false——宁可不提示也不误报。
pub fn is_newer(latest_tag: &str, current: &str) -> bool {
    let parse = |s: &str| semver::Version::parse(s.trim_start_matches('v')).ok();
    match (parse(latest_tag), parse(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

/// 按平台后缀选自动更新包（与版本号解耦；Windows 优先安装程序）。
pub fn select_asset<'a>(assets: &'a [Asset], os: &str, arch: &str) -> Option<&'a Asset> {
    let find = |suffix: &str| assets.iter().find(|a| a.name.ends_with(suffix));
    match (os, arch) {
        ("windows", "x86_64") => {
            find("-windows-x64-setup.exe").or_else(|| find("-windows-x64.zip"))
        }
        ("macos", "aarch64") => find("-macos-arm64.zip"),
        ("linux", "x86_64") => find("-linux-x64.tar.gz"),
        _ => None,
    }
}

pub fn platform_asset(rel: &ReleaseInfo) -> Option<&Asset> {
    select_asset(&rel.assets, std::env::consts::OS, std::env::consts::ARCH)
}

pub fn update_info(rel: &ReleaseInfo) -> UpdateInfo {
    let asset = platform_asset(rel);
    UpdateInfo {
        version: rel.tag_name.trim_start_matches('v').to_string(),
        notes: rel.body.clone(),
        url: rel.html_url.clone(),
        asset_name: asset.map(|a| a.name.clone()).unwrap_or_default(),
        asset_size: asset.map(|a| a.size).unwrap_or(0),
    }
}

/// 流式下载到 updates 目录；progress(pct, downloaded, total) 按整百分比
/// 节流回调。total 未知时 pct 恒 0。已有同尺寸成品直接复用。
pub async fn download(
    http: &reqwest::Client,
    url: &str,
    name: &str,
    size_hint: u64,
    dir: &Path,
    mut progress: impl FnMut(f32, u64, u64),
) -> anyhow::Result<PathBuf> {
    // 资产名来自远端 JSON：拒绝路径成分，防写入目录之外
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        anyhow::bail!("非法更新包文件名：{name}");
    }
    std::fs::create_dir_all(dir)?;
    let final_path = dir.join(name);
    if size_hint > 0 {
        if let Ok(meta) = std::fs::metadata(&final_path) {
            if meta.len() == size_hint {
                progress(1.0, size_hint, size_hint);
                return Ok(final_path);
            }
        }
    }
    let mut resp = http
        .get(url)
        .header("User-Agent", format!("CyberLexi/{} (auto-updater)", current_version()))
        // 整请求超时放宽到 10 分钟（reqwest timeout 含 body 流式读取）
        .timeout(std::time::Duration::from_secs(600))
        .send()
        .await
        .context("下载更新包")?;
    if !resp.status().is_success() {
        anyhow::bail!("下载返回 {}", resp.status());
    }
    let total = size_hint.max(resp.content_length().unwrap_or(0));
    let part = dir.join(format!("{name}.part"));
    let mut file = tokio::fs::File::create(&part).await?;
    let mut downloaded = 0u64;
    let mut last_pct = -1i32;
    while let Some(chunk) = resp.chunk().await.context("读取下载流")? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        let pct = if total > 0 {
            (downloaded as f64 / total as f64) as f32
        } else {
            0.0
        };
        let whole = (pct * 100.0) as i32;
        if whole != last_pct {
            last_pct = whole;
            progress(pct.min(1.0), downloaded, total);
        }
    }
    file.flush().await?;
    drop(file);
    if total > 0 && downloaded != total {
        anyhow::bail!("下载不完整（{downloaded}/{total} 字节），请重试");
    }
    tokio::fs::rename(&part, &final_path).await?;
    progress(1.0, downloaded, total);
    Ok(final_path)
}

/// 解包并生成安装入口（阻塞 IO，worker 经 spawn_blocking 调用）。
/// 返回 install 时拉起的程序路径：setup.exe 原样；其余为替换脚本。
pub fn prepare(asset_path: &Path, updates_dir: &Path) -> anyhow::Result<PathBuf> {
    let name = asset_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if name.ends_with("-setup.exe") {
        return Ok(asset_path.to_path_buf());
    }
    let staging = updates_dir.join("staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    if name.ends_with(".zip") {
        if cfg!(target_os = "macos") {
            // ditto 保留 bundle 权限与签名结构
            run(Command::new("ditto").arg("-x").arg("-k").arg(asset_path).arg(&staging))?;
            prepare_macos(&staging, updates_dir)
        } else {
            extract_zip(asset_path, &staging)?;
            prepare_windows_portable(&staging, updates_dir)
        }
    } else if name.ends_with(".tar.gz") {
        run(Command::new("tar").arg("-xzf").arg(asset_path).arg("-C").arg(&staging))?;
        prepare_linux(&staging, updates_dir)
    } else {
        anyhow::bail!("不支持的更新包格式：{name}")
    }
}

/// 拉起安装入口（detached 子进程，应用退出后存活）。调用方随后立即退出。
pub fn apply(launch: &Path) -> anyhow::Result<()> {
    let name = launch
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut cmd = if name.ends_with(".exe") {
        Command::new(launch)
    } else if name.ends_with(".cmd") {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(launch);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW：覆盖过程不闪控制台；start 拉起的新实例仍有窗口
            c.creation_flags(0x0800_0000);
        }
        c
    } else {
        let mut c = Command::new("sh");
        c.arg(launch);
        c
    };
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("启动 {}", launch.display()))?;
    Ok(())
}

/// Windows 便携 zip：robocopy 覆盖安装目录（不带 /MIR，data/ 保留），
/// 重试 15 次容忍旧进程尚未完全退出，随后重启新版。
fn prepare_windows_portable(staging: &Path, updates_dir: &Path) -> anyhow::Result<PathBuf> {
    let src = find_app_dir(staging, "cyberlexi.exe")?;
    let exe = std::env::current_exe().context("定位当前 exe")?;
    let install_dir = exe.parent().context("exe 无父目录")?;
    let script = format!(
        "@echo off\r\n\
         timeout /t 2 /nobreak >nul\r\n\
         robocopy \"{src}\" \"{dst}\" /E /R:15 /W:1 /NFL /NDL /NJH /NJS\r\n\
         start \"\" \"{new_exe}\"\r\n\
         del \"%~f0\"\r\n",
        src = src.display(),
        dst = install_dir.display(),
        new_exe = install_dir.join("cyberlexi.exe").display(),
    );
    let path = updates_dir.join("apply.cmd");
    std::fs::write(&path, script)?;
    Ok(path)
}

/// macOS：仅支持从 .app bundle 内运行时整包替换。
fn prepare_macos(staging: &Path, updates_dir: &Path) -> anyhow::Result<PathBuf> {
    let bundle = macos_bundle_path()
        .context("当前未从 .app 包内运行，无法自动替换；请到发布页手动下载")?;
    let new_app = staging.join("CyberLexi.app");
    if !new_app.is_dir() {
        anyhow::bail!("更新包内未找到 CyberLexi.app");
    }
    let script = format!(
        "#!/bin/sh\n\
         i=0\n\
         while pgrep -x cyberlexi >/dev/null 2>&1 && [ \"$i\" -lt 30 ]; do sleep 1; i=$((i+1)); done\n\
         rm -rf {bundle}\n\
         ditto {new_app} {bundle}\n\
         xattr -dr com.apple.quarantine {bundle} 2>/dev/null || true\n\
         open {bundle}\n\
         rm -f \"$0\"\n",
        bundle = sh_quote(&bundle),
        new_app = sh_quote(&new_app),
    );
    let path = updates_dir.join("apply.sh");
    std::fs::write(&path, script)?;
    Ok(path)
}

/// Linux：cp -a 合并覆盖（data/ 保留），重启新版。
fn prepare_linux(staging: &Path, updates_dir: &Path) -> anyhow::Result<PathBuf> {
    let src = find_app_dir(staging, "cyberlexi")?;
    let exe = std::env::current_exe().context("定位当前 exe")?;
    let install_dir = exe.parent().context("exe 无父目录")?;
    let new_exe = install_dir.join("cyberlexi");
    let script = format!(
        "#!/bin/sh\n\
         i=0\n\
         while pgrep -x cyberlexi >/dev/null 2>&1 && [ \"$i\" -lt 30 ]; do sleep 1; i=$((i+1)); done\n\
         cp -a {src}/. {dst}/\n\
         chmod +x {exe}\n\
         nohup {exe} >/dev/null 2>&1 &\n\
         rm -f \"$0\"\n",
        src = sh_quote(&src),
        dst = sh_quote(install_dir),
        exe = sh_quote(&new_exe),
    );
    let path = updates_dir.join("apply.sh");
    std::fs::write(&path, script)?;
    Ok(path)
}

/// 当前 exe 所在的 .app bundle（.../CyberLexi.app/Contents/MacOS/cyberlexi）。
fn macos_bundle_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let contents = exe.parent()?.parent()?;
    let bundle = contents.parent()?;
    if bundle.extension().and_then(|e| e.to_str()) == Some("app")
        && contents.join("Info.plist").exists()
    {
        Some(bundle.to_path_buf())
    } else {
        None
    }
}

/// 解包目录内找含标记文件的应用目录（顶层或唯一子目录）。
fn find_app_dir(staging: &Path, marker: &str) -> anyhow::Result<PathBuf> {
    if staging.join(marker).exists() {
        return Ok(staging.to_path_buf());
    }
    for entry in std::fs::read_dir(staging)? {
        let p = entry?.path();
        if p.is_dir() && p.join(marker).exists() {
            return Ok(p);
        }
    }
    anyhow::bail!("更新包内未找到 {marker}")
}

/// zip 解包（enclosed_name 拒绝路径穿越；仅 Windows 便携包走这里）。
fn extract_zip(zip_path: &Path, dest: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        let mut buf = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut buf)?;
        std::fs::write(&out, &buf)?;
    }
    Ok(())
}

fn run(cmd: &mut Command) -> anyhow::Result<()> {
    let out = cmd.output().context("启动解包进程")?;
    if !out.status.success() {
        anyhow::bail!(
            "解包失败（{}）：{}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn sh_quote(p: &Path) -> String {
    format!("'{}'", p.display().to_string().replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const RELEASE_JSON: &str = r###"{
        "tag_name": "v0.2.0",
        "name": "CyberLexi v0.2.0",
        "body": "## What's Changed\n* feat: auto-update",
        "html_url": "https://github.com/Lin-H/CyberLexi/releases/tag/v0.2.0",
        "draft": false,
        "prerelease": false,
        "assets": [
            {"name": "cyberlexi-0.2.0-windows-x64.zip", "size": 100,
             "browser_download_url": "https://example.com/win.zip"},
            {"name": "cyberlexi-0.2.0-windows-x64-setup.exe", "size": 200,
             "browser_download_url": "https://example.com/win-setup.exe"},
            {"name": "cyberlexi-0.2.0-macos-arm64.zip", "size": 300,
             "browser_download_url": "https://example.com/mac.zip"},
            {"name": "cyberlexi-0.2.0-linux-x64.tar.gz", "size": 400,
             "browser_download_url": "https://example.com/linux.tar.gz"}
        ]
    }"###;

    fn release() -> ReleaseInfo {
        serde_json::from_str(RELEASE_JSON).unwrap()
    }

    #[test]
    fn parses_release_json() {
        let rel = release();
        assert_eq!(rel.tag_name, "v0.2.0");
        assert_eq!(rel.assets.len(), 4);
        assert_eq!(rel.assets[1].size, 200);
        assert!(rel.body.contains("auto-update"));
    }

    #[test]
    fn parses_release_without_optional_fields() {
        let rel: ReleaseInfo = serde_json::from_str(
            r#"{"tag_name":"v1.0.0","html_url":"https://x"}"#,
        )
        .unwrap();
        assert!(rel.assets.is_empty());
        assert_eq!(rel.body, "");
    }

    #[test]
    fn semver_comparison() {
        assert!(is_newer("v0.1.2", "0.1.0"));
        assert!(is_newer("v0.2.0", "0.1.9"));
        assert!(is_newer("v1.0.0", "0.99.99"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
        // 预发布号低于同号正式版
        assert!(!is_newer("v0.1.3-rc.1", "0.1.3"));
        assert!(is_newer("v0.1.3-rc.1", "0.1.2"));
        // 解析失败不误报
        assert!(!is_newer("nightly", "0.1.0"));
        assert!(!is_newer("v0.2.0", "dev"));
    }

    #[test]
    fn asset_selection_per_platform() {
        let rel = release();
        // Windows 优先安装程序
        let a = select_asset(&rel.assets, "windows", "x86_64").unwrap();
        assert!(a.name.ends_with("-setup.exe"));
        // 无安装程序时回退便携 zip
        let zip_only: Vec<Asset> = rel
            .assets
            .iter()
            .filter(|a| !a.name.ends_with(".exe"))
            .cloned()
            .collect();
        let a = select_asset(&zip_only, "windows", "x86_64").unwrap();
        assert!(a.name.ends_with("-windows-x64.zip"));
        assert!(select_asset(&rel.assets, "macos", "aarch64")
            .unwrap()
            .name
            .ends_with("-macos-arm64.zip"));
        // macOS x86_64 无产物（CI 仅 arm64）
        assert!(select_asset(&rel.assets, "macos", "x86_64").is_none());
        assert!(select_asset(&rel.assets, "linux", "x86_64")
            .unwrap()
            .name
            .ends_with(".tar.gz"));
        assert!(select_asset(&rel.assets, "freebsd", "x86_64").is_none());
    }

    #[test]
    fn update_info_without_platform_asset() {
        let mut rel = release();
        rel.assets.clear();
        let info = update_info(&rel);
        assert_eq!(info.version, "0.2.0");
        assert!(info.asset_name.is_empty());
        assert_eq!(info.asset_size, 0);
        assert!(!info.url.is_empty());
    }

    #[tokio::test]
    async fn download_rejects_path_traversal_name() {
        let dir = tempfile::tempdir().unwrap();
        let http = reqwest::Client::new();
        let res = download(&http, "https://example.com/x", "../evil.exe", 0, dir.path(), |_, _, _| {}).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn download_reuses_complete_file() {
        let dir = tempfile::tempdir().unwrap();
        let name = "cyberlexi-9.9.9-windows-x64.zip";
        std::fs::write(dir.path().join(name), b"0123456789").unwrap();
        let http = reqwest::Client::new();
        let mut called = false;
        // URL 不可达也能成功——命中本地成品直接返回
        let path = download(
            &http,
            "https://invalid.example/x",
            name,
            10,
            dir.path(),
            |pct, d, t| {
                called = true;
                assert_eq!((pct, d, t), (1.0, 10, 10));
            },
        )
        .await
        .unwrap();
        assert!(called);
        assert_eq!(path, dir.path().join(name));
    }

    #[test]
    fn setup_exe_passes_through() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("cyberlexi-9.9.9-windows-x64-setup.exe");
        std::fs::write(&exe, b"MZ").unwrap();
        let launch = prepare(&exe, dir.path()).unwrap();
        assert_eq!(launch, exe);
        // 不产生 staging
        assert!(!dir.path().join("staging").exists());
    }

    #[test]
    fn find_app_dir_searches_top_and_subdir() {
        let dir = tempfile::tempdir().unwrap();
        // 顶层命中
        std::fs::write(dir.path().join("cyberlexi.exe"), b"x").unwrap();
        assert_eq!(find_app_dir(dir.path(), "cyberlexi.exe").unwrap(), dir.path());
        // 子目录命中（release zip 的实际布局）
        let dir2 = tempfile::tempdir().unwrap();
        let inner = dir2.path().join("cyberlexi-9.9.9-windows-x64");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("cyberlexi.exe"), b"x").unwrap();
        assert_eq!(find_app_dir(dir2.path(), "cyberlexi.exe").unwrap(), inner);
        // 未命中
        let dir3 = tempfile::tempdir().unwrap();
        assert!(find_app_dir(dir3.path(), "cyberlexi.exe").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn prepare_portable_zip_extracts_and_writes_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("cyberlexi-9.9.9-windows-x64.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zw.start_file("cyberlexi-9.9.9-windows-x64/cyberlexi.exe", opts).unwrap();
        zw.write_all(b"MZ-fake").unwrap();
        zw.start_file("cyberlexi-9.9.9-windows-x64/assets/dict/x.csv", opts).unwrap();
        zw.write_all(b"word,def").unwrap();
        zw.finish().unwrap();

        let launch = prepare(&zip_path, dir.path()).unwrap();
        assert_eq!(launch, dir.path().join("apply.cmd"));
        assert!(dir.path().join("staging/cyberlexi-9.9.9-windows-x64/assets/dict/x.csv").exists());
        let script = std::fs::read_to_string(&launch).unwrap();
        assert!(script.contains("robocopy"));
        assert!(script.contains("cyberlexi.exe"));
        assert!(script.contains("del \"%~f0\""));
    }

    #[test]
    fn sh_quote_escapes_single_quotes() {
        assert_eq!(sh_quote(Path::new("/a/b app")), "'/a/b app'");
        assert_eq!(sh_quote(Path::new("/a/it's")), r"'/a/it'\''s'");
    }
}