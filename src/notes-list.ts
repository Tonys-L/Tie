/**
 * 便签列表管理（Hub 页面）：加载/渲染/搜索/排序/标签筛选 + 列表事件委托。
 *
 * 职责边界：
 * - loadNotes 拉取活跃+归档便签，并行加载每条提醒数量，刷新标签侧边栏和列表
 * - renderTagSidebar 渲染左侧标签栏（按便签数量降序）
 * - renderList 渲染便签列表（搜索模式 / 标签筛选 / 排序 / 提醒徽章）
 * - 列表事件委托：归档/恢复/提醒/删除/单击打开/Ctrl+多选
 *
 * 不负责：多选状态与批量操作栏（见 notes-multiselect.ts）、
 *         删除确认弹窗 UI（见 delete-confirm.ts）
 *
 * 被调用方：hub.ts (页面切换/初始加载/语言切换/focus/visibilitychange 调用 loadNotes) +
 *           reminder-dialog.ts (onNotesChanged 回调) +
 *           calendar-view.ts (通过 getActiveNotes/getArchivedNotes 读取便签)
 * 依赖：api.ts + utils.ts (COLOR_MAP/escapeHtml/formatDate) + i18n (t) +
 *       reminder-dialog.ts (showReminderDialog) +
 *       notes-multiselect.ts (initMultiSelect/toggleSelection/clearSelection/hasSelection/refreshSelectionUI) +
 *       delete-confirm.ts (showDeleteConfirm)
 */

import * as api from './api';
import { COLOR_MAP, escapeHtml, formatDate } from './utils';
import { t } from './i18n';
import { showReminderDialog } from './reminder-dialog';
import {
  initMultiSelect,
  toggleSelection,
  clearSelection,
  hasSelection,
  refreshSelectionUI,
} from './notes-multiselect';
import { showDeleteConfirm } from './delete-confirm';

// ===== 模块级 state =====
let currentTab = 'active';
let activeNotes: any[] = [];
let archivedNotes: any[] = [];
let searchQuery = '';
let searchResults: any[] | null = null; // 后端搜索结果缓存
let selectedTag: string | null = null;
let sortBy: 'updated' | 'created' | 'title' = 'updated';

// ===== DOM 引用 =====
const listEl = document.getElementById('list')!;
const searchInput = document.getElementById('search') as HTMLInputElement;
const sortSelect = document.getElementById('sort-select') as HTMLSelectElement;
const tagListEl = document.getElementById('tag-list')!;

// ===== 顶层事件绑定 =====

// Tab 切换（活跃/归档/提醒）
document.querySelectorAll('.mgr-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.mgr-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    currentTab = tab.getAttribute('data-tab') || 'active';
    if (searchInput) { searchInput.value = ''; searchQuery = ''; searchResults = null; }
    // 切换 tab 时清空多选
    clearSelection();
    renderList();
  });
});

// 搜索防抖
let searchTimer: ReturnType<typeof setTimeout> | undefined;
searchInput?.addEventListener('input', () => {
  searchQuery = searchInput.value.trim();
  if (searchTimer) clearTimeout(searchTimer);
  if (!searchQuery) {
    searchResults = null;
    renderList();
    return;
  }
  searchTimer = setTimeout(async () => {
    try {
      const results = await api.searchNotes(searchQuery);
      // 补充提醒数量缓存
      await Promise.allSettled(results.map(async (n: any) => {
        if (n._reminderCount === undefined) {
          try {
            const reminders = await api.getReminders(n.id);
            n._reminderCount = (reminders as any[]).filter(r => r.status === 'pending').length;
          } catch { n._reminderCount = 0; }
        }
      }));
      searchResults = results;
      renderList();
    } catch(e) { console.error('搜索失败:', e); }
  }, 300);
});

// 排序选择
sortSelect?.addEventListener('change', () => {
  sortBy = sortSelect.value as 'updated' | 'created' | 'title';
  renderList();
});

