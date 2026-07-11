import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { marked } from 'marked';
import type { Note, Reminder } from './types';
import { COLORS, escapeHtml, localISO, repeatLabel } from './utils';
import { initLocale, t, applyLocale } from './i18n';
import './styles.css';

initLocale();
applyLocale();
// 同步语言偏好到后端
invoke('set_locale', { locale: localStorage.getItem('locale') || 'zh' });

// ============ Markdown 渲染 ============

// 配置 marked
marked.setOptions({
  breaks: true,
  gfm: true,
});

// 渲染 Markdown 为 HTML，支持待办清单
function renderMarkdown(content: string): string {
  if (!content.trim()) {
    return `<span class="placeholder">${t('note.placeholder')}</span>`;
  }
  let html = marked.parse(content) as string;
  // 将 GFM task list 的 checkbox 美化
  html = html.replace(/<li><input[^>]*disabled[^>]*>\s*/g, '<li class="task-item">');
  html = html.replace(/<input type="checkbox"[^>]*>/g, '');
  return html;
}

// ============ 入口 ============

const win = getCurrentWindow();
const noteId = win.label.startsWith('note-') ? win.label.slice(5) : '';
// 检查 URL 参数：?reminder=1 表示由提醒触发弹出
const urlParams = new URLSearchParams(window.location.search);
const isReminder = urlParams.get('reminder') === '1';

if (noteId) {
  initNoteWindow(noteId);
} else {
  document.getElementById('app')!.innerHTML = `<div class="empty">${t('note.noSelection')}</div>`;
}

async function initNoteWindow(id: string) {
  try {
    const note = await invoke<Note>('get_note', { id });
    if (!note) {
      document.getElementById('app')!.innerHTML = `<div class="empty">${t('note.notExist')}</div>`;
      await win.show();
      return;
    }
    renderNote(note);
    setupWindowEvents(id);
    // 如果是提醒触发的，显示横幅
    if (isReminder) {
      const banner = document.querySelector('[data-reminder-banner]') as HTMLElement;
      if (banner) banner.style.display = 'flex';
      document.getElementById('app')!.classList.add('reminder-flash');
    }
    // 页面渲染完成后再显示窗口，避免白板闪烁
    await win.show();
  } catch (e) {
    console.error('加载便签失败:', e);
    document.getElementById('app')!.innerHTML = `<div class="empty">${t('note.loadFailed')}</div>`;
    await win.show();
  }

  // 监听闪烁事件：窗口已存在时被聚焦，加边框闪烁动画
  getCurrentWindow().listen('flash-window', () => {
    const app = document.getElementById('app')!;
    app.classList.add('flash-highlight');
    setTimeout(() => app.classList.remove('flash-highlight'), 1000);
  });

  // 监听提醒触发事件：窗口已存在时，后端发送此事件显示横幅
  getCurrentWindow().listen('reminder-triggered', () => {
    const app = document.getElementById('app')!;
    const banner = app.querySelector('[data-reminder-banner]') as HTMLElement;
    if (banner) {
      banner.style.display = 'flex';
      app.classList.add('reminder-flash');
    }
  });
}

// ============ 渲染 ============

function renderNote(note: Note) {
  const app = document.getElementById('app')!;
  app.innerHTML = `
    <div class="reminder-banner" data-reminder-banner style="display:none">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>
      <span>${t('note.reminderBanner')}</span>
      <button class="banner-close" data-banner-close>&times;</button>
    </div>
    <div class="title-bar" data-drag>
      <span class="title-text" data-title>${escapeHtml(note.title) || t('app.note')}</span>
	      <button class="icon-btn pin-btn ${note.is_pinned ? 'pinned' : ''}" data-pin title="${t('note.pin')}"></button>
	      <button class="icon-btn" data-close title="${t('note.close')}">&times;</button>
    </div>
    <div class="content-area">
      <div class="content-view" data-content-view>${renderMarkdown(note.content)}</div>
      <textarea class="content-edit" data-content style="display:none" placeholder="${t('note.placeholder')}" spellcheck="false">${escapeHtml(note.content)}</textarea>
    </div>
    <div class="bottom-bar">
      <div class="color-picker">
        ${Object.entries(COLORS).map(([name, c]) =>
          `<div class="color-dot ${note.color === name ? 'active' : ''}" data-color="${name}" style="background:${c.dot}"></div>`
        ).join('')}
      </div>
      <input type="range" class="opacity-slider" data-opacity min="0.3" max="1" step="0.05" value="${note.opacity}">
      <button class="icon-btn reminder-btn" data-reminder title="${t('note.setReminder')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg></button>
	      <button class="icon-btn archive-btn" data-archive title="${t('note.archive')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg></button>
	      <button class="icon-btn del-btn" data-delete title="${t('note.delete')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"/></svg></button>
    </div>
  `;

  applyNoteStyle(note);
  setupNoteEvents(note);

  // 关闭横幅按钮
  const banner = app.querySelector('[data-reminder-banner]') as HTMLElement;
  app.querySelector('[data-banner-close]')!.addEventListener('click', () => {
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
  });
}

