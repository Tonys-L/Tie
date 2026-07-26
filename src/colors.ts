/**
 * 便签颜色配置 + 样式应用（吸收原 note-style.ts）。
 *
 * 职责：
 * - COLOR_MAP：Hub 列表用的颜色映射（颜色名 → hex）
 * - COLORS：便签窗口用的颜色配置（含 rgba 背景生成器 + dot 指示色）
 * - applyNoteStyle：应用颜色 + 透明度到 #app 元素（来自原 note-style.ts）
 *
 * 被调用方：
 * - notes-list.ts (COLOR_MAP)
 * - note-renderer.ts (COLORS, applyNoteStyle)
 * - context-menu.ts (applyNoteStyle)
 * - main.ts (applyNoteStyle)
 *
 * 依赖：types (Note)
 *
 * 设计目的：
 * - 从原 utils.ts 拆出 colors，让颜色配置有独立 seam
 * - 吸收原 note-style.ts，把"颜色配置 + 颜色应用"内聚到同一 module（locality）
 * - formatNoteTime 已迁移到 datetime.ts（职责错位修正：时间格式化不属于颜色模块）
 * - 不持有状态、不调用后端、不修改 note 对象
 */

import type { Note } from './types';

/** Hub 列表用的颜色映射 */
export const COLOR_MAP: Record<string, string> = {
  amber: '#fde047', blue: '#93c5fd', pink: '#f9a8d4', green: '#6ee7b7', white: '#e5e7eb', purple: '#c4b5fd',
};

/** 便签窗口用的颜色配置（含 rgba 背景生成器） */
export const COLORS: Record<string, { bg: (a: number) => string; dot: string }> = {
  amber:  { bg: (a) => `rgba(254, 249, 195, ${a})`, dot: '#fde047' },
  blue:   { bg: (a) => `rgba(219, 234, 254, ${a})`, dot: '#93c5fd' },
  pink:   { bg: (a) => `rgba(252, 231, 243, ${a})`, dot: '#f9a8d4' },
  green:  { bg: (a) => `rgba(209, 250, 229, ${a})`, dot: '#6ee7b7' },
  purple: { bg: (a) => `rgba(237, 233, 254, ${a})`, dot: '#c4b5fd' },
  white:  { bg: (a) => `rgba(255, 255, 255, ${a})`, dot: '#d1d5db' },
};

/** 批量改色面板用的颜色列表（颜色名 → 圆点 hex），单一来源，供 multiselect/context-menu 引用 */
export const BATCH_COLORS: Record<string, string> = Object.fromEntries(
  Object.entries(COLORS).map(([name, c]) => [name, c.dot])
);

/** 应用便签样式：颜色 + 透明度（自定义 hex 转 rgba） */
export function applyNoteStyle(note: Note): void {
  const app = document.getElementById('app')!;
  const colors = COLORS[note.color];
  if (colors) {
    app.style.backgroundColor = colors.bg(note.opacity);
  } else if (note.color.startsWith('#')) {
    // 自定义颜色：hex 转 rgba
    const r = parseInt(note.color.slice(1, 3), 16);
    const g = parseInt(note.color.slice(3, 5), 16);
    const b = parseInt(note.color.slice(5, 7), 16);
    app.style.backgroundColor = `rgba(${r}, ${g}, ${b}, ${note.opacity})`;
  } else {
    app.style.backgroundColor = COLORS.amber.bg(note.opacity);
  }
}