// 列表事件委托（归档/恢复/提醒/删除/单击打开/Ctrl+多选）
listEl.addEventListener('click', async (e) => {
  const archiveBtn = (e.target as HTMLElement).closest('[data-archive]') as HTMLElement;
  const restoreBtn = (e.target as HTMLElement).closest('[data-restore]') as HTMLElement;
  const reminderBtn = (e.target as HTMLElement).closest('[data-reminder]') as HTMLElement;
  const deleteBtn = (e.target as HTMLElement).closest('[data-delete]') as HTMLElement;
  const noteItem = (e.target as HTMLElement).closest('.note-item') as HTMLElement;

  if (archiveBtn) {
    e.stopPropagation();
    try {
      await api.archiveNote(archiveBtn.dataset.archive!);
      loadNotes();
    } catch (err) { console.error('归档失败:', err); }
  } else if (restoreBtn) {
    e.stopPropagation();
    const id = restoreBtn.dataset.restore!;
    try {
      await api.unarchiveNote(id);
      await api.openNote(id);
      loadNotes();
    } catch (err) { console.error('恢复失败:', err); }
  } else if (reminderBtn) {
    e.stopPropagation();
    showReminderDialog(reminderBtn.dataset.reminder!, reminderBtn.dataset.title || t('app.note'), loadNotes);
  } else if (deleteBtn) {
    e.stopPropagation();
    // Hub 列表模式：append 到 body，删除成功后刷新列表
    showDeleteConfirm(deleteBtn.dataset.delete!, document.body, loadNotes);
  } else if (noteItem) {
    const id = noteItem.dataset.id!;
    // Ctrl+点击进入多选 / 已有多选时单击切换选中
    if (e.ctrlKey || e.metaKey || hasSelection()) {
      e.stopPropagation();
      toggleSelection(id);
    } else {
      api.openNote(id);
    }
  }
});

// ===== 导出函数 =====

/** 加载活跃+归档便签，并行加载提醒数量，刷新标签栏和列表 */
export async function loadNotes(): Promise<void> {
  try {
    const [active, archived] = await Promise.all([api.getAllNotes(), api.getArchivedNotes()]);
    activeNotes = active as any[];
    archivedNotes = archived as any[];
    // 保留已有搜索结果的提醒缓存
    if (searchResults) {
      const cached = new Map<string, number>();
      [...activeNotes, ...archivedNotes].forEach(n => {
        if (n._reminderCount !== undefined) cached.set(n.id, n._reminderCount);
      });
      searchResults.forEach(n => {
        if (n._reminderCount === undefined && cached.has(n.id)) {
          n._reminderCount = cached.get(n.id);
        }
      });
    }
    // 并行加载每条便签的提醒数量
    const allNotes = [...activeNotes, ...archivedNotes];
    await Promise.allSettled(allNotes.map(async (n: any) => {
      try {
        const reminders = await api.getReminders(n.id);
        n._reminderCount = (reminders as any[]).filter(r => r.status === 'pending').length;
      } catch { n._reminderCount = 0; }
    }));
    const ca = document.getElementById('count-active');
    const cb = document.getElementById('count-archived');
    const cr = document.getElementById('count-reminders');
    if (ca) ca.textContent = String(activeNotes.length);
    if (cb) cb.textContent = String(archivedNotes.length);
    if (cr) cr.textContent = String([...activeNotes, ...archivedNotes].filter(n => n._reminderCount > 0).length);
    renderTagSidebar();
    renderList();
  } catch(e) { console.error('加载失败:', e); }
}

/** 获取活跃便签（供 calendar-view 读取） */
export function getActiveNotes(): any[] {
  return activeNotes;
}

/** 获取归档便签（供 calendar-view 读取） */
export function getArchivedNotes(): any[] {
  return archivedNotes;
}

// ===== 内部函数 =====

/** 渲染左侧标签栏 */
function renderTagSidebar(): void {
  const allNotes = [...activeNotes, ...archivedNotes];
  const tagMap = new Map<string, number>();
  allNotes.forEach(n => {
    (n.tags || []).forEach((tag: string) => {
      tagMap.set(tag, (tagMap.get(tag) || 0) + 1);
    });
  });
  if (tagMap.size === 0) {
    tagListEl.innerHTML = `<div class="tag-sidebar-empty">${t('hub.noTags')}</div>`;
    return;
  }
  // 按便签数量降序排列
  const sorted = [...tagMap.entries()].sort((a, b) => b[1] - a[1]);
  tagListEl.innerHTML = sorted.map(([tag, count]) =>
    `<div class="tag-sidebar-item ${selectedTag === tag ? 'active' : ''}" data-tag-filter="${escapeHtml(tag)}"><span>${escapeHtml(tag)}</span><span class="tag-count">${count}</span></div>`
  ).join('');
  // 标签筛选点击
  tagListEl.querySelectorAll('[data-tag-filter]').forEach(item => {
    item.addEventListener('click', () => {
      const tag = (item as HTMLElement).dataset.tagFilter!;
      selectedTag = selectedTag === tag ? null : tag;
      renderTagSidebar();
      renderList();
    });
  });
}

