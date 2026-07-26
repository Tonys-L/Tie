/**
 * AI 文本重写：获取选中文本 → 调用后端 AI → 替换选中文本。
 *
 * 职责边界：
 * - getSelectedText 支持双模式（编辑模式 textarea 选区 / 查看模式 window.getSelection）
 * - rewriteText 显示 loading → 调用 api.aiRewriteText → 替换文本 → 显示提示
 * - 选中文本 < 5 字符返回 null（前端预检查，与后端校验对齐）
 *
 * 被调用方：context-menu.ts (右键菜单"AI 重写")
 * 依赖：api.ts (aiRewriteText/updateNoteContent) + ai-client.ts (runAi 包装) +
 *       markdown-renderer.ts (renderMarkdown) + i18n
 */

import type { Note } from './types';
import { t } from './i18n';
import { runAi } from './ai-client';
import * as api from './api';
import { renderMarkdown } from './markdown-renderer';

export type Selection = { text: string; replace: (newText: string) => void };

/**
 * 获取当前选中的文本及其替换函数。
 * - 编辑模式：通过 textarea.selectionStart/End 获取
 * - 查看模式：通过 window.getSelection() 获取
 * - 选中文本 trim 后长度 < 5 时返回 null（前端预检查，与后端校验对齐）
 */
export function getSelectedText(note: Note, app: HTMLElement): Selection | null {
  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;

  // 编辑模式：检查 textarea 选区
  if (textarea && textarea.style.display !== 'none') {
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    if (start !== end) {
      const text = textarea.value.substring(start, end);
      if (text.trim().length < 5) return null;
      return {
        text,
        replace: (newText: string) => {
          textarea.value = textarea.value.substring(0, start) + newText + textarea.value.substring(end);
          textarea.dispatchEvent(new Event('input'));
        }
      };
    }
  }

  // 查看模式：检查 window.getSelection()
  const selection = window.getSelection();
  if (selection && selection.toString().trim().length >= 5) {
    const text = selection.toString();
    return {
      text,
      replace: (newText: string) => {
        note.content = note.content.replace(text, newText);
        api.updateNoteContent(note.id, note.content);
        const contentView = app.querySelector('[data-content-view]') as HTMLElement;
        if (contentView) contentView.innerHTML = renderMarkdown(note.content);
      }
    };
  }

  return null;
}

/**
 * 调用后端 ai_rewrite_text 重写选中文本并替换。
 * 显示 loading → 调用后端 → 替换文本 → 显示结果提示（统一由 runAi 包装）。
 */
export async function rewriteText(selection: Selection, operation: string): Promise<void> {
  const result = await runAi(() => api.aiRewriteText(selection.text, operation), {
    loadingMsg: t('note.aiProcessing'),
    successMsg: t('note.aiReplaced'),
  });
  if (result) {
    selection.replace(result);
  }
}
