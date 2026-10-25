//! Windows SAPI TTS：本地合成人声，补足 MW 录音不足 3 个发音的缺口。
//!
//! 零依赖方案：经 PowerShell 调 System.Speech（SpVoice）。枚举声音在
//! worker 启动时做一次（spawn_blocking）；朗读每次起一个进程，CREATE_NO_WINDOW
//! 防止控制台闪窗。全部阻塞调用，只允许在后台线程执行。

use std::process::Command;

/// 枚举已安装的英语声音（SAPI GetInstalledVoices，按 en-* 文化过滤）。
/// 返回 SAPI 声音全名（如 "Microsoft David Desktop"）；失败/为空返回空表。
pub fn installed_voices() -> Vec<String> {
    let ps = "Add-Type -AssemblyName System.Speech;"
        .to_string()
        + "(New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices()"
        + " | ForEach-Object { \"$($_.VoiceInfo.Culture)|$($_.VoiceInfo.Name)\" }";
    let out = run_ps(&ps);
    let mut voices = Vec::new();
    for line in out.lines() {
        let Some((culture, name)) = line.split_once('|') else { continue };
        let name = name.trim();
        if culture.trim().starts_with("en") && !name.is_empty() && !voices.iter().any(|v| v == name) {
            voices.push(name.to_string());
        }
    }
    voices
}

/// 同步朗读一个词/短语（阻塞直至播完；调用方用后台线程）。
/// `rate`：SAPI 语速 -10..10（0 = 常速）；`voice` 为空时用系统默认声音。
pub fn speak_blocking(voice: &str, text: &str, rate: i32) -> anyhow::Result<()> {
    if text.trim().is_empty() {
        anyhow::bail!("tts: empty text");
    }
    let select = if voice.is_empty() {
        String::new()
    } else {
        format!("try {{ $s.SelectVoice('{}') }} catch {{ }}", ps_quote(voice))
    };
    let ps = format!(
        "Add-Type -AssemblyName System.Speech; \
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         {select} \
         $s.Rate = {rate}; \
         $s.Speak('{}'); \
         $s.Dispose()",
        ps_quote(text),
    );
    let out = run_ps(&ps);
    // Speak 失败时 PowerShell 向 stderr 输出且 stdout 为空；正常也无 stdout。
    // 检测典型错误文本即可，不苛求。
    if out.contains("Exception") {
        anyhow::bail!("tts speak failed");
    }
    Ok(())
}

/// 按钮徽标：声音名的首字母（"Microsoft David Desktop" → "D"）。
/// 跳过厂牌前缀（Microsoft/Windows），找不到字母时回退 "T"（TTS）。
pub fn voice_badge(name: &str) -> String {
    for token in name.split_whitespace() {
        if token.eq_ignore_ascii_case("microsoft") || token.eq_ignore_ascii_case("windows") {
            continue;
        }
        if let Some(c) = token.chars().next() {
            if c.is_ascii_alphabetic() {
                return c.to_uppercase().to_string();
            }
        }
    }
    "T".to_string()
}

/// rodio 倍速（0.75/1.0/1.25）→ SAPI rate（约 ±2）。
pub fn speed_to_rate(speed: f32) -> i32 {
    ((speed - 1.0) * 8.0).round() as i32
}

fn ps_quote(s: &str) -> String {
    s.replace('\'', "''")
}

fn run_ps(script: &str) -> String {
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    no_window(&mut cmd);
    match cmd.output() {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            if !stdout.trim().is_empty() {
                stdout
            } else {
                String::from_utf8_lossy(&o.stderr).to_string()
            }
        }
        Err(e) => format!("powershell spawn failed: {e}"),
    }
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn no_window(_cmd: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_extracts_voice_initial() {
        assert_eq!(voice_badge("Microsoft David Desktop"), "D");
        assert_eq!(voice_badge("Microsoft Zira Desktop"), "Z");
        assert_eq!(voice_badge("Microsoft Huihui"), "H");
        assert_eq!(voice_badge("Narrator"), "N");
        assert_eq!(voice_badge(""), "T");
        assert_eq!(voice_badge("123 456"), "T");
    }

    #[test]
    fn speed_maps_to_sapi_rate() {
        assert_eq!(speed_to_rate(1.0), 0);
        assert_eq!(speed_to_rate(0.75), -2);
        assert_eq!(speed_to_rate(1.25), 2);
    }

    #[test]
    fn ps_quote_doubles_single_quotes() {
        assert_eq!(ps_quote("it's"), "it''s");
        assert_eq!(ps_quote("plain"), "plain");
    }

    #[test]
    fn speak_rejects_empty_text() {
        assert!(speak_blocking("v", "  ", 0).is_err());
    }

    /// 本机枚举（开发机应至少有一个英语声音；无则空表，不算失败）。
    #[test]
    fn installed_voices_never_panics() {
        let _ = installed_voices();
    }
}
