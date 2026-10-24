//! 查询编排：统一入口，英/中双链 + 降级 + 合并。
//!
//! 英文链（spec）：cache → MW（配置了 Key 且配额未熔断）→ ECDICT 兜底 →
//! （纯字母时）CEDICT 拼音/英文候选。中文链：CEDICT 精确 → ECDICT
//! translation FTS。缓存命中的第二次查询零网络请求（单测断言）。

use std::sync::Arc;

use crate::domain::merger::ResultMerger;
use crate::domain::normalizer::{Normalizer, NormalizedQuery};
use crate::domain::types::{
    Candidate, Lang, MergedEntry, MwEntry, QueryResult, Sense, SourceTag,
};
use crate::infra::config::Config;
use crate::infra::db::{CedictRepo, EcdictRepo, HistoryRepo, MetaRepo};
use crate::infra::mw::client::MwClient;
use crate::infra::mw::{MwError, MwHttpClient, MwResponse};
use crate::infra::db::cache_repo::{CacheRepo, SqliteCacheRepo};

pub struct QueryOrchestrator {
    cache: Arc<dyn CacheRepo>,
    mw: tokio::sync::RwLock<Option<Arc<dyn MwHttpClient>>>,
    ecdict: Arc<EcdictRepo>,
    cedict: Arc<CedictRepo>,
    history: Arc<HistoryRepo>,
    normalizer: Normalizer,
    merger: ResultMerger,
    cfg: tokio::sync::RwLock<Config>,
}

