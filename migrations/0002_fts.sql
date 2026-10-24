-- FTS5 全文索引（rusqlite bundled SQLite >= 3.45，trigram 可用）
-- ecdict.translation 为中文文本：trigram tokenizer 支持任意子串匹配（unicode61 无法切 CJK）
-- ecdict 导入走批量 INSERT 触发同步 trigger；重导前先 DROP 重建

CREATE VIRTUAL TABLE IF NOT EXISTS ecdict_fts USING fts5(
  word, translation,
  content='ecdict', content_rowid='rowid',
  tokenize='trigram'
);

CREATE TRIGGER IF NOT EXISTS ecdict_fts_ai AFTER INSERT ON ecdict BEGIN
  INSERT INTO ecdict_fts(rowid, word, translation)
  VALUES (new.rowid, new.word, new.translation);
END;
CREATE TRIGGER IF NOT EXISTS ecdict_fts_ad AFTER DELETE ON ecdict BEGIN
  INSERT INTO ecdict_fts(ecdict_fts, rowid, word, translation)
  VALUES ('delete', old.rowid, old.word, old.translation);
END;
CREATE TRIGGER IF NOT EXISTS ecdict_fts_au AFTER UPDATE ON ecdict BEGIN
  INSERT INTO ecdict_fts(ecdict_fts, rowid, word, translation)
  VALUES ('delete', old.rowid, old.word, old.translation);
  INSERT INTO ecdict_fts(rowid, word, translation)
  VALUES (new.rowid, new.word, new.translation);
END;

-- cedict: simp/trad 走 B-tree 精确索引，FTS 服务拼音前缀与英文释义匹配
CREATE VIRTUAL TABLE IF NOT EXISTS cedict_fts USING fts5(
  pinyin, pinyin_plain, english,
  content='cedict', content_rowid='rowid',
  tokenize='unicode61'
);

CREATE TRIGGER IF NOT EXISTS cedict_fts_ai AFTER INSERT ON cedict BEGIN
  INSERT INTO cedict_fts(rowid, pinyin, pinyin_plain, english)
  VALUES (new.rowid, new.pinyin, new.pinyin_plain, new.english);
END;
CREATE TRIGGER IF NOT EXISTS cedict_fts_ad AFTER DELETE ON cedict BEGIN
  INSERT INTO cedict_fts(cedict_fts, rowid, pinyin, pinyin_plain, english)
  VALUES ('delete', old.rowid, old.pinyin, old.pinyin_plain, old.english);
END;
CREATE TRIGGER IF NOT EXISTS cedict_fts_au AFTER UPDATE ON cedict BEGIN
  INSERT INTO cedict_fts(cedict_fts, rowid, pinyin, pinyin_plain, english)
  VALUES ('delete', old.rowid, old.pinyin, old.pinyin_plain, old.english);
  INSERT INTO cedict_fts(rowid, pinyin, pinyin_plain, english)
  VALUES (new.rowid, new.pinyin, new.pinyin_plain, new.english);
END;
