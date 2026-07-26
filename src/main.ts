/**
 * 便签窗口入口（note-xxx 窗口）：加载便签 → 渲染 → 绑定事件 → 监听后端联动。
 *
 * 职责边界：
 * - initNoteWindow 编排：拉取 note → setNote → renderNote → setupWindowEvents
 * - setupNoteEvents 编排 5 个 bind 子函数（按 UI 部件分组）
 * - 监听 flash-window / reminder-triggered 全局事件
 *
 * 不负责：具体业务实现（已按 UI 部件拆分到独立模块，见下方 imports）
 *
 * 被调用方：HTML 入口（note.html）加载本模块
 * 依赖：note-renderer (renderNote) + colors (applyNoteStyle) + context-menu (showCustomColorPanel) +
 *       reminder-panel (showReminderPanel) + ai-sniff (setupAiSniffButton/sniffAfterSave) +
 *       ai-todo-sort (setupTodoSortButton/clearSortedMark) +
 *       window-state + delete-confirm + title-edit + markdown-renderer + note-context + api + i18n
 */

import { getCurrentWindow } from '@tauri-apps/api/window';

// ===== 共享模块 =====
import type { Note } from './types';
import { initLocale, t, applyLocale } from './i18n';
import * as api from './api';
import { renderMarkdown } from './markdown-renderer';
import './styles.css';

// ===== 便签基础：state / 窗口 / 渲染 =====
import { setNote, setCurrentReminderId } from './note-context';
import { setupWindowEvents, setClosing } from './window-state';
import { applyNoteStyle } from './colors';
import { renderNote } from './note-renderer';

// ===== 便签 UI 部件 =====
import { showCustomColorPanel } from './context-menu';
import { enterTitleEdit } from './title-edit';
import { showDeleteConfirm } from './delete-confirm';
import { showReminderPanel } from './reminder-panel';

// ===== AI 能力 =====
import { setupAiSniffButton, sniffAfterSave } from './ai-sniff';
import { setupTodoSortButton, clearSortedMark } from './ai-todo-sort';
import { FLASH_WINDOW, REMINDER_TRIGGERED } from './events';

initLocale();
applyLocale();
// 同步语言偏好到后端
api.setLocale(localStorage.getItem('locale') || 'zh');

// ============ 入口 ============

const win = getCurrentWindow();
const noteId = win.label.startsWith('note-') ? win.label.slice(5) : '';
// 检查 URL 参数：?reminder=1 表示由提醒触发弹出
const urlParams = new URLSearchParams(window.location.search);
const isReminder = urlParams.get('reminder') === '1';
const urlReminderId = urlParams.get('rid') || '';
setCurrentReminderId(urlReminderId);

if (noteId) {
  initNoteWindow(noteId);
} else {
  document.getElementById('app')!.innerHTML = `<div class="empty">${t('note.noSelection')}</div>`;
}

