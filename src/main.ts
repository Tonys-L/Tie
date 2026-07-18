import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { marked } from 'marked';
import type { Note, Reminder, SniffResult, Suggestion, AiConfig, Template } from './types';
import { COLORS, escapeHtml, localISO, repeatLabel } from './utils';
import { initLocale, t, applyLocale, getLocaleTag } from './i18n';
import { getTemplates, createNoteFromTemplate } from './api';
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

// 图片目录路径，启动时异步获取
let imageDir = '';
invoke<string>('get_image_dir').then(dir => { imageDir = dir; }).catch(() => {});

// 渲染 Markdown 为 HTML，支持待办清单和图片
function renderMarkdown(content: string): string {
  if (!content.trim()) {
    return `<span class="placeholder">${t('note.placeholder')}</span>`;
  }
  // 将 img:filename 替换为 asset 协议 URL，再交给 marked 渲染
  let processed = content;
  if (imageDir) {
    processed = processed.replace(/img:([^\s)]+)/g, (_, filename) => {
      return convertFileSrc(imageDir + '\\' + filename);
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
  return html;
}

// ============ 入口 ============

const win = getCurrentWindow();
const noteId = win.label.startsWith('note-') ? win.label.slice(5) : '';
// 检查 URL 参数：?reminder=1 表示由提醒触发弹出
const urlParams = new URLSearchParams(window.location.search);
const isReminder = urlParams.get('reminder') === '1';
const urlReminderId = urlParams.get('rid') || '';
let currentReminderId = urlReminderId;

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
    setTimeout(() => app.classList.remove('flash-highlight'), 5100);
  });

  // 监听提醒触发事件：窗口已存在时，后端发送此事件显示横幅
  getCurrentWindow().listen('reminder-triggered', (event) => {
    const payload = event.payload as { reminder_id: string };
    currentReminderId = payload.reminder_id;
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
      <button class="banner-action" data-banner-snooze>${t('note.snooze')}</button>
      <button class="banner-action" data-banner-done>${t('note.done')}</button>
      <button class="banner-close" data-banner-close>&times;</button>
    </div>
    <div class="title-bar" data-drag>
      <span class="title-text" data-title>${escapeHtml(note.title) || t('app.note')}</span>
      <span class="title-time">${formatNoteTime(note.created_at)}</span>
	      <button class="icon-btn pin-btn ${note.is_pinned ? 'pinned' : ''}" data-pin title="${t('note.pin')}"></button>
	      <button class="icon-btn" data-close title="${t('note.close')}">&times;</button>
    </div>
    <div class="content-area">
      <div class="content-view" data-content-view>${renderMarkdown(note.content)}</div>
      <textarea class="content-edit" data-content style="display:none" placeholder="${t('note.placeholder')}" spellcheck="false">${escapeHtml(note.content)}</textarea>
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
	      <button class="icon-btn archive-btn" data-archive title="${t('note.archive')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg></button>
	      <button class="icon-btn del-btn" data-delete title="${t('note.delete')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4a1 1 0 011-1h4a1 1 0 011 1v2"/></svg></button>
    </div>
  `;

  applyNoteStyle(note);
  setupNoteEvents(note);
  setupTagEvents(note);

  // 关闭横幅按钮
  const banner = app.querySelector('[data-reminder-banner]') as HTMLElement;
  app.querySelector('[data-banner-close]')!.addEventListener('click', () => {
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
  });
  // 贪睡按钮：5分钟后再次提醒
  app.querySelector('[data-banner-snooze]')!.addEventListener('click', async () => {
    if (currentReminderId) {
      try { await invoke('snooze_reminder', { id: currentReminderId, minutes: 5 }); } catch (e) { console.error('贪睡失败:', e); }
    }
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
  });
  // 完成按钮：标记提醒为已完成
  app.querySelector('[data-banner-done]')!.addEventListener('click', async () => {
    if (currentReminderId) {
      try { await invoke('dismiss_reminder', { id: currentReminderId }); } catch (e) { console.error('完成提醒失败:', e); }
    }
    banner.style.display = 'none';
    app.classList.remove('reminder-flash');
  });

  // ---- 右键菜单 ----
  setupContextMenu(note, app);

  // ---- 待办排序按钮 ----
  setupTodoSortButton(note, app);

  // ---- 空便签模板快捷条 ----
  setupTemplateQuickBar(note, app);
}

// ============ 右键菜单 ============

function setupContextMenu(note: Note, app: HTMLElement) {
  const contentView = app.querySelector('[data-content-view]') as HTMLElement;
  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;

  // 查看模式右键
  contentView.addEventListener('contextmenu', (e) => {
    if ((e.target as HTMLElement).closest('a')) return;
    e.preventDefault();
    showContextMenu(e as MouseEvent, note, app);
  });

  // 编辑模式右键
  textarea.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    showContextMenu(e as MouseEvent, note, app);
  });

  // 点击其他区域关闭菜单
  document.addEventListener('click', () => closeCtxMenu());
  // 窗口失焦关闭菜单（点击桌面等）
  window.addEventListener('blur', () => closeCtxMenu());
  // Esc 关闭菜单
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') closeCtxMenu();
  });
}

function closeCtxMenu() {
  const menu = document.getElementById('ctx-menu');
  if (menu) menu.remove();
}

// ============ 自定义颜色面板 ============

/** 预设自定义颜色（hex 值，覆盖常用色相） */
const PRESET_COLORS: string[] = [
  '#ef4444', '#f59e0b', '#eab308', '#22c55e',
  '#14b8a6', '#3b82f6', '#6366f1', '#8b5cf6',
  '#ec4899', '#64748b', '#0ea5e9', '#84cc16',
];

/** 关闭已存在的颜色面板 */
function closeColorPanel() {
  const panel = document.getElementById('color-panel');
  if (panel) panel.remove();
  // 清理外部 mousedown 监听
  if (colorPanelCloseHandler) {
    document.removeEventListener('mousedown', colorPanelCloseHandler);
    colorPanelCloseHandler = null;
  }
}

/** 颜色面板外部点击关闭监听（全局引用，用于清理） */
let colorPanelCloseHandler: ((ev: MouseEvent) => void) | null = null;

/**
 * 显示自定义颜色面板：12 个预设颜色 + hex 输入框。
 * 替代原生 <input type="color">（WebView2 中弹框位置不可控，默认在屏幕左上角）。
 */
function showCustomColorPanel(note: Note, app: HTMLElement, customDot: HTMLElement) {
  closeColorPanel();
  closeCtxMenu();

  const panel = document.createElement('div');
  panel.id = 'color-panel';

  // 预设颜色网格
  const grid = document.createElement('div');
  grid.className = 'cp-grid';
  PRESET_COLORS.forEach(hex => {
    const cell = document.createElement('div');
    cell.className = 'cp-cell';
    cell.style.background = hex;
    if (note.color.toLowerCase() === hex.toLowerCase()) cell.classList.add('selected');
    cell.addEventListener('click', (ev) => {
      ev.stopPropagation();
      applyCustomColor(note, app, customDot, hex);
      closeColorPanel();
    });
    grid.appendChild(cell);
  });
  panel.appendChild(grid);

  // hex 输入框
  const inputWrap = document.createElement('div');
  inputWrap.className = 'cp-input-wrap';
  const input = document.createElement('input');
  input.type = 'text';
  input.className = 'cp-input';
  input.value = note.color.startsWith('#') ? note.color : '#3b82f6';
  input.maxLength = 7;
  input.setAttribute('spellcheck', 'false');
  const applyBtn = document.createElement('button');
  applyBtn.className = 'cp-apply';
  applyBtn.textContent = '✓';
  applyBtn.title = t('note.customColor');
  const applyHex = () => {
    let v = input.value.trim();
    if (!v.startsWith('#')) v = '#' + v;
    if (/^#[0-9a-fA-F]{6}$/.test(v)) {
      applyCustomColor(note, app, customDot, v.toLowerCase());
      closeColorPanel();
    }
  };
  applyBtn.addEventListener('click', (ev) => { ev.stopPropagation(); applyHex(); });
  input.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter') { ev.preventDefault(); applyHex(); }
    if (ev.key === 'Escape') { ev.preventDefault(); closeColorPanel(); }
  });
  inputWrap.appendChild(input);
  inputWrap.appendChild(applyBtn);
  panel.appendChild(inputWrap);

  document.body.appendChild(panel);

  // 阻止面板内 click 冒泡，防止触发外部关闭监听
  panel.addEventListener('click', (ev) => ev.stopPropagation());

  // 定位面板：圆点上方，不超出窗口
  const rect = customDot.getBoundingClientRect();
  const panelRect = panel.getBoundingClientRect();
  const maxX = window.innerWidth - panelRect.width - 4;
  const x = Math.min(rect.left, maxX);
  // 优先在圆点上方弹出，空间不足时下方
  const aboveY = rect.top - panelRect.height - 4;
  const belowY = rect.bottom + 4;
  const y = aboveY > 4 ? aboveY : belowY;
  panel.style.left = Math.max(4, x) + 'px';
  panel.style.top = y + 'px';

  // 点击面板外部关闭（mousedown 比 click 更可靠，避免输入框失焦后意外关闭）
  const closeOnOutside = (ev: MouseEvent) => {
    if (panel.contains(ev.target as Node)) return;
    closeColorPanel();
  };
  colorPanelCloseHandler = closeOnOutside;
  setTimeout(() => {
    document.addEventListener('mousedown', closeOnOutside);
    window.addEventListener('blur', closeColorPanel, { once: true });
  }, 0);
}

/** 应用自定义颜色：更新 note + UI + 持久化 */
function applyCustomColor(note: Note, app: HTMLElement, customDot: HTMLElement, hex: string) {
  note.color = hex;
  app.querySelectorAll('.color-dot').forEach(d => d.classList.remove('active'));
  customDot.classList.add('active');
  applyNoteStyle(note);
  invoke('update_note_style', {
    id: note.id, color: hex, opacity: note.opacity, isPinned: note.is_pinned,
  });
}

function showContextMenu(e: MouseEvent, note: Note, app: HTMLElement) {
  // 移除已有菜单
  document.getElementById('ctx-menu')?.remove();

  const menu = document.createElement('div');
  menu.id = 'ctx-menu';
  menu.style.cssText = `position:fixed;z-index:99999;background:var(--surface,#fff);border:1px solid var(--border,#e2e8f0);border-radius:8px;padding:4px 0;box-shadow:0 4px 16px rgba(0,0,0,0.12);min-width:140px;font-size:12px;max-height:${Math.floor(window.innerHeight * 0.8)}px;overflow-y:auto;`;

  type MenuItem = { label?: string; action?: () => void; type?: string; danger?: boolean };

  // AI 操作始终显示，点击时若无选区则提示
  const selection = getSelectedText(note, app);
  const aiAction = (op: string) => {
    if (!selection) { showToast(t('note.aiNoSelection'), 'error'); return; }
    rewriteText(selection, op);
  };
  const aiItems: MenuItem[] = [
    { type: 'separator' },
    { label: t('note.aiTidy'), action: () => aiAction('tidy') },
    { label: t('note.aiTodoSplit'), action: () => aiAction('todo_split') },
    { label: t('note.aiStyleFormal'), action: () => aiAction('style_formal') },
    { label: t('note.aiStyleConcise'), action: () => aiAction('style_concise') },
    { label: t('note.aiStyleMild'), action: () => aiAction('style_mild') },
  ];

  const items: MenuItem[] = [
    { label: t('note.tplCreateFrom'), action: () => showTemplatePicker(note, app) },
    { label: t('note.tplApply'), action: () => showTemplateApplier(note, app) },
    ...aiItems,
  ];

  items.forEach(item => {
    if (item.type === 'separator') {
      const sep = document.createElement('div');
      sep.style.cssText = 'height:1px;background:var(--border-light,#e2e8f0);margin:4px 0;';
      menu.appendChild(sep);
      return;
    }
    const el = document.createElement('div');
    el.innerHTML = item.label!;
    el.style.cssText = `padding:6px 12px;cursor:pointer;color:${item.danger ? '#dc2626' : 'var(--text,#333)'};white-space:nowrap;`;
    el.addEventListener('mouseenter', () => el.style.background = 'var(--surface-hover,#f1f5f9)');
    el.addEventListener('mouseleave', () => el.style.background = 'transparent');
    el.addEventListener('click', (ev) => { ev.stopPropagation(); item.action!(); menu.remove(); });
    menu.appendChild(el);
  });

  document.body.appendChild(menu);

  // 定位菜单（不超出窗口）
  const rect = { x: e.clientX, y: e.clientY };
  const menuRect = menu.getBoundingClientRect();
  const maxX = window.innerWidth - menuRect.width - 4;
  const maxY = window.innerHeight - menuRect.height - 4;
  menu.style.left = Math.min(rect.x, maxX) + 'px';
  menu.style.top = Math.min(rect.y, maxY) + 'px';
}

// ============ AI 文本重写 ============

/**
 * 获取当前选中的文本及其替换函数。
 * - 编辑模式：通过 textarea.selectionStart/End 获取
 * - 查看模式：通过 window.getSelection() 获取
 * - 选中文本 trim 后长度 < 5 时返回 null（前端预检查，与后端校验对齐）
 */
function getSelectedText(note: Note, app: HTMLElement): { text: string; replace: (newText: string) => void } | null {
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
        invoke('update_note_content', { id: note.id, content: note.content });
        const contentView = app.querySelector('[data-content-view]') as HTMLElement;
        if (contentView) contentView.innerHTML = renderMarkdown(note.content);
      }
    };
  }

  return null;
}

