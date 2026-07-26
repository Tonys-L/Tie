/**
 * 便签视图入口：渲染整个便签窗口的 DOM + 监听后端事件同步状态。
 *
 * 职责边界：
 * - renderNote 生成便签 DOM 结构（标题栏 + 内容区 + 标签栏 + 底栏）
 * - 监听后端事件：reminder-changed / note-archived / note-unarchived / note-color-changed
 * - 横幅事件：关闭/贪睡/完成按钮
 * - 调用 setupEventsCallback 让 main.ts 编排其他 setup 函数（避免循环依赖）
 *
 * 不负责：applyNoteStyle（在 colors.ts）/ formatNoteTime（在 datetime.ts）
 *
 * 被调用方：main.ts (initNoteWindow)
 * 依赖：colors.ts (COLORS/applyNoteStyle) + datetime.ts (formatNoteTime) + tag-bar.ts + image-resize.ts +
 *       context-menu.ts (setupContextMenu) + ai-todo-sort.ts + template-ui.ts +
 *       markdown-renderer.ts + api.ts + html.ts (escapeHtml) + i18n + note-context.ts
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Note } from './types';
import { t } from './i18n';
import * as api from './api';
import { renderMarkdown } from './markdown-renderer';
import { escapeHtml } from './html';
import { COLORS, applyNoteStyle } from './colors';
import { formatNoteTime } from './datetime';
import { renderTagPills, setupTagEvents } from './tag-bar';
import { setupImageResize } from './image-resize';
import { setupContextMenu } from './context-menu';
import { setupTodoSortButton } from './ai-todo-sort';
import { setupTemplateQuickBar } from './template-ui';
import { getCurrentReminderId } from './note-context';
import { REMINDER_CHANGED, NOTE_ARCHIVED, NOTE_UNARCHIVED, NOTE_COLOR_CHANGED } from './events';

/**
 * 生成便签 DOM 结构（纯 DOM 生成，无事件绑定）。
 */
function renderNoteDom(note: Note): HTMLElement {
  const app = document.getElementById('app')!;
  app.innerHTML = `
    <div class="reminder-banner" data-reminder-banner style="display:none">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>
      <span>${t('note.reminderBanner')}</span>
      <button class="banner-action" data-banner-snooze>${t('note.snooze')}</button>
      <button class="banner-action" data-banner-done>${t('note.done')}</button>
      <button class="banner-close" data-banner-close>&times;</button>
    </div>
    <div class="title-bar${note.is_archived ? ' is-archived' : ''}" data-drag>
	      <span class="title-text" data-title>${escapeHtml(note.title) || t('app.note')}</span>
	      ${note.is_archived ? `<span class="archived-badge">${t('hub.archived')}</span>` : ''}
	      <span class="title-time">${formatNoteTime(note.created_at)}</span>
		      <button class="icon-btn pin-btn ${note.is_pinned ? 'pinned' : ''}" data-pin title="${t('note.pin')}"></button>
		      <button class="icon-btn" data-close title="${t('note.close')}">&times;</button>
	    </div>
    <div class="content-area${note.is_archived ? ' is-archived' : ''}">
	      <div class="content-view" data-content-view>${renderMarkdown(note.content)}</div>
	      <textarea class="content-edit" data-content style="display:none" placeholder="${t('note.placeholder')}" spellcheck="false">${escapeHtml(note.content)}</textarea>
	      ${note.is_archived ? '<div class="archived-overlay"></div>' : ''}
	    </div>
    <div class="tag-bar" data-tag-bar>
      <div class="tag-list" data-tag-list>${renderTagPills(note.tags)}</div>
      <input class="tag-input" data-tag-input placeholder="${t('note.tagPlaceholder')}" maxlength="20">
    </div>
    <div class="bottom-bar">
      <div class="color-picker">
        ${Object.entries(COLORS).map(([name, c]) =>
          `<div class="color-dot ${note.color === name ? 'active' : ''}" data-color="${name}" style="background:${c.dot}"></div>`
        ).join('')}
        <div class="color-dot custom-color-dot ${note.color.startsWith('#') ? 'active' : ''}" data-custom-color title="${t('note.customColor')}"></div>
      </div>
      <input type="range" class="opacity-slider" data-opacity min="0.3" max="1" step="0.05" value="${note.opacity}">
      <button class="icon-btn ai-btn" data-ai-sniff title="${t('hub.aiAssistant')}" disabled><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18h6"/><path d="M10 22h4"/><path d="M12 2a7 7 0 0 0-4 12.7c.6.5 1 1.2 1 2v1.3h6V16.7c0-.8.4-1.5 1-2A7 7 0 0 0 12 2z"/></svg></button>
      <button class="icon-btn reminder-btn" data-reminder title="${t('note.setReminder')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg></button>
	      <button class="icon-btn archive-btn" data-archive title="${note.is_archived ? t('hub.restore') : t('note.archive')}">${note.is_archived
          ? '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="17 1 21 5 17 9"/><path d="M3 11V9a4 4 0 0 1 4-4h14"/><polyline points="7 23 3 19 7 15"/><path d="M21 13v2a4 4 0 0 1-4 4H3"/></svg>'
          : '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>'
        }</button>
	      <button class="icon-btn del-btn" data-delete title="${t('note.delete')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"/></svg></button>
    </div>
  `;
  return app;
}

/**
 * 设置后端事件监听：reminder-changed / note-archived / note-unarchived / note-color-changed。
 *
 * 无论谁触发（手动创建/AI创建/删除/贪睡/关闭），UI 都能自动同步。
 */
