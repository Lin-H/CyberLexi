-- CyberLexi schema v1: meta / cache / wordbook / history / offline dicts
-- 所有 SQL 语句由 migration runner 按 meta.schema_version 顺序执行

CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
-- 约定键: schema_version / data_version / mw_quota_date('YYYY-MM-DD') / mw_quota_used

CREATE TABLE IF NOT EXISTS cache_entry (
  word_key    TEXT PRIMARY KEY,           -- normalized key (L1 lowercase trim)
  raw_json    TEXT NOT NULL,              -- MW 完整报文（数组原样），含 suggestions 场景则不落库
  headword    TEXT,
  phonetic    TEXT,
  pos         TEXT,
  short_defs  TEXT,                       -- JSON array
  senses_json TEXT,                       -- JSON array of Sense
  audio_codes TEXT,                       -- JSON array
  etymology   TEXT,
  date_text   TEXT,
  stems       TEXT,                       -- JSON array
  labels      TEXT,                       -- JSON array
  fetched_at  INTEGER NOT NULL,           -- unix secs
  hit_count   INTEGER NOT NULL DEFAULT 0,
  ttl_days    INTEGER NOT NULL DEFAULT 30
);
CREATE INDEX IF NOT EXISTS idx_cache_fetched ON cache_entry(fetched_at ASC);

CREATE TABLE IF NOT EXISTS wordbook (
  word_key  TEXT PRIMARY KEY,             -- 引用 cache_entry/ecdict，不重复存词条内容
  headword  TEXT NOT NULL,
  status    TEXT NOT NULL DEFAULT 'new'
            CHECK(status IN ('new','learning','familiar','mastered')),
  tags      TEXT NOT NULL DEFAULT '[]',   -- JSON array
  note      TEXT NOT NULL DEFAULT '',
  added_at  INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_wordbook_updated ON wordbook(updated_at DESC);

CREATE TABLE IF NOT EXISTS history (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  query_text TEXT NOT NULL,
  queried_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_history_time ON history(queried_at DESC);

CREATE TABLE IF NOT EXISTS ecdict (
  word        TEXT PRIMARY KEY,
  phonetic    TEXT,
  pos         TEXT,
  translation TEXT,
  collins     INTEGER NOT NULL DEFAULT 0,
  oxford      INTEGER NOT NULL DEFAULT 0,
  tag         TEXT,
  bnc         INTEGER NOT NULL DEFAULT 0,
  frq         INTEGER NOT NULL DEFAULT 0,
  exchange    TEXT
);
CREATE INDEX IF NOT EXISTS idx_ecdict_word ON ecdict(word);

CREATE TABLE IF NOT EXISTS cedict (
  trad         TEXT NOT NULL,
  simp         TEXT NOT NULL,
  pinyin       TEXT NOT NULL,             -- 带声调数字 "ni3 hao3"
  pinyin_plain TEXT NOT NULL,             -- 无声调 "ni hao"
  english      TEXT NOT NULL,             -- "/gloss1/gloss2/" 原样
  PRIMARY KEY (simp, trad, pinyin, english)
);
CREATE INDEX IF NOT EXISTS idx_cedict_simp   ON cedict(simp);
CREATE INDEX IF NOT EXISTS idx_cedict_trad   ON cedict(trad);
CREATE INDEX IF NOT EXISTS idx_cedict_pinyin ON cedict(pinyin_plain);
