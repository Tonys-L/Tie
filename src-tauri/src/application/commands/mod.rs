//! Tauri 命令入口模块。
//!
//! 设计意图：所有 `#[tauri::command]` 集中在本模块下，按业务域拆分为子模块。
//! 本文件仅作为门面（facade），通过 `pub use *` glob 重导出所有命令及其
//! `#[tauri::command]` 宏生成的 `__cmd__xxx` 辅助项（这些项是 pub 但 doc-hidden），
//! 使外部调用方（`lib.rs` 的 `invoke_handler`）仍可用 `commands::xxx` 路径访问，
//! 不感知文件物理拆分。
//!
//! 拆分原因：原 `commands.rs` 单文件 814 行，违反单一职责。
//! 拆分后每个子模块 50~200 行，按业务能力聚合。
//!
//! 注意：必须用 `pub use mod::*;` glob，不能用显式列表，否则 `__cmd__xxx` 辅助项
//! 无法重导出，`tauri::generate_handler!` 会找不到命令。

pub mod ai_commands;
pub mod image_commands;
pub mod note_commands;
pub mod reminder_commands;
pub mod sync_commands;
pub mod template_commands;

// glob 重导出所有 pub 项（含 #[tauri::command] 生成的 __cmd__xxx 辅助函数）
pub use ai_commands::*;
pub use image_commands::*;
pub use note_commands::*;
pub use reminder_commands::*;
pub use sync_commands::*;
pub use template_commands::*;
