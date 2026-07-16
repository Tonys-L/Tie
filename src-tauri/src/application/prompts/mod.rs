pub mod reminder;
pub mod sniff;

// 重导出 ChatMessage，供 prompts 子模块统一使用
pub use crate::application::ai_service::ChatMessage;