impl QueryOrchestrator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cache: Arc<dyn CacheRepo>,
        mw: Option<Arc<dyn MwHttpClient>>,
        ecdict: Arc<EcdictRepo>,
        cedict: Arc<CedictRepo>,
        history: Arc<HistoryRepo>,
        cfg: Config,
    ) -> Self {
        QueryOrchestrator {
            cache,
            mw: tokio::sync::RwLock::new(mw),
            ecdict,
            cedict,
            history,
            normalizer: Normalizer::l1(),
            merger: ResultMerger,
            cfg: tokio::sync::RwLock::new(cfg),
        }
    }

    /// 设置页改完即用：更新配置并按需重建 MW 客户端（改完即生效）。
    pub async fn update_config(
        &self,
        cfg: Config,
        meta: Arc<MetaRepo>,
        now_secs: i64,
    ) -> anyhow::Result<()> {
        let client = if cfg.online_enabled() {
            Some(Arc::new(MwClient::new(&cfg.mw_api_key, meta, now_secs)) as Arc<dyn MwHttpClient>)
        } else {
            None
        };
        *self.mw.write().await = client;
        *self.cfg.write().await = cfg;
        Ok(())
    }

    pub async fn config(&self) -> Config {
        self.cfg.read().await.clone()
    }

    pub async fn online_enabled(&self) -> bool {
        self.mw.read().await.is_some()
    }

    /// 当前 MW 客户端（离线模式为 None；worker 用于配额重置）。
    pub async fn mw_client(&self) -> Option<Arc<dyn MwHttpClient>> {
        self.mw.read().await.clone()
    }

    /// 统一查询入口。
    pub async fn query(&self, raw: &str, now: i64) -> anyhow::Result<QueryResult> {
        let nq = self.normalizer.normalize(raw);
        if nq.key.is_empty() {
            return Ok(QueryResult::NotFound { input: raw.to_string(), suggestions: Vec::new() });
        }
        let _ = self.history.push(&nq.display, now);
        match nq.lang {
            Lang::English => Ok(self.query_english(&nq, now).await),
            Lang::Chinese => Ok(self.query_chinese(&nq, false).await),
        }
    }

    /// 点击中文候选后补全详情（复用中文链精确记录 → 映射为词条展示）。
    pub async fn query_detail(&self, raw: &str, now: i64) -> anyhow::Result<QueryResult> {
        let nq = self.normalizer.normalize(raw);
        if nq.key.is_empty() {
            return Ok(QueryResult::NotFound { input: raw.to_string(), suggestions: Vec::new() });
        }
        match nq.lang {
            Lang::Chinese => Ok(self.query_chinese(&nq, true).await),
            Lang::English => Ok(self.query_english(&nq, now).await),
        }
    }

    async fn query_english(&self, nq: &NormalizedQuery, now: i64) -> QueryResult {
        // 1. 缓存探针（命中即零网络）
        if let Ok(Some(hit)) = self.cache.get(&nq.key, now).await {
            tracing::info!(key = nq.key, "cache hit");
            let ecdict = self.ecdict.lookup(&nq.key).ok().flatten();
            return QueryResult::Entry(self.merger.merge(Some(hit.mw), ecdict, &nq.key));
        }

        // 2. MW 在线（有 Key 且配额未熔断）
        let mw_guard = self.mw.read().await;
        if let Some(client) = mw_guard.as_ref() {
            match client.lookup(&nq.key).await {
                Ok(MwResponse::Entries(entries)) => {
                    let combined = combine_entries(entries);
                    let cfg = self.cfg.read().await;
                    let ttl = cfg.cache_ttl_days;
                    drop(cfg);
                    if let Err(e) = self.cache.put(&nq.key, &combined, ttl, now).await {
                        tracing::warn!(error = %e, "cache put failed");
                    }
                    let ecdict = self.ecdict.lookup(&nq.key).ok().flatten();
                    return QueryResult::Entry(self.merger.merge(Some(combined), ecdict, &nq.key));
                }
                Ok(MwResponse::Suggestions(s)) => {
                    // 未命中：降级离线，保留拼写建议
                    return self.offline_fallback(nq, Some(s), now).await;
                }
                Err(MwError::QuotaExceeded { used, limit }) => {
                    tracing::warn!(used, limit, "mw quota exceeded, offline fallback");
                }
                Err(MwError::InvalidKey) => {
                    tracing::warn!("mw api key rejected, offline fallback");
                }
                Err(e) => {
                    tracing::warn!(error = %e, "mw lookup failed, offline fallback");
                }
            }
        }
        // 3. 离线兜底（无 Key / MW 失败 / 限流）
        self.offline_fallback(nq, None, now).await
    }

    async fn offline_fallback(
        &self,
        nq: &NormalizedQuery,
        suggestions: Option<Vec<String>>,
        _now: i64,
    ) -> QueryResult {
        if let Some(rec) = self.ecdict.lookup(&nq.key).ok().flatten() {
            return QueryResult::Entry(self.merger.merge(None, Some(rec), &nq.key));
        }
        // 纯字母（可能是拼音）：CEDICT 拼音前缀 + 英文释义候选
        if nq.key.chars().all(|c| c.is_ascii_alphabetic() || c == ' ') && !nq.key.is_empty() {
            let cands = self.cedict_candidates(&nq.key).await;
            if !cands.is_empty() {
                return QueryResult::Candidates(cands);
            }
        }
        QueryResult::NotFound {
            input: nq.display.clone(),
            suggestions: suggestions.unwrap_or_default(),
        }
    }

    /// CEDICT 候选：拼音前缀优先，叠加英文释义 FTS，去重限 20。
    async fn cedict_candidates(&self, q: &str) -> Vec<Candidate> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let push = |recs: Vec<crate::domain::types::CedictRecord>, out: &mut Vec<Candidate>, seen: &mut std::collections::HashSet<String>| {
            for r in recs {
                if seen.insert(r.simplified.clone()) {
                    out.push(Candidate {
                        word: r.simplified.clone(),
                        pinyin: Some(r.pinyin.clone()),
                        gloss: r.first_gloss(40),
                    });
                }
            }
        };
        if let Ok(recs) = self.cedict.search_pinyin(q, 20) {
            push(recs, &mut out, &mut seen);
        }
        if out.len() < 20 {
            if let Ok(recs) = self.cedict.search_english(q, 20) {
                push(recs, &mut out, &mut seen);
            }
        }
        out.truncate(20);
        out
    }

    async fn query_chinese(&self, nq: &NormalizedQuery, detail: bool) -> QueryResult {
        let exact = self.cedict.lookup_exact(&nq.key).unwrap_or_default();
        if detail {
            if !exact.is_empty() {
                return QueryResult::Entry(cedict_entry(&nq.key, &exact));
            }
            return QueryResult::NotFound {
                input: nq.display.clone(),
                suggestions: Vec::new(),
            };
        }
        if !exact.is_empty() {
            return QueryResult::Candidates(
                exact
                    .iter()
                    .map(|r| Candidate {
                        word: r.simplified.clone(),
                        pinyin: Some(r.pinyin.clone()),
                        gloss: r.first_gloss(40),
                    })
                    .collect(),
            );
        }
        // ECDICT translation FTS 兜底（低优先级，限 20）
        if let Ok(hits) = self.ecdict.search_translation(&nq.key, 20) {
            let cands: Vec<Candidate> = hits
                .into_iter()
                .map(|(word, translation)| Candidate {
                    word,
                    pinyin: None,
                    gloss: translation
                        .split("\\n")
                        .next()
                        .unwrap_or(&translation)
                        .chars()
                        .take(40)
                        .collect(),
                })
                .collect();
            if !cands.is_empty() {
                return QueryResult::Candidates(cands);
            }
        }
        QueryResult::NotFound {
            input: nq.display.clone(),
            suggestions: Vec::new(),
        }
    }
}

