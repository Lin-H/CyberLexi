//! 基础设施层：数据库 / 网络 / 文件 / 音频。同样禁止 GPUI 类型。

pub mod config;
pub mod db;
pub mod audio;
pub mod importer;
pub mod mw;
pub mod package;
pub mod paths;
pub mod tts;