function setupNoteEventListeners(note: Note, app: HTMLElement): void {
  // 检查是否有活跃提醒，有则给提醒按钮添加 has-reminder class（图标变橙色）
  function refreshReminderIcon(): void {
    api.getReminders(note.id)
      .then(reminders => {
        const btn = app.querySelector('.reminder-btn');
        if (!btn) return;
        const hasActive = reminders.some(r => r.status === 'pending');
        btn.classList.toggle('has-reminder', hasActive);
      })
      .catch(() => {});
  }
  refreshReminderIcon();

  // 监听后端 reminder-changed 事件，统一更新提醒图标状态
  getCurrentWindow().listen<string>(REMINDER_CHANGED, (event) => {
    if (event.payload === note.id) {
      refreshReminderIcon();
    }
  });

  // 监听后端 note-archived 事件：Hub 归档便签时，已打开的窗口加蒙层变只读
  getCurrentWindow().listen<string>(NOTE_ARCHIVED, (event) => {
    if (event.payload !== note.id) return;
    note.is_archived = true;
    // 添加蒙层
    if (!app.querySelector('.archived-overlay')) {
      const overlay = document.createElement('div');
      overlay.className = 'archived-overlay';
      const contentArea = app.querySelector('.content-area');
      if (contentArea) contentArea.appendChild(overlay);
    }
    // 添加归档样式
    const contentArea = app.querySelector('.content-area');
    if (contentArea) contentArea.classList.add('is-archived');
    const titleBarEl = app.querySelector('.title-bar');
    if (titleBarEl) titleBarEl.classList.add('is-archived');
    // 添加已归档标签（如不存在）
    if (!app.querySelector('.archived-badge')) {
      const badge = document.createElement('span');
      badge.className = 'archived-badge';
      badge.textContent = t('hub.archived');
      titleBarEl?.appendChild(badge);
    }
    // 更新归档按钮为恢复按钮
    const btn = app.querySelector('[data-archive]') as HTMLButtonElement;
    if (btn) {
      btn.title = t('note.restore');
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>';
    }
  });

  // 监听后端 note-unarchived 事件：Hub 恢复便签时，已打开的窗口移除蒙层
  getCurrentWindow().listen<string>(NOTE_UNARCHIVED, (event) => {
    if (event.payload !== note.id) return;
    note.is_archived = false;
    // 移除蒙层
    const overlay = app.querySelector('.archived-overlay');
    if (overlay) overlay.remove();
    // 移除归档样式
    const contentArea = app.querySelector('.content-area');
    if (contentArea) contentArea.classList.remove('is-archived');
    const titleBarEl = app.querySelector('.title-bar');
    if (titleBarEl) titleBarEl.classList.remove('is-archived');
    // 移除已归档标签
    const badge = app.querySelector('.archived-badge');
    if (badge) badge.remove();
    // 更新按钮为归档按钮
    const btn = app.querySelector('[data-archive]') as HTMLButtonElement;
    if (btn) {
      btn.title = t('note.archive');
      btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>';
    }
  });

  // 监听后端 note-color-changed 事件：Hub 批量改色时，已打开的窗口同步更新
  getCurrentWindow().listen<{id: string, color: string}>(NOTE_COLOR_CHANGED, (event) => {
    if (event.payload.id !== note.id) return;
    note.color = event.payload.color;
    applyNoteStyle(note);
    // 更新颜色选中状态
    app.querySelectorAll('.color-dot').forEach(d => {
      d.classList.toggle('active', (d as HTMLElement).dataset.color === note.color);
    });
  });
}

/**
 * 设置提醒横幅按钮事件：关闭/贪睡/完成。
 */
function setupReminderBanner(note: Note, app: HTMLElement): void {
  const banner = app.querySelector('[data-reminder-banner]') as HTMLElement;
  // 关闭横幅按钮
  app.querySelector('[data-banner-close]')!.addEventListener('click', () => {
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
    api.restoreWindowOnTop(note.id);
  });
  // 贪睡按钮：5分钟后再次提醒
  app.querySelector('[data-banner-snooze]')!.addEventListener('click', async () => {
    const reminderId = getCurrentReminderId();
    if (reminderId) {
      try { await api.snoozeReminder(reminderId, 5); } catch (e) { console.error('贪睡失败:', e); }
    }
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
    api.restoreWindowOnTop(note.id);
  });
  // 完成按钮：标记提醒为已完成
  app.querySelector('[data-banner-done]')!.addEventListener('click', async () => {
    const reminderId = getCurrentReminderId();
    if (reminderId) {
      try { await api.dismissReminder(reminderId); } catch (e) { console.error('完成提醒失败:', e); }
    }
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
    api.restoreWindowOnTop(note.id);
  });
}

/**
 * 渲染便签窗口（编排函数）。
 *
 * 依次调用：renderNoteDom → applyNoteStyle → setupEventsCallback → setupTagEvents →
 * setupImageResize → setupNoteEventListeners → setupReminderBanner →
 * setupContextMenu → setupTodoSortButton → setupTemplateQuickBar
 *
 * @param setupEventsCallback 由 main.ts 提供，用于编排需要 main.ts 状态的事件绑定
 */
export function renderNote(note: Note, setupEventsCallback: (note: Note, app: HTMLElement) => void): void {
  const app = renderNoteDom(note);
  applyNoteStyle(note);
  setupEventsCallback(note, app);
  setupTagEvents(note);
  setupImageResize(note);
  setupNoteEventListeners(note, app);
  setupReminderBanner(note, app);
  setupContextMenu(note, app);
  setupTodoSortButton(note, app);
  setupTemplateQuickBar(note, app);
}
