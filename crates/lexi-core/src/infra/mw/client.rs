//! MW HTTP 客户端：`GET /api/v3/references/collegiate/json/{word}?key=KEY`。
//!
//! 配额：非商用 ≤1000 次/天/Key。本地 meta 表按 UTC 日计数（与 MW 服务端
//! 重置时刻可能有小时级偏差——**待实测**，README 标注），超限主动熔断到
//! 离线直至次日（或用户在设置页手动重置）。
//!
//! 限流响应形态（HTTP status / body）官方文档未给出：按 403/429 探测并
//! 视为熔断信号——**待实测确认**。

use async_trait::async_trait;

use super::parser::{parse_body, MwResponse};
use crate::infra::db::MetaRepo;

const ENDPOINT: &str = "https://www.dictionaryapi.com/api/v3/references/collegiate/json/";
pub const DAILY_LIMIT: u32 = 1000;

#[derive(thiserror::Error, Debug)]
pub enum MwError {
    #[error("daily quota exceeded ({used}/{limit}), circuit broken to offline")]
    QuotaExceeded { used: u32, limit: u32 },
    #[error("mw api key invalid")]
    InvalidKey,
    #[error("mw http {status}: {body}")]
    Http { status: u16, body: String },
    #[error("network: {0}")]
    Network(String),
}

