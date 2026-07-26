/**
 * 前端 IPC 事件名常量（候选5：消除跨层字符串耦合）。
 *
 * 职责：集中定义所有 listen 的事件名，与后端 event_names.rs 一一对应。
 * 改名时只需改 1 处 + grep 后端引用。
 *
 * 被调用方：
 * - main.ts：FLASH_WINDOW / REMINDER_TRIGGERED
 * - note-renderer.ts：REMINDER_CHANGED / NOTE_ARCHIVED / NOTE_UNARCHIVED / NOTE_COLOR_CHANGED
 * - ai-client.ts / ai-sniff.ts：AI_CONFIG_CHANGED
 */

/** AI 配置变更（重新加载配置） */
export const AI_CONFIG_CHANGED = 'ai-config-changed';

/** 提醒变更（刷新提醒图标状态） */
export const REMINDER_CHANGED = 'reminder-changed';

/** 便签归档（加蒙层变只读） */
export const NOTE_ARCHIVED = 'note-archived';

/** 便签恢复（移除蒙层） */
export const NOTE_UNARCHIVED = 'note-unarchived';

/** 便签颜色变更（同步颜色选中状态） */
export const NOTE_COLOR_CHANGED = 'note-color-changed';

/** 窗口闪烁（触发闪烁动画） */
export const FLASH_WINDOW = 'flash-window';

/** 提醒触发（显示提醒横幅） */
export const REMINDER_TRIGGERED = 'reminder-triggered';
