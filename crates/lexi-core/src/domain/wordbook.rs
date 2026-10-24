//! 单词本领域模型。
//!
//! Phase 1：增删改查、标签、四态流转、按 `word_key` 去重（不重复存储
//! 词条内容，只引用 cache_entry / ecdict 中已保存的数据）。

use serde::{Deserialize, Serialize};

/// 单词本四态（Q2 已确认：任意跳转与回退，每次流转记 updated_at）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WordStatus {
    New,
    Learning,
    Familiar,
    Mastered,
}

impl WordStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WordStatus::New => "new",
            WordStatus::Learning => "learning",
            WordStatus::Familiar => "familiar",
            WordStatus::Mastered => "mastered",
        }
    }

    pub fn parse(s: &str) -> Option<WordStatus> {
        match s {
            "new" => Some(WordStatus::New),
            "learning" => Some(WordStatus::Learning),
            "familiar" => Some(WordStatus::Familiar),
            "mastered" => Some(WordStatus::Mastered),
            _ => None,
        }
    }

    /// 全部状态（UI 流转菜单顺序）。
    pub fn all() -> [WordStatus; 4] {
        [
            WordStatus::New,
            WordStatus::Learning,
            WordStatus::Familiar,
            WordStatus::Mastered,
        ]
    }
}

/// 单词本条目。`word_key` 即查询链路的 normalized key，
/// 与 cache_entry / 单词本去重键共用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordBookItem {
    pub word_key: String,
    pub headword: String,
    pub status: WordStatus,
    pub tags: Vec<String>,
    pub note: String,
    /// unix 秒
    pub added_at: i64,
    /// unix 秒（状态/标签/备注任一变更即更新）
    pub updated_at: i64,
}

/// 单词本门面（repo 之上的领域服务，repo 实现见 infra/db/wordbook_repo.rs）。
pub struct WordBook<S> {
    store: S,
}

impl<S: WordBookStore> WordBook<S> {
    pub fn new(store: S) -> Self {
        WordBook { store }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    /// 添加词条；已存在同 key 时合并标签并返回 false（不重复添加）。
    pub fn add(&self, item: WordBookItem) -> anyhow::Result<bool> {
        if self.store.exists(&item.word_key)? {
            let mut existing = self.store.get(&item.word_key)?.expect("checked exists");
            for t in item.tags {
                if !existing.tags.contains(&t) {
                    existing.tags.push(t);
                }
            }
            existing.updated_at = item.added_at.max(existing.updated_at);
            self.store.put(existing)?;
            Ok(false)
        } else {
            self.store.put(item)?;
            Ok(true)
        }
    }

    pub fn remove(&self, word_key: &str) -> anyhow::Result<bool> {
        self.store.delete(word_key)
    }

    pub fn set_status(&self, word_key: &str, status: WordStatus, now: i64) -> anyhow::Result<()> {
        let mut item = self
            .store
            .get(word_key)?
            .ok_or_else(|| anyhow::anyhow!("word not in wordbook: {word_key}"))?;
        item.status = status;
        item.updated_at = now;
        self.store.put(item)
    }

    pub fn set_tags(&self, word_key: &str, tags: Vec<String>, now: i64) -> anyhow::Result<()> {
        let mut item = self
            .store
            .get(word_key)?
            .ok_or_else(|| anyhow::anyhow!("word not in wordbook: {word_key}"))?;
        item.tags = tags;
        item.updated_at = now;
        self.store.put(item)
    }

    pub fn list(&self) -> anyhow::Result<Vec<WordBookItem>> {
        self.store.list_recent_first()
    }
}

/// 单词本存储接口（infra 层实现；单测用内存实现）。
pub trait WordBookStore: Send + Sync {
    fn exists(&self, word_key: &str) -> anyhow::Result<bool>;
    fn get(&self, word_key: &str) -> anyhow::Result<Option<WordBookItem>>;
    fn put(&self, item: WordBookItem) -> anyhow::Result<()>;
    fn delete(&self, word_key: &str) -> anyhow::Result<bool>;
    fn list_recent_first(&self) -> anyhow::Result<Vec<WordBookItem>>;
}

/// 内存实现（单测/演示用）。
#[derive(Default)]
pub struct InMemoryWordBookStore {
    items: std::sync::Mutex<std::collections::HashMap<String, WordBookItem>>,
}

impl WordBookStore for InMemoryWordBookStore {
    fn exists(&self, word_key: &str) -> anyhow::Result<bool> {
        Ok(self.items.lock().unwrap().contains_key(word_key))
    }
    fn get(&self, word_key: &str) -> anyhow::Result<Option<WordBookItem>> {
        Ok(self.items.lock().unwrap().get(word_key).cloned())
    }
    fn put(&self, item: WordBookItem) -> anyhow::Result<()> {
        self.items
            .lock()
            .unwrap()
            .insert(item.word_key.clone(), item);
        Ok(())
    }
    fn delete(&self, word_key: &str) -> anyhow::Result<bool> {
        Ok(self.items.lock().unwrap().remove(word_key).is_some())
    }
    fn list_recent_first(&self) -> anyhow::Result<Vec<WordBookItem>> {
        let mut v: Vec<WordBookItem> = self.items.lock().unwrap().values().cloned().collect();
        v.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(a.word_key.cmp(&b.word_key)));
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(key: &str) -> WordBookItem {
        WordBookItem {
            word_key: key.into(),
            headword: key.to_uppercase(),
            status: WordStatus::New,
            tags: vec!["gre".into()],
            note: String::new(),
            added_at: 100,
            updated_at: 100,
        }
    }

    #[test]
    fn add_dedups_by_key_and_merges_tags() {
        let wb = WordBook::new(InMemoryWordBookStore::default());
        assert!(wb.add(item("apple")).unwrap());
        let mut dup = item("apple");
        dup.tags = vec!["toefl".into()];
        assert!(!wb.add(dup).unwrap()); // 已存在 → 合并标签
        let got = wb.list().unwrap();
        assert_eq!(got.len(), 1);
        assert!(got[0].tags.contains(&"gre".to_string()));
        assert!(got[0].tags.contains(&"toefl".to_string()));
    }

    #[test]
    fn status_transition_any_direction() {
        let wb = WordBook::new(InMemoryWordBookStore::default());
        wb.add(item("apple")).unwrap();
        // 任意跳转：New → Mastered 直接跳
        wb.set_status("apple", WordStatus::Mastered, 200).unwrap();
        // 回退：Mastered → Learning
        wb.set_status("apple", WordStatus::Learning, 300).unwrap();
        let got = wb.store().get("apple").unwrap().unwrap();
        assert_eq!(got.status, WordStatus::Learning);
        assert_eq!(got.updated_at, 300);
    }

    #[test]
    fn remove_and_missing_status_error() {
        let wb = WordBook::new(InMemoryWordBookStore::default());
        wb.add(item("apple")).unwrap();
        assert!(wb.remove("apple").unwrap());
        assert!(!wb.remove("apple").unwrap());
        assert!(wb.set_status("apple", WordStatus::New, 1).is_err());
    }

    #[test]
    fn list_recent_first() {
        let wb = WordBook::new(InMemoryWordBookStore::default());
        wb.add(item("a")).unwrap();
        let mut b = item("b");
        b.updated_at = 999;
        wb.add(b).unwrap();
        let list = wb.list().unwrap();
        assert_eq!(list[0].word_key, "b");
    }

    #[test]
    fn status_roundtrip() {
        for s in WordStatus::all() {
            assert_eq!(WordStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(WordStatus::parse("bogus"), None);
    }
}