/// 多 homograph 合并为单词条（首 entry 为骨架，senses/audio 串联）。
/// 注意：cache.raw_json 保留首个 entry 的原始 JSON（完整报文数组在
/// MW 响应层已各 entry 分开存储）。
fn combine_entries(entries: Vec<MwEntry>) -> MwEntry {
    let mut it = entries.into_iter();
    let mut first = it.next().expect("combine called with entries");
    for e in it {
        first.senses.extend(e.senses);
        for c in e.audio_codes {
            if !first.audio_codes.contains(&c) {
                first.audio_codes.push(c);
            }
        }
        for s in e.stems {
            if !first.stems.contains(&s) {
                first.stems.push(s);
            }
        }
        if first.phonetic.is_empty() {
            first.phonetic = e.phonetic;
        }
        if first.pos.is_empty() {
            first.pos = e.pos;
        }
        if first.etymology.is_none() {
            first.etymology = e.etymology;
        }
        if first.date.is_none() {
            first.date = e.date;
        }
    }
    first
}

/// CEDICT 记录集 → 词条展示（点击候选后的详情）。
fn cedict_entry(key: &str, recs: &[crate::domain::types::CedictRecord]) -> MergedEntry {
    let headword = recs.first().map(|r| r.simplified.clone()).unwrap_or_else(|| key.to_string());
    let pinyin = recs.first().map(|r| r.pinyin.clone()).unwrap_or_default();
    let mut glosses: Vec<String> = Vec::new();
    for r in recs {
        for g in r.english_glosses() {
            if !glosses.contains(&g) {
                glosses.push(g);
            }
        }
    }
    let senses = glosses
        .iter()
        .enumerate()
        .map(|(i, g)| Sense { sn: Some((i + 1).to_string()), text: g.clone(), examples: Vec::new() })
        .collect();
    MergedEntry {
        headword_display: headword.clone(),
        normalized_key: key.to_string(),
        phonetic: pinyin,
        pos: String::new(),
        short_defs: glosses.iter().take(3).cloned().collect(),
        senses,
        chinese: Some(headword),
        freq: None,
        exam_tags: Vec::new(),
        audio_codes: Vec::new(),
        etymology: None,
        date: None,
        sources: vec![SourceTag::Cedict],
    }
}