/**
 * 调用后端 ai_rewrite_text 重写选中文本并替换。
 * 显示 loading → 调用后端 → 替换文本 → 显示结果提示。
 */
async function rewriteText(selection: { text: string; replace: (newText: string) => void }, operation: string): Promise<void> {
  try {
    showToast(t('note.aiProcessing'), 'info', true);
    const result = await invoke<string>('ai_rewrite_text', { text: selection.text, operation });
    if (result) {
      selection.replace(result);
      showToast(t('note.aiReplaced'), 'success');
    }
  } catch (e) {
    console.error('AI 重写失败:', e);
    showToast(t('note.aiFailed') + ': ' + e, 'error');
  }
}

/**
 * 轻量 toast 提示（底部居中），新提示自动替换已有提示。
 * persistent=true 时不自动消失（用于 loading 状态，由后续 toast 替换）。
 */
function showToast(message: string, type: 'info' | 'success' | 'error' = 'info', persistent: boolean = false): void {
  const existing = document.querySelector('.ai-toast');
  if (existing) existing.remove();
  const toast = document.createElement('div');
  toast.className = 'ai-toast';
  toast.textContent = message;
  const bg = type === 'error' ? '#dc2626' : type === 'success' ? '#16a34a' : '#475569';
  toast.style.cssText = `position:fixed;bottom:20px;left:50%;transform:translateX(-50%);padding:6px 12px;border-radius:4px;font-size:12px;z-index:99999;background:${bg};color:#fff;box-shadow:0 2px 8px rgba(0,0,0,0.15);white-space:nowrap;`;
  document.body.appendChild(toast);
  if (!persistent) {
    setTimeout(() => toast.remove(), 2000);
  }
}

