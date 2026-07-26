//! 后端→前端 IPC 事件名常量（候选5：消除跨层字符串耦合）
//!
//! 职责：集中定义所有 app.emit / app.emit_to 的事件名，避免字符串字面量散布。
//! 前端 src/events.ts 维护对应常量，改名时只需改 1 处 + grep 前端引用。
//!
//! 调用方：
//! - `commands/ai_commands.rs`：AI_CONFIG_CHANGED
//! - `commands/reminder_commands.rs`：REMINDER_CHANGED
//! - `commands/note_commands.rs`：NOTE_ARCHIVED / NOTE_UNARCHIVED / NOTE_COLOR_CHANGED
//! - `window_manager.rs`：FLASH_WINDOW / REMINDER_TRIGGERED

/// AI 配置变更（前端 ai-client.ts / ai-sniff.ts 监听，重新加载配置）
pub const AI_CONFIG_CHANGED: &str = "ai-config-changed";

/// 提醒变更（前端 note-renderer.ts 监听，刷新提醒图标状态）
pub const REMINDER_CHANGED: &str = "reminder-changed";

/// 便签归档（前端 note-renderer.ts 监听，加蒙层变只读）
pub const NOTE_ARCHIVED: &str = "note-archived";

/// 便签恢复（前端 note-renderer.ts 监听，移除蒙层）
pub const NOTE_UNARCHIVED: &str = "note-unarchived";

/// 便签颜色变更（前端 note-renderer.ts 监听，同步颜色选中状态）
pub const NOTE_COLOR_CHANGED: &str = "note-color-changed";

/// 窗口闪烁（前端 main.ts 监听，触发闪烁动画）
pub const FLASH_WINDOW: &str = "flash-window";

/// 提醒触发（前端 main.ts 监听，显示提醒横幅）
pub const REMINDER_TRIGGERED: &str = "reminder-triggered";
