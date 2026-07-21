/**
 * 标题编辑：点击标题进入输入框，blur/Enter 保存，Esc 取消。
 *
 * 职责边界：
 * - 输入框创建 + 光标定位末尾（不全选）
 * - 阻止 mousedown 冒泡到 titleBar 拖拽
 * - 归档便签不允许编辑（业务规则）
 *
 * 被调用方：note-events.ts (标题点击事件)
 * 依赖：api.ts (updateNoteTitle) + i18n + types.ts (Note)
 */

import type { Note } from './types';
import { t } from './i18n';
import * as api from './api';

export function enterTitleEdit(note: Note, titleText: HTMLElement, _app: HTMLElement): void {
  // 归档状态不允许编辑标题
  if (note.is_archived) return;
  const input = document.createElement('input');
  input.type = 'text';
  input.value = note.title;
  input.className = 'title-input';
  input.placeholder = t('note.title');
  titleText.replaceWith(input);
  input.focus();
  // 光标放在末尾，而非全选文字（方便在末尾追加内容）
  input.setSelectionRange(input.value.length, input.value.length);

  // 阻止 input 上的 mousedown 冒泡到 titleBar 触发拖拽（导致失焦退出编辑）
  input.addEventListener('mousedown', (e) => e.stopPropagation());

  const saveTitle = () => {
    note.title = input.value;
    api.updateNoteTitle(note.id, input.value);
    titleText.textContent = input.value || t('app.note');
    input.replaceWith(titleText);
  };

  input.addEventListener('blur', saveTitle);
  input.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter') input.blur();
    if (ev.key === 'Escape') {
      titleText.textContent = note.title || t('app.note');
      input.replaceWith(titleText);
    }
  });
}