// ============ 待办清单 AI 排序 ============

/** 已排序的便签 id 集合（内存级，内容变化时清除，避免重复显示排序按钮） */
const sortedNoteIds = new Set<string>();

/** 提取 content 中所有未完成待办条目（`- [ ]` / `* [ ]` / `+ [ ]`）的文本 */
function extractTodoItems(content: string): string[] {
  return content
    .split('\n')
    .filter(line => /^\s*[-*+]\s+\[ \] /.test(line))
    .map(line => line.replace(/^\s*[-*+]\s+\[ \] /, ''));
}

/** 将排序后的条目按原顺序替换回 content 中的未完成待办行（保留原标记符和缩进） */
function applySortedTodos(content: string, sortedItems: string[]): string {
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
function setupTodoSortButton(note: Note, app: HTMLElement): void {
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
    try {
      const sorted = await invoke<string[]>('ai_sort_todos', { todos });
      if (sorted.length !== todos.length) {
        showToast(t('note.aiSortMismatch'), 'error');
        return;
      }
      note.content = applySortedTodos(note.content, sorted);
      // 标记为已排序，排序后不重新显示按钮
      sortedNoteIds.add(note.id);
      // 更新编辑框和视图
      const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;
      if (textarea) textarea.value = note.content;
      contentView.innerHTML = renderMarkdown(note.content);
      // 自动保存
      invoke('update_note_content', { id: note.id, content: note.content });
      showToast(t('note.aiSortDone'), 'success');
    } catch (e) {
      console.error('AI 排序失败:', e);
      showToast(t('note.aiFailed'), 'error');
    } finally {
      btn.textContent = t('note.aiSortTodos');
      (btn as HTMLButtonElement).disabled = false;
    }
  });
  contentView.insertBefore(btn, contentView.firstChild);
}