function applyNoteStyle(note: Note) {
  const colors = COLORS[note.color] || COLORS.amber;
  const app = document.getElementById('app')!;
  app.style.backgroundColor = colors.bg(note.opacity);
}

// ============ 事件绑定 ============

function setupNoteEvents(note: Note) {
  const app = document.getElementById('app')!;

  // ---- 内容编辑：查看/编辑模式切换 ----
  const contentView = app.querySelector('[data-content-view]') as HTMLElement;
  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;

  // 点击查看区 → 进入编辑模式
  contentView.addEventListener('click', () => {
    contentView.style.display = 'none';
    textarea.style.display = 'block';
    textarea.focus();
    // 光标移到末尾
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
  });

  // 失焦 → 保存并切回查看模式
  textarea.addEventListener('blur', () => {
    const content = textarea.value;
    note.content = content;
    contentView.innerHTML = renderMarkdown(content);
    textarea.style.display = 'none';
    contentView.style.display = 'block';
    invoke('update_note_content', { id: note.id, content });
  });

  // Tab 键插入空格而非切换焦点
  textarea.addEventListener('keydown', (e) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      textarea.value = textarea.value.slice(0, start) + '  ' + textarea.value.slice(end);
      textarea.selectionStart = textarea.selectionEnd = start + 2;
    }
  });

  // ---- 标题双击编辑逻辑在下方拖拽处理中 ----

  // ---- 置顶切换 ----
  const pinBtn = app.querySelector('[data-pin]') as HTMLButtonElement;
  pinBtn.addEventListener('click', () => {
    note.is_pinned = !note.is_pinned;
    pinBtn.classList.toggle('pinned', note.is_pinned);
    invoke('update_note_style', {
      id: note.id, color: note.color, opacity: note.opacity, isPinned: note.is_pinned,
    });
  });

  // ---- 关闭窗口 ----
  app.querySelector('[data-close]')!.addEventListener('click', () => {
    win.close();
  });

  // ---- 颜色切换 ----
  app.querySelectorAll('[data-color]').forEach(dot => {
    dot.addEventListener('click', () => {
      const color = (dot as HTMLElement).dataset.color!;
      note.color = color;
      app.querySelectorAll('[data-color]').forEach(d => d.classList.remove('active'));
      dot.classList.add('active');
      applyNoteStyle(note);
      invoke('update_note_style', {
        id: note.id, color, opacity: note.opacity, isPinned: note.is_pinned,
      });
    });
  });

  // ---- 透明度滑块 ----
  const slider = app.querySelector('[data-opacity]') as HTMLInputElement;
  slider.addEventListener('input', () => {
    note.opacity = parseFloat(slider.value);
    applyNoteStyle(note);
  });
  slider.addEventListener('change', () => {
    invoke('update_note_style', {
      id: note.id, color: note.color, opacity: note.opacity, isPinned: note.is_pinned,
    });
  });

  // ---- 删除便签（自定义确认） ----
  app.querySelector('[data-delete]')!.addEventListener('click', () => {
    showDeleteConfirm(note.id, app);
  });

  // ---- 归档便签 ----
  app.querySelector('[data-archive]')!.addEventListener('click', async () => {
    try {
      await invoke('archive_note', { id: note.id });
      await win.close();
    } catch (e) {
      console.error('归档失败:', e);
    }
  });

  // ---- 提醒设置 ----
  app.querySelector('[data-reminder]')!.addEventListener('click', () => {
    showReminderPanel(note, app);
  });

  // ---- 窗口拖拽 + 标题双击编辑 ----
  const titleBar = app.querySelector('[data-drag]') as HTMLElement;
  const titleText = app.querySelector('[data-title]') as HTMLElement;
  let lastTitleClick = 0;

  titleBar.addEventListener('mousedown', (e) => {
    // 点击按钮（关闭/置顶）不处理
    if ((e.target as HTMLElement).closest('button')) return;

    // 检测是否点在标题文字上，且为双击
    const clickedTitle = (e.target as HTMLElement).closest('[data-title]');
    if (clickedTitle) {
      const now = Date.now();
      if (now - lastTitleClick < 500) {
        // 双击标题 → 编辑模式，不触发拖拽
        e.preventDefault();
        e.stopPropagation();
        enterTitleEdit(note, titleText, app);
        lastTitleClick = 0;
        return;
      }
      lastTitleClick = now;
    }

    // 单击 → 拖拽
    win.startDragging();
  });
}

