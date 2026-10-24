//! CyberLexi core：domain + infrastructure + 编排 + 后台 worker。
//!
//! 硬约束：本 crate 不得依赖 GPUI/gpui-kit，全部使用纯 Rust 类型 +
//! `anyhow::Result`，保证 `cargo test -p lexi-core` 可独立跑通。
//! 网络 / 数据库 / 文件 IO 一律由 worker（独立线程内 tokio 运行时）执行，
//! UI 侧只通过 [`crate::events`] 的 `UiCommand` / `BackendEvent` 交互。

pub mod domain;
pub mod events;
pub mod infra;
pub mod orchestrator;
pub mod worker;

pub use orchestrator::QueryOrchestrator;
