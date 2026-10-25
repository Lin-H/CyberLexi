//! Merriam-Webster Collegiate API 客户端、配额熔断与解析。

pub mod audio_url;
pub mod client;
pub mod markup;
pub mod parser;

pub use client::{MwClient, MwError, MwHttpClient};
pub use parser::{parse_body, parse_cached_entry, MwResponse};