// ============ 标题编辑 ============

function enterTitleEdit(note: Note, titleText: HTMLElement, _app: HTMLElement) {
  const input = document.createElement('input');
  input.type = 'text';
  input.value = note.title;
  input.className = 'title-input';
  input.placeholder = t('note.title');
  titleText.replaceWith(input);
  input.focus();
  input.select();

  const saveTitle = () => {
    note.title = input.value;
    invoke('update_note_title', { id: note.id, title: input.value });
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

// ============ 提醒面板 ============

function showReminderPanel(note: Note, app: HTMLElement) {
  if (app.querySelector('.reminder-overlay')) return;

  const overlay = document.createElement('div');
  overlay.className = 'reminder-overlay';

  // 默认提醒时间：1小时后
  const defaultTime = new Date(Date.now() + 3600000);

  overlay.innerHTML = `
    <div class="reminder-dialog">
      <div class="rd-header">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#6b7280" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>
        <span>${t('note.reminderMe')}</span>
        <button class="rd-close" data-reminder-close>&times;</button>
      </div>
      <div class="rd-quick">
        <button class="qbtn" data-quick="1h">${t('note.oneHour')}</button>
	        <button class="qbtn" data-quick="3h">${t('note.threeHours')}</button>
	        <button class="qbtn" data-quick="tomorrow">${t('note.tomorrow')}</button>
	        <button class="qbtn" data-quick="week">${t('note.nextMonday')}</button>
      </div>
      <input type="datetime-local" class="rd-datetime" data-remind-at value="${localISO(defaultTime)}">
      <div class="rd-repeat">
        <button class="rbtn active" data-repeat="none">${t('note.once')}</button>
	        <button class="rbtn" data-repeat="daily">${t('note.daily')}</button>
	        <button class="rbtn" data-repeat="weekly">${t('note.weekly')}</button>
	        <button class="rbtn" data-repeat="monthly">${t('note.monthly')}</button>
      </div>
      <div class="rd-existing" data-reminder-list></div>
      <button class="rd-save" data-save-reminder>${t('note.setReminder')}</button>
    </div>
  `;
  app.appendChild(overlay);

  // 加载已有提醒
  loadReminders(note.id, overlay);

  // 关闭
  overlay.querySelector('[data-reminder-close]')!.addEventListener('click', () => overlay.remove());
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) overlay.remove();
  });

  // 快捷时间按钮
  let selectedRepeat = 'none';
  overlay.querySelectorAll('[data-quick]').forEach(btn => {
    btn.addEventListener('click', () => {
      const input = overlay.querySelector('[data-remind-at]') as HTMLInputElement;
      const now = new Date();
      const type = (btn as HTMLElement).dataset.quick;
      if (type === '1h') {
        now.setHours(now.getHours() + 1);
      } else if (type === '3h') {
        now.setHours(now.getHours() + 3);
      } else if (type === 'tomorrow') {
        now.setDate(now.getDate() + 1);
        now.setHours(9, 0, 0, 0);
      } else if (type === 'week') {
        const day = now.getDay();
        const daysUntilMonday = day === 0 ? 1 : 8 - day;
        now.setDate(now.getDate() + daysUntilMonday);
        now.setHours(9, 0, 0, 0);
      }
      input.value = localISO(now);
    });
  });

  // 重复选择
  overlay.querySelectorAll('[data-repeat]').forEach(btn => {
    btn.addEventListener('click', () => {
      overlay.querySelectorAll('[data-repeat]').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      selectedRepeat = (btn as HTMLElement).dataset.repeat!;
    });
  });

  // 保存
  overlay.querySelector('[data-save-reminder]')!.addEventListener('click', async () => {
    const input = overlay.querySelector('[data-remind-at]') as HTMLInputElement;
    const dt = new Date(input.value);
    if (isNaN(dt.getTime())) return;
    const remindAt = dt.toISOString();
    try {
      await invoke('create_reminder', {
        noteId: note.id,
        noteTitle: note.title || t('app.note'),
        remindAt,
        repeatType: selectedRepeat,
      });
      overlay.remove();
    } catch (e) {
      console.error('创建提醒失败:', e);
    }
  });
}