/** 渲染便签列表（搜索模式 / 标签筛选 / 排序 / 提醒徽章） */
function renderList(): void {
  let notes: any[];
  let isSearchMode = false;
  if (searchQuery && searchResults) {
    isSearchMode = true;
    notes = searchResults;
  } else if (currentTab === 'reminders') {
    notes = [...activeNotes, ...archivedNotes].filter(n => (n._reminderCount || 0) > 0);
  } else {
    notes = currentTab === 'active' ? activeNotes : archivedNotes;
  }
  // 标签筛选
  if (selectedTag) {
    notes = notes.filter(n => (n.tags || []).includes(selectedTag));
  }
  if (notes.length === 0) {
    const emptyText = searchQuery ? t('hub.noMatch')
      : selectedTag ? t('hub.noMatch')
      : currentTab === 'reminders' ? t('hub.noReminders')
      : currentTab === 'active' ? t('hub.noActive') : t('hub.noArchived');
    listEl.innerHTML = `<div class="empty-state"><svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg><span>${emptyText}</span></div>`;
    refreshSelectionUI();
    return;
  }
  // 排序
  const sorted = [...notes].sort((a, b) => {
    if (sortBy === 'title') {
      const ta = (a.title || t('hub.noTitle')).toLowerCase();
      const tb = (b.title || t('hub.noTitle')).toLowerCase();
      return ta.localeCompare(tb);
    } else if (sortBy === 'created') {
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    }
    return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
  });
  listEl.innerHTML = sorted.map(n => {
    const color = COLOR_MAP[n.color] || COLOR_MAP.amber;
    const title = n.title || t('hub.noTitle');
    // 搜索模式：使用 FTS5 highlight 片段（含 <mark> 标签），否则普通预览
    const preview = isSearchMode && n.highlight
      ? n.highlight
      : (n.content || '').replace(/[#*`>\-\[\]]/g, '').slice(0, 60) || t('hub.noContent');
    const previewHtml = isSearchMode && n.highlight
      ? preview  // highlight 已是 HTML（<mark> 包裹），直接渲染
      : escapeHtml(preview);
    const isArchived = archivedNotes.some(a => a.id === n.id);
    const showTag = isSearchMode || currentTab === 'reminders';
    const tag = showTag ? (isArchived ? `<span class="note-tag archived">${t('hub.archived')}</span>` : `<span class="note-tag active">${t('hub.activeNotes')}</span>`) : '';
    const dateStr = formatDate(n.updated_at);
    const tagsHtml = (n.tags && n.tags.length > 0)
      ? `<div class="note-tags">${n.tags.slice(0, 3).map((tg: string) => `<span class="note-tag-pill">${escapeHtml(tg)}</span>`).join('')}${n.tags.length > 3 ? `<span class="note-tag-pill">+${n.tags.length - 3}</span>` : ''}</div>`
      : '';
    const actionBtn = isArchived
	      ? `<button class="act-btn restore" data-restore="${n.id}" title="${t('hub.restore')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg></button>`
	      : `<button class="act-btn reminder" data-reminder="${n.id}" data-title="${escapeHtml(title)}" title="${t('hub.reminders')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg></button><button class="act-btn archive" data-archive="${n.id}" title="${t('note.archive')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg></button>`;
    const reminderBadge = n._reminderCount > 0 ? `<span class="reminder-badge">${n._reminderCount}</span>` : '';
    return `<div class="note-item" data-id="${n.id}"><div class="note-color" style="background:${color}"></div><div class="note-text"><div class="note-title">${escapeHtml(title)} ${tag}</div><div class="note-preview">${previewHtml}</div>${tagsHtml}</div>${reminderBadge}<span class="note-date">${dateStr}</span><div class="note-actions">${actionBtn}<button class="act-btn delete" data-delete="${n.id}" title="${t('note.delete')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg></button></div></div>`;
  }).join('');
  // 列表 DOM 被替换后重新高亮选中项
  refreshSelectionUI();
}

// ===== 模块初始化 =====
// 向多选模块注入依赖（callback 模式打破循环依赖）
initMultiSelect({
  reloadList: loadNotes,
  getCurrentTab: () => currentTab,
});
