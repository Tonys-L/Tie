/**
 * 便签多选与批量操作（Hub 页面）：管理选中状态 + 批量操作栏事件。
 *
 * 职责边界：
 * - selectedIds state 管理（增删/清空/查询）
 * - 批量操作栏事件绑定（归档/恢复/删除/改色/取消）
 * - updateMultiSelectUI（高亮选中项 + 批量操作栏显示/隐藏 + 归档/恢复按钮切换）
 * - Esc 退出多选
 *
 * 不负责：列表事件委托（由 notes-list 处理后调用本模块的 toggle/clear）、
 *         单条删除确认（由 delete-confirm 处理）
 *
 * 被调用方：notes-list.ts（init 注入依赖 + 调用 toggle/clear/refreshSelectionUI）
 * 依赖：api.ts (批量操作) + i18n (t) +
 *       notes-list.ts（通过 init 注入的 reloadList/getCurrentTab callback，避免循环依赖）
 */

import * as api from './api';
import { t } from './i18n';
import { BATCH_COLORS } from './colors';

// ===== 模块级 state =====
let selectedIds: Set<string> = new Set();

// 由 notes-list 注入的 callback（打破循环依赖）
let reloadList: () => void = () => {};
let getCurrentTab: () => string = () => 'active';

// ===== DOM 引用 =====
const listEl = document.getElementById('list')!;
const batchBar = document.getElementById('batch-bar')!;

// ===== 导出函数 =====

/**
 * 初始化多选模块：注入依赖并绑定事件。
 *
 * 由 notes-list 顶层调用，传入 loadNotes（用于批量操作后刷新）和
 * getCurrentTab（用于切换归档/恢复按钮可见性）。
 */
export function initMultiSelect(deps: {
  reloadList: () => void;
  getCurrentTab: () => string;
}): void {
  reloadList = deps.reloadList;
  getCurrentTab = deps.getCurrentTab;
  bindBatchBarEvents();
  bindEscExit();
}

/** 切换某条便签的选中状态（Ctrl+点击或已有多选时单击） */
export function toggleSelection(id: string): void {
  if (selectedIds.has(id)) {
    selectedIds.delete(id);
  } else {
    selectedIds.add(id);
  }
  updateMultiSelectUI();
}

/** 清空选择（Tab 切换/Esc/批量操作完成时调用） */
export function clearSelection(): void {
  selectedIds.clear();
  updateMultiSelectUI();
}

/** 是否有选中项（列表单击判断是否拦截） */
export function hasSelection(): boolean {
  return selectedIds.size > 0;
}

/** 刷新多选 UI（renderList 后调用，因 DOM 被替换需重新高亮） */
export function refreshSelectionUI(): void {
  updateMultiSelectUI();
}

// ===== 内部函数 =====

/** 绑定批量操作栏 5 个按钮事件 */
function bindBatchBarEvents(): void {
  // 批量归档
  batchBar.querySelector('[data-batch-archive]')?.addEventListener('click', async () => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    try {
      await api.batchArchiveNotes(ids);
      clearSelectionAndReload();
    } catch (err) { console.error('批量归档失败:', err); }
  });

  // 批量恢复（归档 tab）
  batchBar.querySelector('[data-batch-restore]')?.addEventListener('click', async () => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    try {
      await api.batchUnarchiveNotes(ids);
      clearSelectionAndReload();
    } catch (err) { console.error('批量恢复失败:', err); }
  });

  // 批量删除（需确认）
  batchBar.querySelector('[data-batch-delete]')?.addEventListener('click', async () => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    if (!confirm(t('hub.batchDeleteConfirm').replace('{n}', String(ids.length)))) return;
    try {
      await api.batchDeleteNotes(ids);
      clearSelectionAndReload();
    } catch (err) { console.error('批量删除失败:', err); }
  });

  // 批量改色（弹出颜色选择面板）
  batchBar.querySelector('[data-batch-color]')?.addEventListener('click', () => {
    const ids = [...selectedIds];
    if (ids.length === 0) return;
    showColorPicker(ids);
  });

  // 取消多选
  batchBar.querySelector('[data-batch-cancel]')?.addEventListener('click', () => {
    clearSelection();
  });
}

/** Esc 退出多选 */
function bindEscExit(): void {
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && selectedIds.size > 0) {
      clearSelection();
    }
  });
}

/** 弹出颜色选择面板 */
function showColorPicker(ids: string[]): void {
  const overlay = document.createElement('div');
  overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.3);display:flex;align-items:center;justify-content:center;z-index:9999;';
  const panel = document.createElement('div');
  panel.style.cssText = 'background:var(--surface);border-radius:10px;padding:12px;box-shadow:0 8px 28px rgba(0,0,0,0.2);display:flex;gap:8px;flex-wrap:wrap;width:200px;';
  const allColors = BATCH_COLORS;
  Object.entries(allColors).forEach(([name, dot]) => {
    const c = document.createElement('div');
    c.style.cssText = `width:28px;height:28px;border-radius:50%;cursor:pointer;background:${dot};border:2px solid rgba(0,0,0,0.1);transition:transform 0.12s;`;
    c.title = name;
    c.addEventListener('click', async () => {
      try { await api.batchUpdateColor(ids, name); } catch (err) { console.error('批量改色失败:', err); }
      overlay.remove();
      clearSelectionAndReload();
    });
    c.addEventListener('mouseenter', () => c.style.transform = 'scale(1.15)');
    c.addEventListener('mouseleave', () => c.style.transform = 'scale(1)');
    panel.appendChild(c);
  });
  overlay.appendChild(panel);
  overlay.addEventListener('click', (e) => { if (e.target === overlay) overlay.remove(); });
  document.body.appendChild(overlay);
}

/** 更新多选 UI：高亮选中项 + 显示/隐藏批量操作栏 + 切换归档/恢复按钮 */
function updateMultiSelectUI(): void {
  // 高亮/取消高亮
  listEl.querySelectorAll('.note-item').forEach(el => {
    const id = (el as HTMLElement).dataset.id!;
    el.classList.toggle('selected', selectedIds.has(id));
  });
  // 批量操作栏
  if (selectedIds.size > 0) {
    batchBar.style.display = 'flex';
    const countEl = batchBar.querySelector('.batch-count');
    if (countEl) countEl.textContent = String(selectedIds.size);
    // 归档 tab 显示"恢复"，活跃 tab 显示"归档"
    const archiveBtn = batchBar.querySelector('[data-batch-archive]') as HTMLElement;
    const restoreBtn = batchBar.querySelector('[data-batch-restore]') as HTMLElement;
    if (getCurrentTab() === 'archived') {
      if (archiveBtn) archiveBtn.style.display = 'none';
      if (restoreBtn) restoreBtn.style.display = '';
    } else {
      if (archiveBtn) archiveBtn.style.display = '';
      if (restoreBtn) restoreBtn.style.display = 'none';
    }
  } else {
    batchBar.style.display = 'none';
  }
}

/** 清空多选并触发列表刷新（批量操作成功后调用） */
function clearSelectionAndReload(): void {
  clearSelection();
  reloadList();
}