/// 组装默认编排器（worker 使用）。
pub fn build_orchestrator(
    db: crate::infra::db::DbHandle,
    cfg: Config,
    meta: Option<Arc<MetaRepo>>,
    now_secs: i64,
) -> Arc<QueryOrchestrator> {
    let mw: Option<Arc<dyn MwHttpClient>> = match (&meta, cfg.online_enabled()) {
        (Some(meta), true) => Some(Arc::new(MwClient::new(&cfg.mw_api_key, meta.clone(), now_secs))),
        _ => None,
    };
    let history = Arc::new(HistoryRepo::new(db.clone()));
    Arc::new(QueryOrchestrator::new(
        Arc::new(SqliteCacheRepo::new(db.clone())),
        mw,
        Arc::new(EcdictRepo::new(db.clone())),
        Arc::new(CedictRepo::new(db.clone())),
        history,
        cfg,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{CedictRecord, EcdictRecord};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingMock {
        calls: AtomicU32,
        entries_body: Mutex<Vec<Result<MwResponse, MwError>>>,
    }

    impl CountingMock {
        fn ok(bodies: Vec<&str>) -> Self {
            CountingMock {
                calls: AtomicU32::new(0),
                entries_body: Mutex::new(
                    bodies
                        .into_iter()
                        .map(|b| Ok(crate::infra::mw::parse_body(b).unwrap()))
                        .collect(),
                ),
            }
        }
        fn err(e: MwError) -> Self {
            CountingMock { calls: AtomicU32::new(0), entries_body: Mutex::new(vec![Err(e)]) }
        }
        fn calls(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    use std::sync::Mutex;

    #[async_trait]
    impl MwHttpClient for CountingMock {
        async fn lookup(&self, _word: &str) -> Result<MwResponse, MwError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.entries_body.lock().unwrap().pop().expect("mock exhausted")
        }
        fn quota(&self) -> (u32, u32) {
            (0, 1000)
        }
    }

    fn mw_body(hw: &str) -> String {
        serde_json::json!([{
            "meta": {"id": hw, "stems": [hw]},
            "hwi": {"hw": hw, "prs": [{
                "mw": "ɪkˈsæmpəl",
                "sound": {"audio": "exam0001", "ref": "c", "stat": "1"}
            }]},
            "fl": "noun",
            "shortdef": ["an example"],
            "def": [{"sseq": [[
                ["sense", {"sn": "1", "dt": [
                    ["text", "{bc}an example"],
                    ["vis", [{"t": "an {wi}exemplar{\\/wi} sentence"}]]
                ]}]
            ]]}]
        }])
        .to_string()
    }

    async fn fixture(mw: Option<Arc<CountingMock>>) -> (Arc<QueryOrchestrator>, crate::infra::db::DbHandle) {
        let db = crate::infra::db::DbHandle::from(Arc::new(
            crate::infra::db::Db::open_in_memory().unwrap(),
        ));
        let ecdict = EcdictRepo::new(db.clone());
        ecdict.insert_batch(&[
            EcdictRecord {
                word: "apple".into(),
                phonetic: "ˈæpl".into(),
                pos: "n".into(),
                translation: "n. 苹果".into(),
                collins: 3, oxford: 1, tag: "cet4".into(), bnc: 100, frq: 200,
                exchange: String::new(),
            },
            EcdictRecord {
                word: "banana".into(),
                phonetic: String::new(),
                pos: "n".into(),
                translation: "n. 香蕉".into(),
                collins: 0, oxford: 0, tag: String::new(), bnc: 0, frq: 0,
                exchange: String::new(),
            },
        ]).unwrap();
        let cedict = CedictRepo::new(db.clone());
        cedict.insert_batch(&[CedictRecord {
            traditional: "蘋果".into(),
            simplified: "苹果".into(),
            pinyin: "ping2 guo3".into(),
            pinyin_plain: "ping guo".into(),
            english: "/apple/(fruit)/".into(),
        }]).unwrap();
        let orch = Arc::new(QueryOrchestrator::new(
            Arc::new(SqliteCacheRepo::new(db.clone())),
            mw.map(|m| m as Arc<dyn MwHttpClient>),
            Arc::new(ecdict), Arc::new(cedict),
            Arc::new(HistoryRepo::new(db.clone())),
            Config::default(),
        ));
        (orch, db)
    }

    fn now() -> i64 {
        1_800_000_000
    }

    #[tokio::test]
    async fn second_query_hits_cache_no_network() {
        let mock = Arc::new(CountingMock::ok(vec![&mw_body("voluminous")]));
        let (orch, _db) = fixture(Some(mock.clone())).await;
        let first = orch.query("Voluminous ", now()).await.unwrap();
        let QueryResult::Entry(e1) = first else { panic!("entry") };
        assert_eq!(e1.headword_display, "voluminous");
        assert!(e1.sources.contains(&SourceTag::Mw));
        assert_eq!(mock.calls(), 1);
        // 二查：零网络（缓存命中），且结果等价
        let second = orch.query("voluminous", now() + 1).await.unwrap();
        let QueryResult::Entry(e2) = second else { panic!("entry") };
        assert_eq!(e2.headword_display, "voluminous");
        assert_eq!(mock.calls(), 1, "cache hit must not hit network");
    }

    #[tokio::test]
    async fn ecdict_chinese_translation_merged_with_mw() {
        let mock = Arc::new(CountingMock::ok(vec![&mw_body("apple")]));
        let (orch, _) = fixture(Some(mock)).await;
        let r = orch.query("apple", now()).await.unwrap();
        let QueryResult::Entry(e) = r else { panic!() };
        assert_eq!(e.chinese.as_deref(), Some("n. 苹果"));
        assert_eq!(e.freq, Some((100, 200, 3, 1)));
        assert_eq!(e.exam_tags, vec!["cet4"]);
    }

    #[tokio::test]
    async fn no_key_falls_to_ecdict_offline() {
        let (orch, _) = fixture(None).await;
        let r = orch.query("apple", now()).await.unwrap();
        let QueryResult::Entry(e) = r else { panic!("offline entry") };
        assert_eq!(e.sources, vec![SourceTag::Ecdict]);
        assert_eq!(e.headword_display, "apple");
    }

    #[tokio::test]
    async fn mw_error_falls_to_ecdict() {
        let mock = Arc::new(CountingMock::err(MwError::Network("boom".into())));
        let (orch, _) = fixture(Some(mock)).await;
        let r = orch.query("apple", now()).await.unwrap();
        let QueryResult::Entry(e) = r else { panic!() };
        assert_eq!(e.sources, vec![SourceTag::Ecdict]);
    }

    #[tokio::test]
    async fn quota_exceeded_falls_to_ecdict() {
        let mock = Arc::new(CountingMock::err(MwError::QuotaExceeded { used: 1000, limit: 1000 }));
        let (orch, _) = fixture(Some(mock)).await;
        let r = orch.query("apple", now()).await.unwrap();
        assert!(matches!(r, QueryResult::Entry(_)));
    }

    #[tokio::test]
    async fn suggestions_surface_in_not_found() {
        let body = r#"["voluminous","voluminousness"]"#;
        let mock = Arc::new(CountingMock::ok(vec![body]));
        let (orch, _) = fixture(Some(mock)).await;
        // MW 未命中且 ECDICT 无收录
        let r = orch.query("voluminousx", now()).await.unwrap();
        match r {
            QueryResult::NotFound { suggestions, .. } => {
                assert_eq!(suggestions, vec!["voluminous", "voluminousness"]);
            }
            _ => panic!("expected NotFound"),
        }
    }

    #[tokio::test]
    async fn chinese_exact_returns_candidates_then_detail() {
        let (orch, _) = fixture(None).await;
        let r = orch.query("苹果", now()).await.unwrap();
        let QueryResult::Candidates(c) = r else { panic!("candidates") };
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].word, "苹果");
        assert_eq!(c[0].pinyin.as_deref(), Some("ping2 guo3"));
        // 点击候选 → 详情
        let d = orch.query_detail("苹果", now()).await.unwrap();
        let QueryResult::Entry(e) = d else { panic!("detail entry") };
        assert_eq!(e.headword_display, "苹果");
        assert!(e.short_defs.contains(&"apple".to_string()));
        assert_eq!(e.sources, vec![SourceTag::Cedict]);
    }

    #[tokio::test]
    async fn pinyin_english_input_finds_cedict_candidates() {
        let (orch, _) = fixture(None).await;
        // "ping" 是纯字母：ecdict 无 → cedict pinyin 前缀候选
        let r = orch.query("ping", now()).await.unwrap();
        let QueryResult::Candidates(c) = r else { panic!("candidates, got {r:?}") };
        assert!(c.iter().any(|x| x.word == "苹果"));
        // 英文释义 FTS 兜底（ecdict miss 的英文词）
        let r = orch.query("fruitlike", now()).await.unwrap(); // 无 MW、无 ecdict
        // 无 cedict english 前缀命中（"fruitlike" 非 "fruit" 前缀词…… fruit 是 gloss 内词）
        // english FTS 用前缀表达式，"fruit" 应命中 /apple/(fruit)/
        let r2 = orch.query("fruit", now()).await.unwrap();
        let QueryResult::Candidates(c2) = r2 else { panic!("candidates") };
        assert!(c2.iter().any(|x| x.word == "苹果"));
        let _ = r; // fruitlike 允许 NotFound
    }

    #[tokio::test]
    async fn chinese_ecdict_translation_fallback() {
        let (orch, _) = fixture(None).await;
        // "香蕉" 无 cedict 收录 → ecdict translation LIKE 兜底命中 banana
        let r = orch.query("香蕉", now()).await.unwrap();
        let QueryResult::Candidates(c) = r else { panic!("candidates, got {r:?}") };
        assert!(c.iter().any(|x| x.word == "banana"), "{c:?}");
    }

    #[tokio::test]
    async fn empty_query_returns_not_found() {
        let (orch, _) = fixture(None).await;
        let r = orch.query("   ", now()).await.unwrap();
        assert!(matches!(r, QueryResult::NotFound { .. }));
    }

    #[tokio::test]
    async fn history_recorded() {
        let (orch, db) = fixture(None).await;
        orch.query("apple", now()).await.unwrap();
        let h = HistoryRepo::new(db);
        assert_eq!(h.recent().unwrap(), vec!["apple".to_string()]);
    }
}
