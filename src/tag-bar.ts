/**
 * 标签栏：渲染标签 pills + 添加/删除标签事件。
 *
 * 职责边界：
 * - renderTagPills 纯渲染（HTML 字符串）
 * - refreshTagBar 把 pills 写入 DOM
 * - setupTagEvents 绑定输入框回车添加 + × 删除事件，调用 api.updateNoteTags
 *
 * 被调用方：note-events.ts (setupNoteEvents 编排)、note-renderer.ts (renderNote 调用 refreshTagBar)
 * 依赖：api.ts (updateNoteTags) + utils.ts (escapeHtml) + types.ts (Note)
 */

import type { Note } from './types';
import { escapeHtml } from './utils';
import * as api from './api';

export function renderTagPills(tags: string[]): string {
  return tags.map(tag =>
    `<span class="tag-pill" data-tag="${escapeHtml(tag)}">${escapeHtml(tag)}<button class="tag-remove" data-tag-remove="${escapeHtml(tag)}">&times;</button></span>`
  ).join('');
}

export function refreshTagBar(note: Note): void {
  const tagList = document.querySelector('[data-tag-list]') as HTMLElement;
  if (tagList) tagList.innerHTML = renderTagPills(note.tags);
}

export function setupTagEvents(note: Note): void {
  const tagInput = document.querySelector('[data-tag-input]') as HTMLInputElement;
  const tagList = document.querySelector('[data-tag-list]') as HTMLElement;
  if (!tagInput || !tagList) return;

  // 回车或逗号添加标签
  tagInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      const val = tagInput.value.trim();
      if (val) {
        // 直接调用后端，domain 层处理去重和限制
        const newTags = [...note.tags, val];
        note.tags = newTags;
        refreshTagBar(note);
        tagInput.value = '';
        api.updateNoteTags(note.id, newTags);
      }
    }
  });

  // 点击标签的 × 删除
  tagList.addEventListener('click', (e) => {
    const removeBtn = (e.target as HTMLElement).closest('[data-tag-remove]') as HTMLElement;
    if (removeBtn) {
      e.stopPropagation();
      const tag = removeBtn.dataset.tagRemove!;
      note.tags = note.tags.filter(t => t !== tag);
      refreshTagBar(note);
      api.updateNoteTags(note.id, note.tags);
    }
  });
}
