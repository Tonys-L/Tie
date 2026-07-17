pub mod reminder;
pub mod report;
pub mod rewrite;
pub mod sniff;
pub mod sort;

// 重导出 ChatMessage，供 prompts 子模块统一使用
pub use crate::application::ai_service::ChatMessage;
