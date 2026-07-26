/**
 * 待办清单 AI 排序：检测未完成待办 >3 时显示排序按钮，调用 AI 按紧急程度排序。
 *
 * 职责边界：
 * - extractTodoItems / applySortedTodos 纯文本处理（提取/替换待办行）
 * - setupTodoSortButton UI：检测条件 → 插入按钮 → 调用 api.aiSortTodos → 替换内容
 * - sortedNoteIds 集合记录已排序便签，避免重复显示按钮
 *
 * 被调用方：main.ts (bindTodoSort) + note-renderer.ts (renderNote 回调) + template-ui.ts (应用模板后清除排序标记)
 * 依赖：api.ts (aiSortTodos/updateNoteContent) + ai-client.ts (runAi 包装) +
 *       markdown-renderer.ts (renderMarkdown) + toast.ts (showToast) + i18n
 */

import type { Note } from './types';
import { t } from './i18n';
import { showToast } from './toast';
import { runAi } from './ai-client';
import * as api from './api';
import { renderMarkdown } from './markdown-renderer';

/** 已排序的便签 id 集合（内存级，内容变化时清除，避免重复显示排序按钮） */
const sortedNoteIds = new Set<string>();

/** 清除指定便签的已排序标记（内容被替换时调用，允许重新检测排序按钮） */
export function clearSortedMark(noteId: string): void {
  sortedNoteIds.delete(noteId);
}

/** 提取 content 中所有未完成待办条目（`- [ ]` / `* [ ]` / `+ [ ]`）的文本 */
export function extractTodoItems(content: string): string[] {
  return content
    .split('\n')
    .filter(line => /^\s*[-*+]\s+\[ \] /.test(line))
    .map(line => line.replace(/^\s*[-*+]\s+\[ \] /, ''));
}

/** 将排序后的条目按原顺序替换回 content 中的未完成待办行（保留原标记符和缩进） */
export function applySortedTodos(content: string, sortedItems: string[]): string {
  const lines = content.split('\n');
  let idx = 0;
  return lines
    .map(line => {
      const m = line.match(/^(\s*)([-*+]\s+)\[ \] (.*)$/);
      if (m) {
        const item = idx < sortedItems.length ? sortedItems[idx] : m[3];
        idx++;
        return `${m[1]}${m[2]}[ ] ${item}`;
      }
      return line;
    })
    .join('\n');
}

/** 检测待办条目 >3 且未排序时在 content-view 顶部显示 AI 排序按钮 */
export function setupTodoSortButton(note: Note, app: HTMLElement): void {
  const todos = extractTodoItems(note.content);
  if (todos.length <= 3) return;
  // 已排序的便签不再显示按钮（内容变化时清除标记）
  if (sortedNoteIds.has(note.id)) return;

  const contentView = app.querySelector('[data-content-view]') as HTMLElement;
  if (!contentView) return;

  // 避免重复插入
  if (contentView.querySelector('.todo-sort-btn')) return;

  const btn = document.createElement('button');
  btn.className = 'todo-sort-btn';
  btn.textContent = t('note.aiSortTodos');
  btn.style.cssText = 'display:block;margin:4px 0 8px;padding:4px 10px;font-size:11px;background:#3B82F6;color:#fff;border:none;border-radius:4px;cursor:pointer;';
  btn.addEventListener('click', async () => {
    btn.textContent = t('note.aiSorting');
    (btn as HTMLButtonElement).disabled = true;
    const sorted = await runAi(() => api.aiSortTodos(todos), {
      errorPrefix: t('note.aiFailed'),
    });
    if (sorted) {
      if (sorted.length !== todos.length) {
        showToast(t('note.aiSortMismatch'), 'error');
      } else {
        note.content = applySortedTodos(note.content, sorted);
        // 标记为已排序，排序后不重新显示按钮
        sortedNoteIds.add(note.id);
        // 更新编辑框和视图
        const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;
        if (textarea) textarea.value = note.content;
        contentView.innerHTML = renderMarkdown(note.content);
        // 自动保存
        api.updateNoteContent(note.id, note.content);
        showToast(t('note.aiSortDone'), 'success');
      }
    }
    btn.textContent = t('note.aiSortTodos');
    (btn as HTMLButtonElement).disabled = false;
  });
  contentView.insertBefore(btn, contentView.firstChild);
}
