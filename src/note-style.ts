/**
 * 便签样式应用 + 时间格式化（纯展示逻辑，无副作用）。
 *
 * 职责边界：
 * - applyNoteStyle 应用颜色 + 透明度到 #app 元素
 * - formatNoteTime 格式化创建时间为简短显示
 * - 不持有状态、不调用后端、不修改 note 对象
 *
 * 被调用方：note-renderer.ts (renderNote) + context-menu.ts (applyCustomColor) + main.ts (颜色/透明度滑块)
 * 依赖：utils.ts (COLORS) + types (Note)
 *
 * 设计目的：把 applyNoteStyle 从 note-renderer.ts 拆出，打破 note-renderer ↔ context-menu 循环依赖。
 */

import type { Note } from './types';
import { COLORS } from './utils';

/** 格式化便签创建时间为简短显示（如 "7/17 10:30"） */
export function formatNoteTime(iso: string): string {
  const d = new Date(iso);
  const locale = localStorage.getItem('locale') || 'zh';
  const month = d.getMonth() + 1;
  const day = d.getDate();
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  return locale === 'zh' ? `${month}/${day} ${hh}:${mm}` : `${month}/${day} ${hh}:${mm}`;
}

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
