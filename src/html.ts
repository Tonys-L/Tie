/**
 * HTML 字符串转义工具。
 *
 * 职责：仅提供 escapeHtml，无状态、无副作用。
 *
 * 被调用方：
 * - calendar-view.ts / note-renderer.ts / notes-list.ts / reminder-dialog.ts
 * - tag-bar.ts / template-manager.ts / template-ui.ts / ai-sniff.ts
 *
 * 依赖：无
 *
 * 设计目的：从原 utils.ts 拆出，让 HTML 转义有独立 seam，便于按需引入。
 */

/** HTML 转义：& < > 三字符替换为实体 */
export function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}