async function initNoteWindow(id: string) {
  try {
    const note = await api.getNote(id);
    if (!note) {
      document.getElementById('app')!.innerHTML = `<div class="empty">${t('note.notExist')}</div>`;
      await win.show();
      return;
    }
    setNote(note);
    // setupNoteEvents 通过 callback 传入 renderNote，避免循环依赖
    renderNote(note, setupNoteEvents);
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
  getCurrentWindow().listen(FLASH_WINDOW, () => {
    const app = document.getElementById('app')!;
    app.classList.add('flash-highlight');
    setTimeout(() => app.classList.remove('flash-highlight'), 5100);
  });

  // 监听提醒触发事件：窗口已存在时，后端发送此事件显示横幅
  getCurrentWindow().listen(REMINDER_TRIGGERED, (event) => {
    const payload = event.payload as { reminder_id: string };
    setCurrentReminderId(payload.reminder_id);
    const app = document.getElementById('app')!;
    const banner = app.querySelector('[data-reminder-banner]') as HTMLElement;
    if (banner) {
      banner.style.display = 'flex';
      app.classList.add('reminder-flash');
    }
  });
}

// ============ 事件绑定 ============

/**
 * 绑定便签内所有 UI 事件（编排函数，仅调用子绑定函数）。
 * 由 renderNote 通过 callback 调用，此时 DOM 已生成。
 *
 * 拆分原则：按 UI 部件 / 交互类型分组，每个子函数 < 100 行，便于 AI 局部阅读。
 */
function setupNoteEvents(note: Note, app: HTMLElement): void {
  bindContentEdit(note, app);
  bindShortcutsAndClipboard(app);
  bindToolbar(note, app);
  bindReminderAndAi(note, app);
  bindTitleDrag(note, app);
}

/**
 * 内容编辑：查看↔编辑模式切换 + checkbox 切换 + 链接拦截 + 失焦保存。
 */
function bindContentEdit(note: Note, app: HTMLElement): void {
  const contentView = app.querySelector('[data-content-view]') as HTMLElement;
  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;

  // 点击查看区 → 进入编辑模式（链接和 checkbox 除外）
  contentView.addEventListener('click', (e) => {
    // 拦截 checkbox 点击：切换待办状态，不进入编辑模式
    const checkbox = (e.target as HTMLElement).closest('.task-checkbox') as HTMLInputElement;
    if (checkbox) {
      e.preventDefault();
      e.stopPropagation();
      const idx = parseInt(checkbox.dataset.taskIndex || '0');
      // 在 content 中找到第 idx 个 task list 行并切换 [ ] ↔ [x]
      const lines = note.content.split('\n');
      let count = 0;
      for (let i = 0; i < lines.length; i++) {
        const m = lines[i].match(/^(\s*[-*+]\s+)\[([ x])\]/);
        if (m) {
          if (count === idx) {
            const isChecked = m[2] === 'x';
            lines[i] = lines[i].replace(/\[[ x]\]/, isChecked ? '[ ]' : '[x]');
            break;
          }
          count++;
        }
      }
      note.content = lines.join('\n');
      textarea.value = note.content;
      contentView.innerHTML = renderMarkdown(note.content);
      clearSortedMark(note.id);
      setupTodoSortButton(note, app);
      api.updateNoteContent(note.id, note.content);
      return;
    }
    // 拦截链接点击：在系统浏览器打开
    const link = (e.target as HTMLElement).closest('a');
    if (link) {
      e.preventDefault();
      const href = link.getAttribute('href');
      if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
        api.openUrl(href).catch(err => console.error('打开链接失败:', err));
      }
      return;
    }
    // 拦截图片容器/手柄点击
    if ((e.target as HTMLElement).closest('.img-wrap, .img-resize-handle')) return;
    // 归档状态不允许编辑
    if (note.is_archived) return;
    contentView.style.display = 'none';
    textarea.style.display = 'block';
    textarea.focus();
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
  });

  // 失焦 → 保存并切回查看模式
  textarea.addEventListener('blur', () => {
    const content = textarea.value;
    if (content !== note.content) clearSortedMark(note.id);
    note.content = content;
    contentView.innerHTML = renderMarkdown(content);
    textarea.style.display = 'none';
    contentView.style.display = 'block';
    api.updateNoteContent(note.id, content);
    setupTodoSortButton(note, app);
    sniffAfterSave(note);
  });
}

/**
 * 快捷键（Ctrl+S/Esc/Ctrl+N/Tab）+ 粘贴图片 + 拖拽图片。
 */
function bindShortcutsAndClipboard(app: HTMLElement): void {
  const contentView = app.querySelector('[data-content-view]') as HTMLElement;
  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;

  // 全局快捷键
  document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      if (textarea.style.display !== 'none') textarea.blur();
    }
    if (e.key === 'Escape' && textarea.style.display !== 'none') {
      e.preventDefault();
      textarea.blur();
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      api.createNote();
    }
  });

  // Tab 键插入空格
  textarea.addEventListener('keydown', (e) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      textarea.value = textarea.value.slice(0, start) + '  ' + textarea.value.slice(end);
      textarea.selectionStart = textarea.selectionEnd = start + 2;
    }
  });

  // 粘贴图片：保存为文件，插入 Markdown 引用
  textarea.addEventListener('paste', (e) => {
    const items = e.clipboardData?.items;
    if (!items) return;
    for (const item of items) {
      if (item.type.startsWith('image/')) {
        e.preventDefault();
        const blob = item.getAsFile();
        if (!blob) continue;
        const ext = blob.type.split('/')[1] || 'png';
        blob.arrayBuffer().then(async (buffer) => {
          const data = Array.from(new Uint8Array(buffer));
          const filename = await api.saveImage(data, ext);
          const md = `![](img:${filename})`;
          const start = textarea.selectionStart;
          const end = textarea.selectionEnd;
          textarea.value = textarea.value.slice(0, start) + md + textarea.value.slice(end);
          textarea.selectionStart = textarea.selectionEnd = start + md.length;
          textarea.dispatchEvent(new Event('input'));
        });
        return;
      }
    }
  });

  // 拖拽图片文件：保存为文件，插入 Markdown 引用
  const handleDrop = (e: DragEvent) => {
    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) return;
    const file = files[0];
    if (!file.type.startsWith('image/')) return;
    e.preventDefault();
    const ext = file.name.split('.').pop()?.toLowerCase() || file.type.split('/')[1] || 'png';
    file.arrayBuffer().then(async (buffer) => {
      const data = Array.from(new Uint8Array(buffer));
      const filename = await api.saveImage(data, ext);
      const md = `![](img:${filename})`;
      if (textarea.style.display === 'none') {
        contentView.style.display = 'none';
        textarea.style.display = 'block';
      }
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      textarea.value = textarea.value.slice(0, start) + md + textarea.value.slice(end);
      textarea.selectionStart = textarea.selectionEnd = start + md.length;
      textarea.focus();
    });
  };
  const preventDragOver = (e: DragEvent) => e.preventDefault();
  textarea.addEventListener('drop', handleDrop);
  textarea.addEventListener('dragover', preventDragOver);
  contentView.addEventListener('drop', handleDrop);
  contentView.addEventListener('dragover', preventDragOver);
}