// ============ 空便签模板快捷条 ============

/**
 * 空便签模板快捷条：当 note.content 为空时在 content-area 顶部显示模板按钮列表。
 * - 点击模板按钮 → 填充内容、保存、切回查看模式渲染 markdown、隐藏快捷条
 * - 用户输入内容后隐藏快捷条；内容被清空后重新显示
 * - 无模板时不显示
 */
function setupTemplateQuickBar(note: Note, app: HTMLElement): void {
  const contentArea = app.querySelector('.content-area') as HTMLElement;
  if (!contentArea) return;

  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;
  const contentView = app.querySelector('[data-content-view]') as HTMLElement;

  // 创建快捷条（absolute 定位，不影响内容布局）
  const bar = document.createElement('div');
  bar.className = 'tpl-quick-bar';
  bar.style.display = 'none';
  contentArea.appendChild(bar);

  // 控制显隐：内容为空且有模板时显示
  const updateVisibility = (hasTemplates: boolean) => {
    const isEmpty = !note.content.trim();
    bar.style.display = (isEmpty && hasTemplates) ? 'flex' : 'none';
  };

  // 监听 textarea 输入：内容变化时更新显隐
  textarea.addEventListener('input', () => {
    note.content = textarea.value;
    updateVisibility(true);
  });

  // 监听 textarea blur：内容可能被清空（用户编辑后删除所有内容）
  textarea.addEventListener('blur', () => {
    updateVisibility(true);
  });

  // 加载模板列表
  getTemplates()
    .then(templates => {
      if (templates.length === 0) {
        updateVisibility(false);
        return;
      }
      bar.innerHTML = `
        <span class="tpl-quick-label">${t('note.tplQuickTitle')}</span>
        <div class="tpl-quick-list">
          ${templates.map(tp =>
            `<button class="tpl-quick-btn" data-tpl-id="${escapeHtml(tp.id)}">${escapeHtml(tp.name)}</button>`
          ).join('')}
        </div>
      `;
      // 绑定模板按钮点击
      bar.querySelectorAll('[data-tpl-id]').forEach(btn => {
        btn.addEventListener('click', (ev) => {
          ev.stopPropagation();
          const tplId = (btn as HTMLElement).dataset.tplId!;
          const tpl = templates.find(tp => tp.id === tplId);
          if (!tpl) return;
          // 填充内容
          textarea.value = tpl.content;
          note.content = tpl.content;
          invoke('update_note_content', { id: note.id, content: tpl.content });
          // 隐藏快捷条
          bar.style.display = 'none';
          // 切回查看模式并重新渲染 markdown
          if (contentView) {
            contentView.innerHTML = renderMarkdown(tpl.content);
            contentView.style.display = 'block';
            textarea.style.display = 'none';
          }
          // 清除待办排序标记，允许新内容重新检测排序按钮
          sortedNoteIds.delete(note.id);
          setupTodoSortButton(note, app);
        });
      });
      updateVisibility(true);
    })
    .catch(err => {
      console.error('加载模板失败:', err);
      updateVisibility(false);
    });
}

// ============ 模板选择浮层（右键菜单"从模板新建" / "应用模板到当前便签"） ============

/**
 * 显示模板选择浮层（通用）。
 * @param title 浮层标题
 * @param onSelect 选择模板后的回调（传入 Template）
 */
