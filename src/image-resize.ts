/**
 * 图片拖拽调整大小：拖动右下角手柄改变宽度，松开后将宽度写回 Markdown。
 *
 * 职责边界：
 * - mousedown 监听 .img-resize-handle → mousemove 实时调整宽度
 * - mouseup 时调用 saveImageWidth 把 ![](img:filename) 改写为 ![](img:filename{width=N})
 * - 同步 textarea 值，避免编辑模式切换时丢失宽度
 *
 * 被调用方：note-events.ts (setupNoteEvents 编排)
 * 依赖：api.ts (updateNoteContent) + types.ts (Note)
 */

import type { Note } from './types';
import * as api from './api';

export function setupImageResize(note: Note): void {
  const contentView = document.querySelector('[data-content-view]') as HTMLElement;
  if (!contentView) return;

  contentView.addEventListener('mousedown', (e) => {
    const handle = (e.target as HTMLElement).closest('.img-resize-handle') as HTMLElement;
    if (!handle) return;

    e.preventDefault();
    e.stopPropagation();

    const wrap = handle.closest('.img-wrap') as HTMLElement;
    if (!wrap) return;

    const startX = e.clientX;
    const startWidth = wrap.offsetWidth;

    const onMove = (ev: MouseEvent) => {
      const dx = ev.clientX - startX;
      const newWidth = Math.max(60, Math.min(startWidth + dx, contentView.offsetWidth - 8));
      wrap.style.width = newWidth + 'px';
    };

    const onUp = () => {
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
      const finalWidth = Math.round(wrap.offsetWidth);
      const filename = wrap.dataset.imgFilename;
      if (filename) {
        saveImageWidth(note, filename, finalWidth);
      }
    };

    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  });
}

/** 将图片宽度保存到 Markdown 内容中：![](img:filename) → ![](img:filename{width=N}) */
function saveImageWidth(note: Note, filename: string, width: number): void {
  const escaped = filename.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // 已有 {width=N}：替换
  const withWidthRegex = new RegExp(`\\(img:${escaped}\\{width=\\d+\\}\\)`);
  // 无 {width=N}：追加
  const withoutWidthRegex = new RegExp(`\\(img:${escaped}\\)`);

  const before = note.content;
  if (withWidthRegex.test(note.content)) {
    note.content = note.content.replace(withWidthRegex, `(img:${filename}{width=${width}})`);
  } else if (withoutWidthRegex.test(note.content)) {
    note.content = note.content.replace(withoutWidthRegex, `(img:${filename}{width=${width}})`);
  }

  // 调试：确认内容被修改
  if (note.content === before) {
    console.warn('[img-resize] 内容未修改，filename=', filename, 'width=', width, 'content snippet=', before.substring(0, 200));
  } else {
    console.log('[img-resize] 内容已修改，filename=', filename, 'width=', width);
  }

  // 同步 textarea 值，避免编辑模式切换时丢失宽度
  const textarea = document.querySelector('[data-content]') as HTMLTextAreaElement;
  if (textarea) textarea.value = note.content;

  api.updateNoteContent(note.id, note.content);
}
