-- Phase 2 占位表：学习计划 / 复习算法 / 学习统计。
-- Phase 1 只建表与空 trait（见 domain/learning.rs），不写任何业务逻辑。

CREATE TABLE IF NOT EXISTS review_log (
  id          INTEGER PRIMARY KEY,
  word_key    TEXT NOT NULL,
  reviewed_at INTEGER NOT NULL,
  result      TEXT NOT NULL DEFAULT ''    -- Phase 2 定义枚举
);

CREATE TABLE IF NOT EXISTS study_plan (
  id       INTEGER PRIMARY KEY,
  payload  TEXT NOT NULL DEFAULT '{}'     -- Phase 2 定义结构
);