async function showTemplateDialog(
  title: string,
  app: HTMLElement,
  onSelect: (tpl: Template) => Promise<void>
): Promise<void> {
  // 移除已有浮层
  app.querySelector('.tpl-picker-overlay')?.remove();

  const overlay = document.createElement('div');
  overlay.className = 'tpl-picker-overlay';

  const dialog = document.createElement('div');
  dialog.className = 'tpl-picker-dialog';
  dialog.innerHTML = `
    <div class="tp-header">
      <span>${title}</span>
      <button class="tp-close">&times;</button>
    </div>
    <div class="tp-list" data-tp-list>
      <div class="tp-loading">...</div>
    </div>
  `;
  overlay.appendChild(dialog);
  app.appendChild(overlay);

  // 关闭事件
  const close = () => overlay.remove();
  dialog.querySelector('.tp-close')!.addEventListener('click', close);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });

  // 加载模板列表
  const listEl = dialog.querySelector('[data-tp-list]') as HTMLElement;
  try {
    const templates = await getTemplates();
    if (templates.length === 0) {
      listEl.innerHTML = `<div class="tp-empty">${t('note.tplQuickEmpty')}</div>`;
      return;
    }
    listEl.innerHTML = templates.map(tp =>
      `<button class="tp-item" data-tp-id="${escapeHtml(tp.id)}">${escapeHtml(tp.name)}</button>`
    ).join('');
    listEl.querySelectorAll('[data-tp-id]').forEach(btn => {
      btn.addEventListener('click', async () => {
        const tplId = (btn as HTMLElement).dataset.tpId!;
        const tpl = templates.find(tp => tp.id === tplId);
        if (!tpl) return;
        try {
          await onSelect(tpl);
          close();
        } catch (e) {
          showToast(t('note.executeFailed') + ': ' + e, 'error');
        }
      });
    });
  } catch (e) {
    listEl.innerHTML = `<div class="tp-empty">${t('note.tplQuickEmpty')}</div>`;
    console.error('加载模板失败:', e);
  }
}

/**
 * 「从模板新建便签」：调用后端创建新便签并打开新窗口，当前便签不受影响。
 */
async function showTemplatePicker(_note: Note, app: HTMLElement): Promise<void> {
  await showTemplateDialog(t('note.tplPickerTitle'), app, async (tpl) => {
    await createNoteFromTemplate(tpl.id);
    showToast(t('note.tplCreated'), 'success');
  });
}

/**
 * 「应用模板到当前便签」：在当前便签 content 末尾追加 `\n\n` + 模板内容，保存并重新渲染。
 * - 非破坏性：不覆盖已有内容
 * - 触发 update_note_content 保存
 * - 重新渲染 content-view 的 markdown
 */
async function showTemplateApplier(note: Note, app: HTMLElement): Promise<void> {
  await showTemplateDialog(t('note.tplApplierTitle'), app, async (tpl) => {
    // 在末尾追加模板内容（用空行分隔）
    const separator = note.content.trim() ? '\n\n' : '';
    const newContent = note.content + separator + tpl.content;
    // 更新内存 + UI + 后端
    note.content = newContent;
    const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;
    const contentView = app.querySelector('[data-content-view]') as HTMLElement;
    if (textarea) textarea.value = newContent;
    if (contentView) contentView.innerHTML = renderMarkdown(newContent);
    invoke('update_note_content', { id: note.id, content: newContent });
    // 清除待办排序标记，允许新内容重新检测排序按钮
    sortedNoteIds.delete(note.id);
    setupTodoSortButton(note, app);
    showToast(t('note.tplApplied'), 'success');
  });
}

/** 格式化便签创建时间为简短显示（如 "7/17 10:30"） */
function formatNoteTime(iso: string): string {
  const d = new Date(iso);
  const locale = localStorage.getItem('locale') || 'zh';
  const month = d.getMonth() + 1;
  const day = d.getDate();
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  return locale === 'zh' ? `${month}/${day} ${hh}:${mm}` : `${month}/${day} ${hh}:${mm}`;
}

function applyNoteStyle(note: Note) {
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

// ============ 标签渲染 ============

function renderTagPills(tags: string[]): string {
  return tags.map(tag =>
    `<span class="tag-pill" data-tag="${escapeHtml(tag)}">${escapeHtml(tag)}<button class="tag-remove" data-tag-remove="${escapeHtml(tag)}">&times;</button></span>`
  ).join('');
}

function refreshTagBar(note: Note) {
  const tagList = document.querySelector('[data-tag-list]') as HTMLElement;
  if (tagList) tagList.innerHTML = renderTagPills(note.tags);
}

function setupTagEvents(note: Note) {
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
        invoke('update_note_tags', { id: note.id, tags: newTags });
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
      invoke('update_note_tags', { id: note.id, tags: note.tags });
    }
  });
}

// ============ 事件绑定 ============

