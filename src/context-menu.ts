/**
 * 右键菜单 + 自定义颜色面板。
 *
 * 职责边界：
 * - setupContextMenu 绑定查看/编辑模式 contextmenu 事件 + 全局关闭监听
 * - showContextMenu 显示菜单（AI 重写 + 模板操作）
 * - showCustomColorPanel 12 预设色 + hex 输入框（替代原生 input[type=color]）
 * - applyCustomColor 应用自定义颜色
 *
 * 被调用方：note-renderer.ts (renderNote) + main.ts (颜色圆点点击)
 * 依赖：ai-rewrite.ts (getSelectedText/rewriteText) + template-ui.ts (showTemplatePicker/Applier) +
 *       note-style.ts (applyNoteStyle) + utils.ts (showToast) + api.ts (getAiConfig/updateNoteStyle) + i18n
 */

import type { Note } from './types';
import { t } from './i18n';
import { showToast } from './utils';
import * as api from './api';
import { getSelectedText, rewriteText } from './ai-rewrite';
import { showTemplatePicker, showTemplateApplier } from './template-ui';
import { applyNoteStyle } from './note-style';

export function setupContextMenu(note: Note, app: HTMLElement): void {
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

function closeCtxMenu(): void {
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

/** 颜色面板外部点击关闭监听（全局引用，用于清理） */
let colorPanelCloseHandler: ((ev: MouseEvent) => void) | null = null;

/** 关闭已存在的颜色面板 */
function closeColorPanel(): void {
  const panel = document.getElementById('color-panel');
  if (panel) panel.remove();
  // 清理外部 mousedown 监听
  if (colorPanelCloseHandler) {
    document.removeEventListener('mousedown', colorPanelCloseHandler);
    colorPanelCloseHandler = null;
  }
}

/**
 * 显示自定义颜色面板：12 个预设颜色 + hex 输入框。
 * 替代原生 <input type="color">（WebView2 中弹框位置不可控，默认在屏幕左上角）。
 */
export function showCustomColorPanel(note: Note, app: HTMLElement, customDot: HTMLElement): void {
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
function applyCustomColor(note: Note, app: HTMLElement, customDot: HTMLElement, hex: string): void {
  note.color = hex;
  app.querySelectorAll('.color-dot').forEach(d => d.classList.remove('active'));
  customDot.classList.add('active');
  applyNoteStyle(note);
  api.updateNoteStyle(note.id, hex, note.opacity, note.is_pinned);
}

async function showContextMenu(e: MouseEvent, note: Note, app: HTMLElement): Promise<void> {
  // 移除已有菜单
  document.getElementById('ctx-menu')?.remove();

  const menu = document.createElement('div');
  menu.id = 'ctx-menu';
  menu.style.cssText = `position:fixed;z-index:99999;background:var(--surface,#fff);border:1px solid var(--border,#e2e8f0);border-radius:8px;padding:4px 0;box-shadow:0 4px 16px rgba(0,0,0,0.12);min-width:140px;font-size:12px;max-height:${Math.floor(window.innerHeight * 0.8)}px;overflow-y:auto;`;

  type MenuItem = { label?: string; action?: () => void; type?: string; danger?: boolean; disabled?: boolean };

  // 检查 AI 是否已配置
  let aiConfigured = false;
  try {
    const config = await api.getAiConfig();
    aiConfigured = !!(config && config.api_key && config.api_key.length > 0);
  } catch { /* 读取失败视为未配置 */ }

  // AI 操作始终显示，未配置时禁用
  const selection = getSelectedText(note, app);
  const aiAction = (op: string) => {
    if (!selection) { showToast(t('note.aiNoSelection'), 'error'); return; }
    rewriteText(selection, op);
  };
  const aiDisabled = !aiConfigured || note.is_archived;
  const aiItems: MenuItem[] = [
    { type: 'separator' },
    { label: t('note.aiTidy'), action: () => aiAction('tidy'), disabled: aiDisabled },
    { label: t('note.aiTodoSplit'), action: () => aiAction('todo_split'), disabled: aiDisabled },
    { label: t('note.aiStyleFormal'), action: () => aiAction('style_formal'), disabled: aiDisabled },
    { label: t('note.aiStyleConcise'), action: () => aiAction('style_concise'), disabled: aiDisabled },
    { label: t('note.aiStyleMild'), action: () => aiAction('style_mild'), disabled: aiDisabled },
  ];

  const items: MenuItem[] = [
    { label: t('note.tplCreateFrom'), action: () => showTemplatePicker(note, app), disabled: note.is_archived },
    { label: t('note.tplApply'), action: () => showTemplateApplier(note, app), disabled: note.is_archived },
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
    if (item.disabled) {
      el.style.cssText = 'padding:6px 12px;cursor:not-allowed;color:var(--text-disabled,#aaa);white-space:nowrap;';
      el.title = t('hub.aiNotConfigured');
      el.addEventListener('click', (ev) => {
        ev.stopPropagation();
        showToast(t('hub.aiNotConfigured'), 'error');
        menu.remove();
      });
    } else {
      el.style.cssText = `padding:6px 12px;cursor:pointer;color:${item.danger ? '#dc2626' : 'var(--text,#333)'};white-space:nowrap;`;
      el.addEventListener('mouseenter', () => el.style.background = 'var(--surface-hover,#f1f5f9)');
      el.addEventListener('mouseleave', () => el.style.background = 'transparent');
      el.addEventListener('click', (ev) => { ev.stopPropagation(); item.action!(); menu.remove(); });
    }
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