async function loadReminders(noteId: string, container: HTMLElement) {
  try {
    const reminders = await invoke<Reminder[]>('get_reminders', { noteId });
    const list = container.querySelector('[data-reminder-list]') as HTMLElement;
    // 只显示未触发的提醒（pending 状态）
    const active = reminders.filter(r => r.status === 'pending');
    if (active.length === 0) {
      list.innerHTML = '';
      return;
    }
    // 折叠模式：默认只显示一行摘要，点击展开
    list.innerHTML = `<div class="rd-summary">${t('note.existingReminders', { n: active.length })} ▸</div>` +
      `<div class="rd-list" style="display:none">` +
      active.map(r => {
        const dt = new Date(r.remind_at).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
        const repeatText = r.repeat_type !== 'none' ? ` · ${repeatLabel(r.repeat_type)}` : '';
        return `<div class="reminder-item"><span>${dt}${repeatText}</span><button class="rd-del" data-del-id="${r.id}">&times;</button></div>`;
      }).join('') +
      `</div>`;

    // 点击摘要切换展开/折叠
    const summary = list.querySelector('.rd-summary') as HTMLElement;
    const detail = list.querySelector('.rd-list') as HTMLElement;
    summary.addEventListener('click', () => {
      const expanded = detail.style.display !== 'none';
      detail.style.display = expanded ? 'none' : 'block';
      summary.textContent = expanded
	        ? `${t('note.existingReminders', { n: active.length })} ▸`
	        : `${t('note.existingReminders', { n: active.length })} ▾`;
    });

    // 删除提醒
    list.querySelectorAll('[data-del-id]').forEach(btn => {
      btn.addEventListener('click', async (e) => {
        e.stopPropagation();
        const id = (btn as HTMLElement).dataset.delId!;
        try {
          await invoke('delete_reminder', { id });
          loadReminders(noteId, container);
        } catch (err) {
          console.error('删除提醒失败:', err);
        }
      });
    });
  } catch (e) {
    console.error('加载提醒失败:', e);
  }
}

// ============ 删除确认弹窗 ============

function showDeleteConfirm(noteId: string, app: HTMLElement) {
  // 已存在则跳过
  if (app.querySelector('.delete-overlay')) return;

  const overlay = document.createElement('div');
  overlay.className = 'delete-overlay';
  overlay.innerHTML = `
	    <div class="delete-dialog">
	      <p>${t('note.deleteConfirm')}</p>
	      <div class="delete-actions">
	        <button class="btn-cancel">${t('note.cancel')}</button>
	        <button class="btn-confirm">${t('note.deleteBtn')}</button>
	      </div>
	    </div>
	  `;
  app.appendChild(overlay);

  overlay.querySelector('.btn-cancel')!.addEventListener('click', () => overlay.remove());
  overlay.querySelector('.btn-confirm')!.addEventListener('click', () => {
    invoke('delete_note', { id: noteId });
    win.close();
  });
}

// ============ 窗口状态持久化 ============

function setupWindowEvents(id: string) {
  let saveTimeout: ReturnType<typeof setTimeout> | undefined;

  const saveWindowState = () => {
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      try {
        const pos = await win.outerPosition();
        const size = await win.outerSize();
        await invoke('update_note_window_state', {
          id,
          posX: pos.x,
          posY: pos.y,
          width: size.width,
          height: size.height,
        });
      } catch (e) {
        console.error('保存窗口状态失败:', e);
      }
    }, 500);
  };

  win.onMoved(() => saveWindowState());
  win.onResized(() => saveWindowState());
}

// ============ 工具函数 ============

// initNoteWindow 在模块加载时由前端入口调用