function setupNoteEvents(note: Note) {
  const app = document.getElementById('app')!;

  // ---- 内容编辑：查看/编辑模式切换 ----
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
      // 同步 textarea 值，避免编辑模式时内容不一致
      textarea.value = note.content;
      // 重新渲染查看区
      contentView.innerHTML = renderMarkdown(note.content);
      // checkbox 切换改变了待办状态，清除已排序标记并重新检测排序按钮
      sortedNoteIds.delete(note.id);
      setupTodoSortButton(note, app);
      // 自动保存
      invoke('update_note_content', { id: note.id, content: note.content });
      // checkbox 切换不触发嗅探（只是状态变化，内容主体未变）
      return;
    }
    // 拦截链接点击：在系统浏览器打开，不进入编辑模式
    const link = (e.target as HTMLElement).closest('a');
    if (link) {
      e.preventDefault();
      const href = link.getAttribute('href');
      if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
        invoke('open_url', { url: href }).catch(err => console.error('打开链接失败:', err));
      }
      return;
    }
    contentView.style.display = 'none';
    textarea.style.display = 'block';
    textarea.focus();
    // 光标移到末尾
    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
  });

  // 失焦 → 保存并切回查看模式
  textarea.addEventListener('blur', () => {
    const content = textarea.value;
    // 内容变化时清除已排序标记，允许再次排序
    if (content !== note.content) {
      sortedNoteIds.delete(note.id);
    }
    note.content = content;
    contentView.innerHTML = renderMarkdown(content);
    textarea.style.display = 'none';
    contentView.style.display = 'block';
    invoke('update_note_content', { id: note.id, content });
    // 重新检测待办排序按钮（内容变化后待办数量可能 > 3）
    setupTodoSortButton(note, app);
    // 嗅探：失焦保存后异步识别时间关键词
    sniffAfterSave(note);
  });

  // ---- 快捷键 ----
  document.addEventListener('keydown', (e) => {
    // Ctrl+S：保存当前编辑内容
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      if (textarea.style.display !== 'none') {
        // 编辑模式 → 保存并切回查看模式
        textarea.blur();
      } else {
        // 查看模式 → 无操作（已自动保存）
      }
    }
    // Esc：退出编辑模式回到查看模式
    if (e.key === 'Escape') {
      if (textarea.style.display !== 'none') {
        e.preventDefault();
        textarea.blur();
      }
    }
    // Ctrl+N：新建便签
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      invoke('create_note');
    }
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
          const filename = await invoke<string>('save_image', { data, ext });
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
      const filename = await invoke<string>('save_image', { data, ext });
      const md = `![](img:${filename})`;
      // 如果不在编辑模式，先切换到编辑模式
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

  // 阻止拖拽默认行为（防止浏览器打开文件）
  const preventDragOver = (e: DragEvent) => {
    e.preventDefault();
  };

  textarea.addEventListener('drop', handleDrop);
  textarea.addEventListener('dragover', preventDragOver);
  contentView.addEventListener('drop', handleDrop);
  contentView.addEventListener('dragover', preventDragOver);

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
      app.querySelectorAll('.color-dot').forEach(d => d.classList.remove('active'));
      dot.classList.add('active');
      applyNoteStyle(note);
      invoke('update_note_style', {
        id: note.id, color, opacity: note.opacity, isPinned: note.is_pinned,
      });
    });
  });

  // ---- 自定义颜色 ----
  // 点击圆点弹出颜色面板（WebView2 原生 color picker 弹框位置不可控，改用自定义面板）
  const customDot = app.querySelector('[data-custom-color]') as HTMLElement;
  if (customDot) {
    customDot.addEventListener('click', (e) => {
      e.stopPropagation();
      showCustomColorPanel(note, app, customDot);
    });
  }

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

  // ---- AI 手动嗅探 ----
  setupAiSniffButton(note, app);

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
        <button class="rbtn" data-repeat="lunar_monthly">${t('note.lunarMonthly')}</button>
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

  // AI 自然语言解析已移至便签保存后的自动嗅探气泡，此处仅保留手动表单
  const reminderTitle = note.title || t('app.note');

  // 保存
  overlay.querySelector('[data-save-reminder]')!.addEventListener('click', async () => {
    const input = overlay.querySelector('[data-remind-at]') as HTMLInputElement;
    const dt = new Date(input.value);
    if (isNaN(dt.getTime())) return;
    const remindAt = dt.toISOString();
    try {
      await invoke('create_reminder', {
        noteId: note.id,
        noteTitle: reminderTitle,
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
        const dt = new Date(r.remind_at).toLocaleString(getLocaleTag(), { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
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

// ============ AI 建议面板：便签保存后自动嗅探并给出建议 ============

// 每个便签最近一次嗅探时间戳（noteId → ts），10 秒内不重复嗅探同一便签
const sniffDebounceMap = new Map<string, number>();
const sniffContentMap = new Map<string, string>();
const SNIFF_DEBOUNCE_MS = 10_000;

// 当前面板自动消失定时器
let sniffPanelTimer: number | null = null;

/**
 * 初始化 AI 手动嗅探按钮：
 * - 异步检查 AI 配置，未配置时保持置灰并提示
 * - 已配置时启用按钮，点击触发 force 嗅探
 * - 加载中按钮禁用并显示"⏳ 处理中..."
 */
function setupAiSniffButton(note: Note, app: HTMLElement): void {
  const btn = app.querySelector('[data-ai-sniff]') as HTMLButtonElement;
  if (!btn) return;

  // 异步检查 AI 配置
  invoke<AiConfig>('get_ai_config')
    .then(config => {
      if (config && config.api_key && config.api_key.length > 0) {
        btn.disabled = false;
        btn.title = t('hub.aiAssistant');
      } else {
        btn.disabled = true;
        btn.title = t('hub.aiNotConfigured');
      }
    })
    .catch(() => {
      // 读取配置失败：保持置灰
      btn.disabled = true;
      btn.title = t('hub.aiNotConfigured');
    });

  btn.addEventListener('click', () => {
    if (btn.disabled) return;
    // 加载状态：禁止重复点击
    btn.disabled = true;
    const originalHTML = btn.innerHTML;
    btn.innerHTML = `<span style="font-size:11px;">${t('hub.sniffLoading')}</span>`;

    sniffAfterSave(note, true, (suggestions) => {
      // 恢复按钮
      btn.innerHTML = originalHTML;
      // 重新检查配置状态以决定是否启用（配置可能在加载时已就绪）
      invoke<AiConfig>('get_ai_config')
        .then(config => {
          btn.disabled = !(config && config.api_key && config.api_key.length > 0);
        })
        .catch(() => { btn.disabled = false; });

      // 无建议时给一个轻提示
      if (suggestions.length === 0) {
        showSniffEmptyHint(app);
      }
    });
  });
}

/**
 * 嗅探无建议时的轻量提示（2 秒后自动消失）。
 */
function showSniffEmptyHint(app: HTMLElement): void {
  const existing = app.querySelector('.sniff-empty-hint');
  if (existing) existing.remove();
  const hint = document.createElement('div');
  hint.className = 'sniff-empty-hint';
  hint.textContent = t('hub.sniffNoSuggestions');
  app.appendChild(hint);
  setTimeout(() => hint.remove(), 2000);
}

/**
 * 便签保存后调用嗅探：异步、防抖、静默失败。
 * 命中建议则显示右侧 AI 建议面板。
 * 仅在内容变化时触发，避免无谓 AI 调用。
 *
 * force=true 时绕过防抖和内容变化检查（手动触发），
 * 失败时通过可选回调通知调用方。
 */
async function sniffAfterSave(note: Note, force: boolean = false, onDone?: (suggestions: Suggestion[]) => void): Promise<void> {
  if (!force) {
    // 内容未变则跳过
    const lastContent = sniffContentMap.get(note.id);
    if (lastContent === note.content) { if (onDone) onDone([]); return; }
    // 防抖：10 秒内不重复嗅探同一便签
    const now = Date.now();
    const last = sniffDebounceMap.get(note.id) || 0;
    if (now - last < SNIFF_DEBOUNCE_MS) { if (onDone) onDone([]); return; }
    // 前端预检查嗅探开关：关闭则直接跳过，不发起 IPC 调用
    try {
      const config = await invoke<AiConfig>('get_ai_config');
      if (!config.sniff_enabled) { if (onDone) onDone([]); return; }
    } catch { /* 读取配置失败则继续，后端会再次校验 */ }
    sniffDebounceMap.set(note.id, now);
    sniffContentMap.set(note.id, note.content);
  }

  // 嗅探完全异步，不阻塞保存流程；失败静默
  invoke<Suggestion[]>('sniff_suggestions', { content: note.content })
    .then(suggestions => {
      if (suggestions && suggestions.length > 0) {
        showSuggestionPanel(note, suggestions);
      }
      if (onDone) onDone(suggestions || []);
    })
    .catch(err => {
      console.error('嗅探失败:', err);
      if (onDone) onDone([]);
    });
}

/**
 * 在便签窗口右侧显示 AI 建议面板（半透明浮层，不占主编辑区）。
 * 同一时间只保留一个面板；10 秒后自动消失。
 */
function showSuggestionPanel(note: Note, suggestions: Suggestion[]): void {
  // 移除已有面板
  const existing = document.querySelector('.sniff-panel');
  if (existing) existing.remove();
  if (sniffPanelTimer !== null) {
    clearTimeout(sniffPanelTimer);
    sniffPanelTimer = null;
  }

  const panel = document.createElement('div');
  panel.className = 'sniff-panel';
  panel.innerHTML = `
    <div class="sniff-panel-header">
      <span class="sniff-panel-title">${t('hub.aiSuggestions')}</span>
      <button class="sniff-panel-close" data-panel-close title="${t('note.close')}">&times;</button>
    </div>
    <div class="sniff-panel-list">
      ${suggestions.map((s, i) => `
        <div class="sniff-item" data-item-index="${i}">
          <div class="sniff-item-title">${escapeHtml(s.title)}</div>
          <div class="sniff-item-desc">${escapeHtml(s.description)}</div>
          <button class="sniff-item-exec" data-exec-index="${i}">${t('hub.execute')}</button>
        </div>
      `).join('')}
    </div>
  `;
  document.body.appendChild(panel);

  const removePanel = () => {
    panel.remove();
    if (sniffPanelTimer !== null) {
      clearTimeout(sniffPanelTimer);
      sniffPanelTimer = null;
    }
  };

  // 关闭按钮
  panel.querySelector('[data-panel-close]')!.addEventListener('click', removePanel);

  // 执行按钮分发
  suggestions.forEach((suggestion, i) => {
    const execBtn = panel.querySelector(`[data-exec-index="${i}"]`) as HTMLButtonElement;
    execBtn.addEventListener('click', async () => {
      const item = panel.querySelector(`[data-item-index="${i}"]`) as HTMLElement;
      execBtn.disabled = true;
      try {
        await executeSuggestion(note, suggestion);
        // 成功：该项变为绿色"已执行"状态，2 秒后面板消失
        item.classList.add('executed');
        item.innerHTML = `
          <div class="sniff-item-title">${escapeHtml(suggestion.title)}</div>
          <div class="sniff-item-done">${t('hub.executed')}</div>
        `;
        // 执行成功后不消失，用户可能还要执行其他建议
      } catch (e) {
        // 失败：该项显示红色错误提示，恢复按钮可点击
        console.error('执行建议失败:', e);
        item.classList.add('failed');
        const errDiv = document.createElement('div');
        errDiv.className = 'sniff-item-error';
        errDiv.textContent = String(e);
        // 避免重复追加错误提示
        if (!item.querySelector('.sniff-item-error')) {
          item.appendChild(errDiv);
        }
        execBtn.disabled = false;
      }
    });
  });

  // 不自动消失：只在用户点击关闭、再次分析、或关闭便签时消失
  if (sniffPanelTimer !== null) {
    clearTimeout(sniffPanelTimer);
    sniffPanelTimer = null;
  }
}

/**
 * 根据 suggestion.type 分发执行建议。
 * - reminder：调用 create_reminder，从 data 提取 start_time/title/repeat_type
 *   - start_time 格式 "YYYY-MM-DD HH:mm" → ISO
 *   - repeat_type === 'once' 映射为 'none'（后端要求）
 *   - 标题优先用 data.title，兜底 note.title
 * - todo_split：把字符串数组转为待办清单 Markdown，替换便签正文
 * - tidy：用规整后的文本替换便签正文
 * - style：用切换后的文本替换便签正文
 * - tag_suggest：把推荐标签追加到便签（去重，domain 层兜底限制）
 */
async function executeSuggestion(note: Note, suggestion: Suggestion): Promise<void> {
  switch (suggestion.type) {
    case 'reminder':
      await executeReminder(note, suggestion.data as SniffResult);
      break;
    case 'todo_split':
      await executeTodoSplit(note, suggestion.data as string[]);
      break;
    case 'tidy':
      await executeTidy(note, suggestion.data as string);
      break;
    case 'style':
      await executeStyle(note, suggestion.data as { style_type: string; styled_text: string });
      break;
    case 'tag_suggest':
      await executeTagSuggest(note, suggestion.data as string[]);
      break;
    default:
      throw new Error(`${t('hub.executeFailed')}: ${suggestion.type}`);
  }
}

/**
 * 更新便签正文：同步内存/textarea/查看区/后端。
 * 用于 todo_split/tidy/style 三种建议的正文替换。
 */
function updateNoteContent(note: Note, newContent: string): void {
  note.content = newContent;
  const textarea = document.querySelector('[data-content]') as HTMLTextAreaElement | null;
  const contentView = document.querySelector('[data-content-view]') as HTMLElement | null;
  if (textarea) textarea.value = newContent;
  if (contentView) contentView.innerHTML = renderMarkdown(newContent);
  invoke('update_note_content', { id: note.id, content: newContent });
}

/**
 * reminder：调用 create_reminder 创建提醒。
 * - start_time 格式 "YYYY-MM-DD HH:mm" → ISO
 * - repeat_type === 'once' 映射为 'none'（后端要求）
 * - 标题优先用 data.title，兜底 note.title
 */
async function executeReminder(note: Note, data: SniffResult): Promise<void> {
  const dt = new Date(data.start_time.replace(' ', 'T'));
  if (isNaN(dt.getTime())) {
    throw new Error('invalid start_time: ' + data.start_time);
  }
  const noteTitle = data.title || note.title || t('app.note');
  const repeatType = data.repeat_type === 'once' ? 'none' : data.repeat_type;
  await invoke('create_reminder', {
    noteId: note.id,
    noteTitle,
    remindAt: dt.toISOString(),
    repeatType,
  });
}

/**
 * todo_split：把字符串数组转为 GFM 待办清单 Markdown，替换便签正文。
 */
async function executeTodoSplit(note: Note, todos: string[]): Promise<void> {
  if (!Array.isArray(todos) || todos.length === 0) {
    throw new Error('empty todos');
  }
  const newContent = todos.map(todo => `- [ ] ${todo}`).join('\n');
  updateNoteContent(note, newContent);
}

/**
 * tidy：用规整后的文本替换便签正文。
 */
async function executeTidy(note: Note, tidyText: string): Promise<void> {
  if (typeof tidyText !== 'string' || !tidyText.trim()) {
    throw new Error('empty tidy text');
  }
  updateNoteContent(note, tidyText);
}

/**
 * style：用切换文风后的文本替换便签正文。
 */
async function executeStyle(note: Note, data: { style_type: string; styled_text: string }): Promise<void> {
  if (!data || !data.styled_text || !data.styled_text.trim()) {
    throw new Error('empty styled text');
  }
  updateNoteContent(note, data.styled_text);
}

/**
 * tag_suggest：把推荐标签追加到便签（前端去重，domain 层兜底限制数量/长度）。
 */
async function executeTagSuggest(note: Note, tags: string[]): Promise<void> {
  if (!Array.isArray(tags) || tags.length === 0) {
    throw new Error('empty tags');
  }
  // 前端去重：过滤掉便签已有的标签
  const existing = new Set(note.tags);
  const newTags = tags.filter(tag => tag && !existing.has(tag));
  if (newTags.length === 0) {
    // 全部已存在：无需调用后端，视为成功
    return;
  }
  const merged = [...note.tags, ...newTags];
  note.tags = merged;
  refreshTagBar(note);
  await invoke('update_note_tags', { id: note.id, tags: merged });
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
