/**
 * Markdown 渲染器：配置 marked + 自定义渲染（图片路径转换、待办清单 checkbox）。
 *
 * 职责边界：
 * - 只负责把 content 字符串渲染为 HTML 字符串
 * - 不绑定任何 DOM 事件，不操作便签状态
 *
 * 被调用方：note-renderer.ts (renderNote)、ai-sniff.ts (updateNoteContent)
 * 依赖：marked 库 + @tauri-apps/api/core (convertFileSrc) + api.ts (getImageDir)
 */

import { convertFileSrc } from '@tauri-apps/api/core';
import { marked } from 'marked';
import { t } from './i18n';
import * as api from './api';

// 配置 marked
marked.setOptions({
  breaks: true,
  gfm: true,
});

// 图片目录路径，启动时异步获取
let imageDir = '';
api.getImageDir().then(dir => { imageDir = dir; }).catch(() => {});

/**
 * 渲染 Markdown 为 HTML，支持待办清单和图片。
 * - img:filename{width=N} 语法 → 本地图片 + 可调整大小容器
 * - GFM task list → 可交互 checkbox（data-task-index 用于点击切换）
 * - 外部图片 URL → 同样包裹可调整大小容器
 */
export function renderMarkdown(content: string): string {
  if (!content.trim()) {
    return `<span class="placeholder">${t('note.placeholder')}</span>`;
  }
  let processed = content;
  if (imageDir) {
    // 将 img:filename{width=300} 或 img:filename 替换为 HTML img 标签
    // 必须在 marked 解析前处理，避免 {width=N} 被当作 URL 的一部分导致图片加载失败
    processed = processed.replace(/!\[([^\]]*)\]\(img:([^\s{}]+)(?:\{width=(\d+)\})?\)/g, (_match, alt: string, filename: string, width: string | undefined) => {
      const url = convertFileSrc(imageDir + '\\' + filename);
      if (width) {
        return `<span class="img-wrap" data-img-filename="${filename}" data-img-width="${width}" style="width:${width}px"><img src="${url}" alt="${alt}"><span class="img-resize-handle"></span></span>`;
      }
      return `<span class="img-wrap" data-img-filename="${filename}"><img src="${url}" alt="${alt}"><span class="img-resize-handle"></span></span>`;
    });
  }
  let html = marked.parse(processed) as string;
  // 美化 GFM task list：保留可交互 checkbox，添加 data-task-index 用于点击切换
  let taskIndex = 0;
  html = html.replace(
    /<li><input[^>]*type="checkbox"[^>]*>/g,
    (match: string) => {
      const checked = match.includes('checked');
      const idx = taskIndex++;
      return `<li class="task-item"><input type="checkbox" class="task-checkbox" data-task-index="${idx}" ${checked ? 'checked' : ''}>`;
    }
  );
  // 处理非 img: 开头的外部图片 URL（也包裹可调整大小容器）
  html = html.replace(/<img([^>]*)src="([^"]*)"([^>]*)>/g, (_match, before: string, src: string, after: string) => {
    // 已被 img-wrap 包裹的跳过
    if (before.includes('img-wrap') || after.includes('img-wrap') || src.startsWith('data:')) return _match;
    return `<span class="img-wrap"><img${before}src="${src}"${after}><span class="img-resize-handle"></span></span>`;
  });
  return html;
}