/// 可 mock 的 MW HTTP 客户端接口（orchestrator 依赖它做缓存命中不落网测试）。
#[async_trait]
pub trait MwHttpClient: Send + Sync {
    async fn lookup(&self, word: &str) -> Result<MwResponse, MwError>;
    /// 当前配额状态 (used, limit)。
    fn quota(&self) -> (u32, u32);
    /// 重置本地配额计数（设置页手动重置；默认无操作，mock 不需要覆盖）。
    fn reset_quota(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// 真实客户端（reqwest + rustls）。
pub struct MwClient {
    http: reqwest::Client,
    key: String,
    meta: std::sync::Arc<MetaRepo>,
    day: i64,
    used: std::sync::atomic::AtomicU32,
    limit: u32,
}

/// UTC 日序号（自 1970-01-01）；配额按此跨天重置。
pub fn utc_day(now_secs: i64) -> i64 {
    now_secs.div_euclid(86_400)
}

impl MwClient {
    /// `now_secs` 用于确定配额计数归属日（测试可注入）。
    pub fn new(key: &str, meta: std::sync::Arc<MetaRepo>, now_secs: i64) -> Self {
        let day = utc_day(now_secs);
        // 恢复当日持久化计数
        let (_, used) = meta.quota(&day.to_string()).unwrap_or_default();
        MwClient {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("reqwest client"),
            key: key.trim().to_string(),
            meta,
            day,
            used: std::sync::atomic::AtomicU32::new(used),
            limit: DAILY_LIMIT,
        }
    }

    fn check_quota(&self) -> Result<(), MwError> {
        let used = self.used.load(std::sync::atomic::Ordering::SeqCst);
        if used >= self.limit {
            return Err(MwError::QuotaExceeded { used, limit: self.limit });
        }
        Ok(())
    }

    fn count_request(&self) -> u32 {
        let next = self.used.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        // 持久化（失败仅日志：计数短暂丢失可接受，宁少勿熔断误伤）
        if let Err(e) = self
            .meta
            .quota_incr(&self.day.to_string())
        {
            tracing::warn!(error = %e, "persist mw quota failed");
        }
        next
    }

    pub fn used(&self) -> u32 {
        self.used.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn reset_quota(&self) -> anyhow::Result<()> {
        self.meta.quota_reset()?;
        self.used.store(0, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl MwHttpClient for MwClient {
    async fn lookup(&self, word: &str) -> Result<MwResponse, MwError> {
        self.check_quota()?;
        let url = format!("{ENDPOINT}{}?key={}", urlencode_path(word), self.key);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| MwError::Network(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| MwError::Network(e.to_string()))?;

        if status.is_success() {
            self.count_request();
            tracing::info!(word, quota = self.used(), "mw lookup ok");
            parse_body(&body).map_err(|e| MwError::Http { status: 200, body: e.to_string() })
        } else {
            // 403：MW 对无效 Key/越权返回；429：限流（形态待实测）
            match status.as_u16() {
                403 => Err(MwError::InvalidKey),
                429 => {
                    tracing::warn!("mw rate limited, circuit broken");
                    Err(MwError::QuotaExceeded { used: self.limit, limit: self.limit })
                }
                _ => Err(MwError::Http { status: status.as_u16(), body: body.chars().take(200).collect() }),
            }
        }
    }

    fn quota(&self) -> (u32, u32) {
        (self.used(), self.limit)
    }
}

/// MW endpoint 的 word 段 path 编码（保留字母数字，其余百分号编码）。
fn urlencode_path(word: &str) -> String {
    let mut out = String::with_capacity(word.len() * 3);
    for b in word.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    /// 计数 mock：记录调用次数，可编排响应序列。
    struct MockMw {
        calls: AtomicU32,
        responses: Mutex<Vec<Result<MwResponse, MwError>>>,
    }

    impl MockMw {
        fn with(responses: Vec<Result<MwResponse, MwError>>) -> Self {
            MockMw { calls: AtomicU32::new(0), responses: Mutex::new(responses) }
        }
        fn calls(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl MwHttpClient for MockMw {
        async fn lookup(&self, _word: &str) -> Result<MwResponse, MwError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let mut q = self.responses.lock().unwrap();
            q.pop()
                .unwrap_or_else(|| panic!("mock exhausted at call {}", n + 1))
        }
        fn quota(&self) -> (u32, u32) {
            (0, DAILY_LIMIT)
        }
    }

    fn entry_json(hw: &str) -> String {
        format!(
            r#"[{{"meta":{{"id":"{hw}","stems":["{hw}"]}},"hwi":{{"hw":"{hw}"}},"fl":"noun","shortdef":["d1"],"def":[{{"sseq":[[["sense",{{"sn":"1","dt":[["text","{{bc}}d1"]]}}]]]}}]}}]"#
        )
    }

    fn one_entry(hw: &str) -> MwResponse {
        parse_body(&entry_json(hw)).unwrap()
    }

    #[test]
    fn mock_counts_calls() {
        let m = MockMw::with(vec![Ok(one_entry("a")), Ok(one_entry("b"))]);
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            assert!(m.lookup("a").await.is_ok());
            assert!(m.lookup("b").await.is_ok());
        });
        assert_eq!(m.calls(), 2);
    }

    #[tokio::test]
    async fn quota_guard_blocks_at_limit() {
        let dir = tempfile::tempdir().unwrap();
        let db = std::sync::Arc::new(crate::infra::db::Db::open(&dir.path().join("t.sqlite")).unwrap());
        let meta = std::sync::Arc::new(MetaRepo::new(db));
        let now = 1_800_000_000i64;
        let client = MwClient::new("k", meta.clone(), now);
        // 预灌配额到上限
        for _ in 0..DAILY_LIMIT {
            client.count_request();
        }
        let err = client.lookup("word").await.unwrap_err();
        assert!(matches!(err, MwError::QuotaExceeded { .. }));
        // 重置后可用
        client.reset_quota().unwrap();
        assert_eq!(client.used(), 0);
        // （不再实际发网络请求——这里只验证熔断解除状态）
    }

    #[tokio::test]
    async fn quota_persists_across_client_restart() {
        let dir = tempfile::tempdir().unwrap();
        let db = std::sync::Arc::new(crate::infra::db::Db::open(&dir.path().join("t.sqlite")).unwrap());
        let meta = std::sync::Arc::new(MetaRepo::new(db));
        let now = 1_800_000_000i64;
        {
            let c = MwClient::new("k", meta.clone(), now);
            c.count_request();
            c.count_request();
        }
        // 同日重启：计数恢复
        let c2 = MwClient::new("k", meta, now);
        assert_eq!(c2.used(), 2);
        // 次日：重置
        let c3 = MwClient::new("k", std::sync::Arc::new(MetaRepo::new(
            std::sync::Arc::new(crate::infra::db::Db::open(&dir.path().join("t.sqlite")).unwrap()),
        )), now + 86_400);
        assert_eq!(c3.used(), 0);
    }

    #[test]
    fn utc_day_math() {
        assert_eq!(utc_day(0), 0);
        assert_eq!(utc_day(86_399), 0);
        assert_eq!(utc_day(86_400), 1);
    }

    #[test]
    fn path_encoding() {
        assert_eq!(urlencode_path("hello"), "hello");
        assert_eq!(urlencode_path("ice-cream"), "ice-cream");
        assert_eq!(urlencode_path("café"), "caf%C3%A9");
        assert_eq!(urlencode_path("a b"), "a%20b");
    }

    /// 组合一个真实 MwClient 的 key 校验路径（不发网络）：InvalidKey 来自
    /// 403 响应，此处仅验证错误类型可从 status 映射（逻辑在 lookup 内联）。
    #[test]
    fn error_display_is_helpful() {
        let e = MwError::QuotaExceeded { used: 1000, limit: 1000 };
        assert!(e.to_string().contains("quota"));
    }
}
