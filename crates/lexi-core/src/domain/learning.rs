//! Phase 2 占位：学习计划 / 复习算法 / 学习统计。
//!
//! 硬约束：Phase 1 一律不实现业务逻辑。本模块只保留空 trait 与占位表
//!（见 migrations/0003_phase2_placeholder.sql），保证 Phase 2 可无痛插入。

use super::wordbook::WordBookItem;

/// 复习调度器（SM-2 类算法）占位。Phase 2 实现。
pub trait ReviewScheduler: Send + Sync {
    /// 给定词条与历史，返回下次应复习时间；Phase 1 恒返回 None。
    fn next_review(&self, _item: &WordBookItem) -> Option<std::time::SystemTime> {
        None
    }
}

/// Phase 1 的空实现。
pub struct NoopScheduler;

impl ReviewScheduler for NoopScheduler {}