/**
 * 工具栏：置顶/关闭/颜色/自定义颜色/透明度/删除/归档。
 */
function bindToolbar(note: Note, app: HTMLElement): void {
  // 置顶切换
  const pinBtn = app.querySelector('[data-pin]') as HTMLButtonElement;
  pinBtn.addEventListener('click', () => {
    note.is_pinned = !note.is_pinned;
    pinBtn.classList.toggle('pinned', note.is_pinned);
    api.updateNoteStyle(note.id, note.color, note.opacity, note.is_pinned);
  });

  // 关闭窗口：若处于编辑模式，先保存内容再关闭
  // 避免 close_note_if_empty 检查到空内容导致便签被误删除（INV-003 竞态）
  app.querySelector('[data-close]')!.addEventListener('click', async () => {
    const textareaEl = app.querySelector('[data-content]') as HTMLTextAreaElement;
    const contentViewEl = app.querySelector('[data-content-view]') as HTMLElement;
    if (textareaEl.style.display !== 'none') {
      const content = textareaEl.value;
      if (content !== note.content) {
        note.content = content;
        try {
          await api.updateNoteContent(note.id, content);
        } catch (e) {
          console.error('保存便签失败:', e);
        }
      }
      textareaEl.style.display = 'none';
      contentViewEl.style.display = 'block';
      contentViewEl.innerHTML = renderMarkdown(content);
    }
    setClosing(true);
    win.close();
  });

  // 颜色切换
  app.querySelectorAll('[data-color]').forEach(dot => {
    dot.addEventListener('click', () => {
      const color = (dot as HTMLElement).dataset.color!;
      note.color = color;
      app.querySelectorAll('.color-dot').forEach(d => d.classList.remove('active'));
      dot.classList.add('active');
      applyNoteStyle(note);
      api.updateNoteStyle(note.id, color, note.opacity, note.is_pinned);
    });
  });

  // 自定义颜色：点击圆点弹出颜色面板
  const customDot = app.querySelector('[data-custom-color]') as HTMLElement;
  if (customDot) {
    customDot.addEventListener('click', (e) => {
      e.stopPropagation();
      showCustomColorPanel(note, app, customDot);
    });
  }

  // 透明度滑块
  const slider = app.querySelector('[data-opacity]') as HTMLInputElement;
  slider.addEventListener('input', () => {
    note.opacity = parseFloat(slider.value);
    applyNoteStyle(note);
  });
  slider.addEventListener('change', () => {
    api.updateNoteStyle(note.id, note.color, note.opacity, note.is_pinned);
  });

  // 删除便签（自定义确认）
  app.querySelector('[data-delete]')!.addEventListener('click', () => {
    showDeleteConfirm(note.id, app);
  });

  // 归档/恢复便签
  app.querySelector('[data-archive]')!.addEventListener('click', async () => {
    try {
      if (note.is_archived) {
        await api.unarchiveNote(note.id);
        note.is_archived = false;
        app.querySelector('.archived-overlay')?.remove();
        app.querySelector('.content-area')?.classList.remove('is-archived');
        app.querySelector('.title-bar')?.classList.remove('is-archived');
        app.querySelector('.archived-badge')?.remove();
        const btn = app.querySelector('[data-archive]') as HTMLButtonElement;
        if (btn) {
          btn.title = t('note.archive');
          btn.innerHTML = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg>';
        }
      } else {
        await api.archiveNote(note.id);
        await win.close();
      }
    } catch (e) {
      console.error('归档/恢复失败:', e);
    }
  });
}

/**
 * 提醒按钮 + AI 嗅探按钮。
 */
function bindReminderAndAi(note: Note, app: HTMLElement): void {
  app.querySelector('[data-reminder]')!.addEventListener('click', () => {
    showReminderPanel(note, app);
  });
  setupAiSniffButton(note, app);
}

/**
 * 标题栏拖拽 + 双击编辑标题。
 */
function bindTitleDrag(note: Note, app: HTMLElement): void {
  const titleBar = app.querySelector('[data-drag]') as HTMLElement;
  const titleText = app.querySelector('[data-title]') as HTMLElement;
  let lastTitleClick = 0;

  titleBar.addEventListener('mousedown', (e) => {
    if ((e.target as HTMLElement).closest('button')) return;
    const clickedTitle = (e.target as HTMLElement).closest('[data-title]');
    if (clickedTitle) {
      const now = Date.now();
      if (now - lastTitleClick < 500) {
        e.preventDefault();
        e.stopPropagation();
        enterTitleEdit(note, titleText, app);
        lastTitleClick = 0;
        return;
      }
      lastTitleClick = now;
    }
    win.startDragging();
  });
}
