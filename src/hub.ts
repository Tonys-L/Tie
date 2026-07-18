import { open } from '@tauri-apps/plugin-shell';
import { enable as enableAutoStart, disable as disableAutoStart, isEnabled as isAutoStartEnabled } from '@tauri-apps/plugin-autostart';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { check } from '@tauri-apps/plugin-updater';
import type { Reminder, ShortcutConfig, Template } from './types';
import * as api from './api';
import { COLOR_MAP, escapeHtml, formatDate, localISO, quickDate, repeatLabel } from './utils';
import { initLocale, t, applyLocale, getLocale, setLocale, getLocaleTag } from './i18n';

initLocale();

// ===== 通知点击 → 激活对应便签 =====
listen('tauri://notification', (event: any) => {
  const noteId = event?.payload?.data?.note_id || event?.payload?.note_id;
  if (noteId) {
    invoke('activate_note_by_id', { noteId }).catch(err => console.error('激活便签失败:', err));
  }
});

// ===== 主题 =====
const savedTheme = localStorage.getItem('theme') || 'light';
if (savedTheme === 'dark') {
  document.body.classList.add('dark');
  const moon = document.getElementById('icon-moon') as HTMLElement;
  const sun = document.getElementById('icon-sun') as HTMLElement;
  const label = document.getElementById('theme-label') as HTMLElement;
  if (moon) moon.style.display = 'none';
  if (sun) sun.style.display = 'block';
  if (label) label.textContent = t('hub.lightMode');
}

document.getElementById('theme-btn')?.addEventListener('click', () => {
  const isDark = document.body.classList.toggle('dark');
  localStorage.setItem('theme', isDark ? 'dark' : 'light');
  const moon = document.getElementById('icon-moon') as HTMLElement;
  const sun = document.getElementById('icon-sun') as HTMLElement;
  const label = document.getElementById('theme-label') as HTMLElement;
  if (moon) moon.style.display = isDark ? 'none' : 'block';
  if (sun) sun.style.display = isDark ? 'block' : 'none';
  if (label) label.textContent = isDark ? t('hub.lightMode') : t('hub.darkMode');
});

// ===== 关于页 GitHub 链接 =====
document.getElementById('about-github-link')?.addEventListener('click', (e) => {
  e.preventDefault();
  open('https://github.com/Tonys-L/Tie');
});

// ===== 页面切换 =====
document.querySelectorAll('.nav-item').forEach(item => {
  item.addEventListener('click', () => {
    document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    item.classList.add('active');
    const page = document.getElementById('page-' + item.getAttribute('data-page'));
    if (page) page.classList.add('active');
    if (item.getAttribute('data-page') === 'notes') loadNotes();
    if (item.getAttribute('data-page') === 'calendar') loadCalendar();
    if (item.getAttribute('data-page') === 'general') loadGeneralSettings();
    if (item.getAttribute('data-page') === 'sync') loadSyncConfig();
    if (item.getAttribute('data-page') === 'ai') loadAiConfig();
    if (item.getAttribute('data-page') === 'shortcuts') loadShortcutConfig();
  });
});

// ===== 便签管理 =====
let currentTab = 'active';
let activeNotes: any[] = [];
let archivedNotes: any[] = [];
let searchQuery = '';
let searchResults: any[] | null = null; // 后端搜索结果缓存
let selectedTag: string | null = null;
let sortBy: 'updated' | 'created' | 'title' = 'updated';
const listEl = document.getElementById('list')!;
const searchInput = document.getElementById('search') as HTMLInputElement;
const sortSelect = document.getElementById('sort-select') as HTMLSelectElement;
const tagListEl = document.getElementById('tag-list')!;

// ===== 多选状态 =====
let selectedIds: Set<string> = new Set();
const batchBar = document.getElementById('batch-bar')!;

document.querySelectorAll('.mgr-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.mgr-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    currentTab = tab.getAttribute('data-tab') || 'active';
    if (searchInput) { searchInput.value = ''; searchQuery = ''; searchResults = null; }
    // 切换 tab 时清空多选
    selectedIds.clear();
    updateMultiSelectUI();
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

async function loadNotes() {
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

// ===== 标签侧边栏 =====

function renderTagSidebar() {
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

function renderList() {
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
}

// 事件委托
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
    showReminderDialog(reminderBtn.dataset.reminder!, reminderBtn.dataset.title || t('app.note'));
  } else if (deleteBtn) {
    e.stopPropagation();
    showDeleteConfirm(deleteBtn.dataset.delete!);
  } else if (noteItem) {
    const id = noteItem.dataset.id!;
    // Ctrl+点击进入多选模式
    if (e.ctrlKey || e.metaKey) {
      e.stopPropagation();
      if (selectedIds.has(id)) {
        selectedIds.delete(id);
      } else {
        selectedIds.add(id);
      }
      updateMultiSelectUI();
    } else if (selectedIds.size > 0) {
      // 已有多选时，单击切换选中
      e.stopPropagation();
      if (selectedIds.has(id)) {
        selectedIds.delete(id);
      } else {
        selectedIds.add(id);
      }
      updateMultiSelectUI();
    } else {
      api.openNote(id);
    }
  }
});

// Esc 退出多选
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && selectedIds.size > 0) {
    selectedIds.clear();
    updateMultiSelectUI();
  }
});

/** 更新多选 UI：高亮选中项 + 显示/隐藏批量操作栏 */
function updateMultiSelectUI() {
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
    if (currentTab === 'archived') {
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

/** 清空多选并刷新列表 */
function clearSelectionAndReload() {
  selectedIds.clear();
  updateMultiSelectUI();
  loadNotes();
}

// ===== 提醒设置弹窗（在 Hub 页面内，不打开便签窗口）=====

function showReminderDialog(noteId: string, noteTitle: string) {
  const existing = document.getElementById('reminder-overlay');
  if (existing) existing.remove();

  const defaultTime = new Date(Date.now() + 3600000);

  const overlay = document.createElement('div');
  overlay.id = 'reminder-overlay';
  overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.35);backdrop-filter:blur(2px);display:flex;align-items:center;justify-content:center;z-index:9999;';

  const dialog = document.createElement('div');
  dialog.style.cssText = 'background:var(--surface);border-radius:12px;padding:16px;box-shadow:0 8px 32px rgba(0,0,0,0.2);width:300px;';
  dialog.innerHTML = `
    <div style="display:flex;align-items:center;gap:6px;margin-bottom:12px;">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>
      <span style="font-size:13px;font-weight:600;color:var(--text-title);flex:1;">${t('hub.reminderFor')}${escapeHtml(noteTitle)}</span>
      <button id="rm-close" style="border:none;background:none;color:var(--text-muted);font-size:18px;cursor:pointer;padding:0 4px;line-height:1;">&times;</button>
    </div>
    <div style="display:flex;gap:6px;margin-bottom:10px;">
      <button class="qbtn" data-quick="1h" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.oneHour')}</button>
	      <button class="qbtn" data-quick="3h" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.threeHours')}</button>
	      <button class="qbtn" data-quick="tomorrow" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.tomorrow')}</button>
	      <button class="qbtn" data-quick="week" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.nextMonday')}</button>
    </div>
    <input type="datetime-local" id="rm-datetime" value="${localISO(defaultTime)}" style="width:100%;box-sizing:border-box;padding:6px 8px;border:1px solid var(--border);border-radius:6px;font-size:13px;outline:none;color:var(--text-title);background:var(--surface);margin-bottom:10px;font-family:inherit;">
    <div style="display:flex;gap:6px;margin-bottom:10px;">
      <button class="rbtn active" data-repeat="none" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.once')}</button>
	      <button class="rbtn" data-repeat="daily" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.daily')}</button>
	      <button class="rbtn" data-repeat="weekly" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.weekly')}</button>
	      <button class="rbtn" data-repeat="monthly" style="flex:1;padding:5px 0;border:1px solid var(--border);border-radius:6px;background:var(--surface);color:var(--text);font-size:12px;cursor:pointer;font-family:inherit;">${t('note.monthly')}</button>
    </div>
    <div id="rm-existing" style="margin-bottom:10px;font-size:12px;color:var(--text-muted);"></div>
    <button id="rm-save" style="width:100%;padding:8px 0;border:none;border-radius:8px;background:#3b82f6;color:#fff;font-size:13px;font-weight:500;cursor:pointer;font-family:inherit;">${t('note.setReminder')}</button>
  `;
  overlay.appendChild(dialog);
  document.body.appendChild(overlay);

  // 激活样式
  const style = document.createElement('style');
  style.textContent = '.rbtn.active{background:#3b82f6!important;color:#fff!important;border-color:#3b82f6!important;}';
  dialog.appendChild(style);

  let selectedRepeat = 'none';

  // 加载已有提醒
  loadExistingReminders(noteId);

  // 关闭
  dialog.querySelector('#rm-close')!.addEventListener('click', () => overlay.remove());
  overlay.addEventListener('click', (e) => { if (e.target === overlay) overlay.remove(); });

  // 快捷时间
  dialog.querySelectorAll('[data-quick]').forEach(btn => {
    btn.addEventListener('click', () => {
      const input = dialog.querySelector('#rm-datetime') as HTMLInputElement;
      const type = (btn as HTMLElement).dataset.quick!;
      input.value = localISO(quickDate(type));
    });
  });

  // 重复选择
  dialog.querySelectorAll('[data-repeat]').forEach(btn => {
    btn.addEventListener('click', () => {
      dialog.querySelectorAll('[data-repeat]').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      selectedRepeat = (btn as HTMLElement).dataset.repeat!;
    });
  });

  // AI 自然语言解析已移至便签保存后的自动嗅探气泡，此处仅保留手动表单
  const reminderTitle = noteTitle;

  // 保存
  dialog.querySelector('#rm-save')!.addEventListener('click', async () => {
    const input = dialog.querySelector('#rm-datetime') as HTMLInputElement;
    const dt = new Date(input.value);
    if (isNaN(dt.getTime())) return;
    try {
      await api.createReminder(noteId, reminderTitle, dt.toISOString(), selectedRepeat);
      overlay.remove();
      loadNotes();
    } catch (e) { console.error('创建提醒失败:', e); }
  });
}

async function loadExistingReminders(noteId: string) {
  try {
    const reminders = await api.getReminders(noteId);
    const active = reminders.filter((r: Reminder) => r.status === 'pending');
    const container = document.getElementById('rm-existing')!;
    if (active.length === 0) { container.innerHTML = ''; return; }
    const label = (t: string) => t === 'none' ? '' : ` · ${repeatLabel(t)}`;
    container.innerHTML = active.map((r: Reminder) => {
      const dt = new Date(r.remind_at).toLocaleString(getLocaleTag(), { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
      return `<div style="display:flex;align-items:center;justify-content:space-between;padding:4px 0;border-bottom:1px solid var(--border-light);"><span>${dt}${label(r.repeat_type)}</span><button class="rm-del" data-id="${r.id}" style="border:none;background:none;color:#ef4444;cursor:pointer;font-size:14px;padding:0 4px;">&times;</button></div>`;
    }).join('');
    container.querySelectorAll('.rm-del').forEach(btn => {
      btn.addEventListener('click', async () => {
        try {
          await api.deleteReminder((btn as HTMLElement).dataset.id!);
          loadExistingReminders(noteId);
          loadNotes();
        } catch (e) { console.error('删除提醒失败:', e); }
      });
    });
  } catch (e) { console.error('加载提醒失败:', e); }
}

function showDeleteConfirm(id: string) {
  const existing = document.getElementById('confirm-overlay');
  if (existing) existing.remove();
  const overlay = document.createElement('div');
  overlay.id = 'confirm-overlay';
  overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.35);backdrop-filter:blur(2px);display:flex;align-items:center;justify-content:center;z-index:9999;';
  const dialog = document.createElement('div');
  dialog.style.cssText = 'background:var(--surface);border-radius:12px;padding:20px;box-shadow:0 8px 32px rgba(0,0,0,0.2);width:280px;text-align:center;';
  dialog.innerHTML = `<div style="font-size:14px;font-weight:500;color:var(--text-title);margin-bottom:6px;">${t('hub.deleteConfirm')}</div><div style="font-size:12px;color:var(--text-muted);margin-bottom:16px;">${t('hub.deleteIrreversible')}</div><div style="display:flex;gap:8px;"><button id="cf-cancel" style="flex:1;padding:7px 0;border:1px solid var(--border);border-radius:7px;background:var(--surface);color:var(--text);font-size:13px;cursor:pointer;font-family:inherit;">${t('note.cancel')}</button><button id="cf-ok" style="flex:1;padding:7px 0;border:none;border-radius:7px;background:#ef4444;color:#fff;font-size:13px;cursor:pointer;font-family:inherit;">${t('note.deleteBtn')}</button></div>`;
  overlay.appendChild(dialog);
  document.body.appendChild(overlay);

  dialog.querySelector('#cf-cancel')!.addEventListener('click', () => overlay.remove());
  dialog.querySelector('#cf-ok')!.addEventListener('click', async () => {
    overlay.remove();
    try { await api.deleteNote(id); loadNotes(); }
    catch (err) { console.error('删除失败:', err); }
  });
  overlay.addEventListener('click', (e) => { if (e.target === overlay) overlay.remove(); });
}

// ===== 同步设置 =====
let syncConfigLoaded = false;
async function loadSyncConfig() {
  if (syncConfigLoaded) return;
  syncConfigLoaded = true;

  document.getElementById('gitee-link')?.addEventListener('click', (e) => { e.preventDefault(); open('https://gitee.com/profile/personal_access_tokens'); });
  document.getElementById('github-link')?.addEventListener('click', (e) => { e.preventDefault(); open('https://github.com/settings/tokens'); });

  const gitInstalled = await api.checkGit();
  const gitEl = document.getElementById('git-status')!;
  if (gitInstalled) {
    gitEl.className = 'status-card ok';
    try {
      const config = await api.getSyncConfig();
      const branch = config.branch || 'main';
      document.getElementById('git-status-text')!.textContent = `${t('hub.gitInstalled')} [${branch}]`;
    } catch {
      document.getElementById('git-status-text')!.textContent = t('hub.gitInstalled');
    }
  }
  else { gitEl.className = 'status-card err'; document.getElementById('git-status-text')!.textContent = t('hub.gitNotInstalled'); }
  (gitEl as HTMLElement).style.display = 'flex';

  try {
    const config = await api.getSyncConfig();
    (document.getElementById('repo-url') as HTMLInputElement).value = config.repo_url || '';
    (document.getElementById('username') as HTMLInputElement).value = config.username || '';
    (document.getElementById('token') as HTMLInputElement).value = config.token || '';
    (document.getElementById('branch') as HTMLInputElement).value = config.branch || 'main';
    if (config.auto_sync) document.getElementById('auto-sync')!.classList.add('on');
  } catch (e) { console.error('加载配置失败:', e); }

  document.getElementById('auto-sync')?.addEventListener('click', () => { document.getElementById('auto-sync')!.classList.toggle('on'); });

  function getSyncConfig() {
    return {
      repo_url: (document.getElementById('repo-url') as HTMLInputElement).value.trim(),
      username: (document.getElementById('username') as HTMLInputElement).value.trim(),
      token: (document.getElementById('token') as HTMLInputElement).value.trim(),
      branch: (document.getElementById('branch') as HTMLInputElement).value.trim() || 'main',
      auto_sync: document.getElementById('auto-sync')!.classList.contains('on'),
    };
  }

  document.getElementById('save-btn')?.addEventListener('click', async () => {
    try { await api.saveSyncConfig(getSyncConfig()); showSyncStatus(t('hub.configSaved'), 'ok'); }
	    catch (e) { showSyncStatus(t('hub.saveFailed') + ': ' + e, 'err'); }
  });

  document.getElementById('sync-btn')?.addEventListener('click', async () => {
    const btn = document.getElementById('sync-btn') as HTMLElement;
    // 全屏蒙层
    const overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.25);backdrop-filter:blur(2px);display:flex;align-items:center;justify-content:center;z-index:9999;';
    overlay.innerHTML = `<span style="color:var(--text-title);font-size:14px;font-weight:500;background:var(--surface);padding:12px 24px;border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,0.15);">${t('hub.syncing')}</span>`;
    document.body.appendChild(overlay);
    btn.style.opacity = '0.6'; btn.style.pointerEvents = 'none';
    try {
      // 先保存配置，再执行同步
      await api.saveSyncConfig(getSyncConfig());
      const result = await api.syncNotes() as string;
      console.log('[同步] 结果:', result);
      const branch = (document.getElementById('branch') as HTMLInputElement)?.value || 'main';
      showSyncStatus(`${result} [${branch}]`, 'ok');
    } catch (e: any) {
      console.error('[同步] 失败:', e);
      const errMsg = String(e);
      // 检测分支不存在错误，提示用户是否创建分支
      if (errMsg.startsWith('BRANCH_NOT_FOUND:')) {
        const existingBranches = errMsg.substring('BRANCH_NOT_FOUND:'.length);
        const branchInput = document.getElementById('branch') as HTMLInputElement;
        const branchName = branchInput?.value || 'main';
        // 移除蒙层和恢复按钮
        if (overlay.parentNode) overlay.parentNode.removeChild(overlay);
        btn.style.opacity = ''; btn.style.pointerEvents = '';
        showBranchCreateDialog(branchName, existingBranches, async () => {
          // 用户确认创建分支
          const overlay2 = document.createElement('div');
          overlay2.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.25);backdrop-filter:blur(2px);display:flex;align-items:center;justify-content:center;z-index:9999;';
          overlay2.innerHTML = `<span style="color:var(--text-title);font-size:14px;font-weight:500;background:var(--surface);padding:12px 24px;border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,0.15);">${t('hub.syncing')}</span>`;
          document.body.appendChild(overlay2);
          btn.style.opacity = '0.6'; btn.style.pointerEvents = 'none';
          try {
            const result2 = await api.syncNotes(true) as string;
            showSyncStatus(result2, 'ok');
          } catch (e2: any) {
            showSyncStatus(t('hub.syncFailed') + ': ' + e2, 'err');
          } finally {
            if (overlay2.parentNode) overlay2.parentNode.removeChild(overlay2);
            btn.style.opacity = ''; btn.style.pointerEvents = '';
          }
        });
      } else {
	      showSyncStatus(t('hub.syncFailed') + ': ' + e, 'err');
      }
    } finally {
      if (overlay.parentNode) overlay.parentNode.removeChild(overlay);
      btn.style.opacity = ''; btn.style.pointerEvents = '';
    }
  });

  function showSyncStatus(msg: string, type: string) {
    const el = document.getElementById('sync-status')!;
    el.className = 'status-card ' + type;
    document.getElementById('sync-status-text')!.textContent = msg;
    (el as HTMLElement).style.display = 'flex';
    if (type !== 'loading') setTimeout(() => { (el as HTMLElement).style.display = 'none'; }, 5000);
  }

  function showBranchCreateDialog(branch: string, existingBranches: string, onConfirm: () => void) {
    const dialog = document.createElement('div');
    dialog.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.4);display:flex;align-items:center;justify-content:center;z-index:10000;';
    dialog.innerHTML = `
      <div style="background:var(--surface);border-radius:12px;padding:28px 32px;max-width:440px;box-shadow:0 8px 32px rgba(0,0,0,0.2);">
        <div style="font-size:16px;font-weight:600;color:var(--text-title);margin-bottom:12px;">${t('hub.branchNotFoundTitle')}</div>
        <div style="font-size:13px;color:var(--text-body);line-height:1.6;margin-bottom:8px;">
          ${t('hub.branchNotFoundMsg').replace('{branch}', branch).replace('{existing}', existingBranches)}
        </div>
        <div style="font-size:13px;color:var(--text-body);line-height:1.6;margin-bottom:20px;">
          ${t('hub.branchCreateConfirm')}
        </div>
        <div style="display:flex;gap:12px;justify-content:flex-end;">
          <button id="bc-cancel" style="padding:8px 20px;border:1px solid var(--border);border-radius:6px;background:transparent;color:var(--text-body);cursor:pointer;font-size:13px;">${t('hub.branchCancel')}</button>
          <button id="bc-ok" style="padding:8px 20px;border:none;border-radius:6px;background:var(--accent);color:#fff;cursor:pointer;font-size:13px;">${t('hub.branchCreate')}</button>
        </div>
      </div>
    `;
    document.body.appendChild(dialog);
    dialog.querySelector('#bc-cancel')!.addEventListener('click', () => { dialog.remove(); });
    dialog.querySelector('#bc-ok')!.addEventListener('click', () => { dialog.remove(); onConfirm(); });
  }
}

// ===== 快捷键设置 =====
let shortcutConfigLoaded = false;
async function loadShortcutConfig() {
  if (shortcutConfigLoaded) return;
  shortcutConfigLoaded = true;

  try {
    const config = await api.getShortcutConfig();
    (document.getElementById('shortcut-new-note') as HTMLInputElement).value = config.new_note;
    (document.getElementById('shortcut-show-all') as HTMLInputElement).value = config.show_all;
    (document.getElementById('shortcut-toggle-hub') as HTMLInputElement).value = config.toggle_hub || 'ctrl+shift+h';
  } catch (e) { console.error('加载快捷键配置失败:', e); }

  function getShortcutConfig(): ShortcutConfig {
    return {
      new_note: (document.getElementById('shortcut-new-note') as HTMLInputElement).value.trim().toLowerCase(),
      show_all: (document.getElementById('shortcut-show-all') as HTMLInputElement).value.trim().toLowerCase(),
      toggle_hub: (document.getElementById('shortcut-toggle-hub') as HTMLInputElement).value.trim().toLowerCase(),
    };
  }

  function showShortcutStatus(msg: string, type: string) {
    const el = document.getElementById('shortcut-status')!;
    el.className = 'status-card ' + type;
    document.getElementById('shortcut-status-text')!.textContent = msg;
    (el as HTMLElement).style.display = 'flex';
    setTimeout(() => { (el as HTMLElement).style.display = 'none'; }, 3000);
  }

  document.getElementById('shortcut-save-btn')?.addEventListener('click', async () => {
    const config = getShortcutConfig();
    if (!config.new_note || !config.show_all || !config.toggle_hub) {
      showShortcutStatus(t('hub.shortcutEmpty'), 'err');
      return;
    }
    try {
      await api.saveShortcutConfig(config);
      showShortcutStatus(t('hub.shortcutSaved'), 'ok');
    } catch (e) {
      showShortcutStatus(t('hub.saveFailed') + ': ' + e, 'err');
    }
  });

  document.getElementById('shortcut-reset-btn')?.addEventListener('click', () => {
    (document.getElementById('shortcut-new-note') as HTMLInputElement).value = 'ctrl+shift+n';
    (document.getElementById('shortcut-show-all') as HTMLInputElement).value = 'ctrl+shift+s';
    (document.getElementById('shortcut-toggle-hub') as HTMLInputElement).value = 'ctrl+shift+h';
  });
}

// ===== 日历视图 =====
let calLoaded = false;
let calYear = new Date().getFullYear();
let calMonth = new Date().getMonth() + 1;
let calReminders: Reminder[] = [];
let calSelectedDate: string | null = null;
let calView: 'month' | 'year' = 'month';
let calLunarMap = new Map<number, string>();
let calNoteActivityDays = new Set<number>();
let calCreateRepeat = 'none';

async function loadCalendar() {
  if (!calLoaded) {
    calLoaded = true;
    document.getElementById('cal-prev')?.addEventListener('click', () => {
      if (calView === 'year') {
        calYear--;
      } else {
        if (calMonth === 1) { calMonth = 12; calYear--; } else calMonth--;
        calSelectedDate = null;
      }
      renderCalendar();
    });
    document.getElementById('cal-next')?.addEventListener('click', () => {
      if (calView === 'year') {
        calYear++;
      } else {
        if (calMonth === 12) { calMonth = 1; calYear++; } else calMonth++;
        calSelectedDate = null;
      }
      renderCalendar();
    });
    document.querySelectorAll('.cal-view-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        const v = (btn as HTMLElement).dataset.view as 'month' | 'year';
        calView = v;
        document.querySelectorAll('.cal-view-btn').forEach(b => b.classList.toggle('active', b === btn));
        renderCalendar();
      });
    });
    document.getElementById('cal-modal-cancel')?.addEventListener('click', closeCreateReminderModal);
    document.getElementById('cal-modal-create')?.addEventListener('click', createReminderFromCalendar);
  }
  await renderCalendar();
}

async function renderCalendar() {
  const titleEl = document.getElementById('cal-title');
  const monthView = document.getElementById('cal-month-view');
  const yearView = document.getElementById('cal-year-view');
  if (calView === 'year') {
    if (titleEl) titleEl.textContent = getLocale() === 'zh' ? `${calYear}年` : `${calYear}`;
    if (monthView) monthView.style.display = 'none';
    if (yearView) yearView.style.display = 'block';
    await renderYearView();
  } else {
    if (titleEl) titleEl.textContent = getLocale() === 'zh' ? `${calYear}年${calMonth}月` : `${calMonth}/${calYear}`;
    if (monthView) monthView.style.display = 'flex';
    if (yearView) yearView.style.display = 'none';
    await renderMonthView();
  }
}

async function renderMonthView() {
  const weekdaysEl = document.getElementById('cal-weekdays');
  if (weekdaysEl) {
    const isZh = getLocale() === 'zh';
    const names = isZh ? ['日','一','二','三','四','五','六'] : ['Sun','Mon','Tue','Wed','Thu','Fri','Sat'];
    weekdaysEl.innerHTML = names.map(n => `<span>${n}</span>`).join('');
  }

  // 并行加载提醒、农历、便签活动
  try {
    const [reminders, lunarDates, noteDays] = await Promise.all([
      api.getRemindersByMonth(calYear, calMonth),
      api.getLunarDates(calYear, calMonth),
      api.getNotesActivityByMonth(calYear, calMonth),
    ]);
    calReminders = reminders;
    calLunarMap = new Map(lunarDates.map(d => [d.day, d.lunar_text]));
    calNoteActivityDays = new Set(noteDays);
  } catch (e) {
    console.error('加载日历数据失败:', e);
    calReminders = [];
    calLunarMap = new Map();
    calNoteActivityDays = new Set();
  }

  const remindersByDay = new Map<number, Reminder[]>();
  calReminders.forEach(r => {
    const d = new Date(r.remind_at);
    if (d.getFullYear() === calYear && d.getMonth() + 1 === calMonth) {
      const day = d.getDate();
      if (!remindersByDay.has(day)) remindersByDay.set(day, []);
      remindersByDay.get(day)!.push(r);
    }
  });

  const gridEl = document.getElementById('cal-grid');
  if (!gridEl) return;

  const startWeekday = new Date(calYear, calMonth - 1, 1).getDay();
  const daysInMonth = new Date(calYear, calMonth, 0).getDate();
  const today = new Date();
  const isCurrentMonth = today.getFullYear() === calYear && today.getMonth() + 1 === calMonth;

  // 本周范围
  const dow = today.getDay();
  const weekStart = new Date(today); weekStart.setDate(today.getDate() - dow); weekStart.setHours(0,0,0,0);
  const weekEnd = new Date(weekStart); weekEnd.setDate(weekStart.getDate() + 6); weekEnd.setHours(23,59,59,999);

  let html = '';
  const prevMonthDays = new Date(calYear, calMonth - 1, 0).getDate();
  for (let i = startWeekday - 1; i >= 0; i--) {
    html += `<div class="cal-day other-month"><div class="cal-day-top"><span class="cal-day-num">${prevMonthDays - i}</span></div></div>`;
  }
  for (let d = 1; d <= daysInMonth; d++) {
    const isToday = isCurrentMonth && d === today.getDate();
    const isSelected = calSelectedDate === `${calYear}-${calMonth}-${d}`;
    const dateObj = new Date(calYear, calMonth - 1, d);
    const isThisWeek = dateObj >= weekStart && dateObj <= weekEnd;
    const dayReminders = remindersByDay.get(d) || [];
    const lunarText = calLunarMap.get(d) || '';
    const hasNote = calNoteActivityDays.has(d);

    const remindersHtml = dayReminders.slice(0, 2).map(r => {
      const time = new Date(r.remind_at).toLocaleTimeString(getLocaleTag(), { hour: '2-digit', minute: '2-digit' });
      const status = r.status || 'pending';
      return `<div class="cal-day-reminder ${status}">${time} ${escapeHtml(r.note_title)}</div>`;
    }).join('') + (dayReminders.length > 2 ? `<div class="cal-day-more">+${dayReminders.length - 2}</div>` : '');

    html += `<div class="cal-day${isToday ? ' today' : ''}${isThisWeek ? ' this-week' : ''}${isSelected ? ' selected' : ''}" data-day="${d}">
      <div class="cal-day-top"><span class="cal-day-num">${d}</span><span class="cal-day-lunar">${lunarText}</span></div>
      <div class="cal-day-reminders">${remindersHtml}</div>
      ${hasNote ? '<div class="cal-day-note-dot"></div>' : ''}
    </div>`;
  }
  const remaining = 42 - (startWeekday + daysInMonth);
  for (let d = 1; d <= remaining; d++) {
    html += `<div class="cal-day other-month"><div class="cal-day-top"><span class="cal-day-num">${d}</span></div></div>`;
  }
  gridEl.innerHTML = html;

  gridEl.querySelectorAll('.cal-day[data-day]').forEach(el => {
    el.addEventListener('click', () => {
      const day = parseInt((el as HTMLElement).dataset.day!);
      calSelectedDate = `${calYear}-${calMonth}-${day}`;
      renderMonthView();
      showDayDetail(day);
    });
  });

  if (calSelectedDate) {
    const day = parseInt(calSelectedDate.split('-')[2]);
    showDayDetail(day);
  }
}

async function renderYearView() {
  const gridEl = document.getElementById('cal-year-grid');
  if (!gridEl) return;

  // 并行加载全年提醒
  const monthData = await Promise.all(
    Array.from({ length: 12 }, (_, i) =>
      api.getRemindersByMonth(calYear, i + 1).catch(() => [])
    )
  );

  const today = new Date();
  let html = '';
  for (let m = 1; m <= 12; m++) {
    const reminders = monthData[m - 1];
    const reminderDays = new Set<number>();
    reminders.forEach(r => {
      const d = new Date(r.remind_at);
      if (d.getFullYear() === calYear && d.getMonth() + 1 === m) reminderDays.add(d.getDate());
    });
    const startWd = new Date(calYear, m - 1, 1).getDay();
    const daysInM = new Date(calYear, m, 0).getDate();
    const isCurrentMonth = today.getFullYear() === calYear && today.getMonth() + 1 === m;

    let daysHtml = '';
    for (let i = 0; i < startWd; i++) daysHtml += '<div class="cal-year-month-day"></div>';
    for (let d = 1; d <= daysInM; d++) {
      const isToday = isCurrentMonth && d === today.getDate();
      const hasR = reminderDays.has(d);
      daysHtml += `<div class="cal-year-month-day${isToday ? ' today' : ''}${hasR ? ' has-reminder' : ''}">${d}</div>`;
    }

    html += `<div class="cal-year-month" data-month="${m}">
      <div class="cal-year-month-title">${getLocale() === 'zh' ? `${m}月` : monthNamesEn[m - 1]}</div>
      <div class="cal-year-month-grid">${daysHtml}</div>
    </div>`;
  }
  gridEl.innerHTML = html;

  gridEl.querySelectorAll('.cal-year-month').forEach(el => {
    el.addEventListener('click', () => {
      calMonth = parseInt((el as HTMLElement).dataset.month!);
      calView = 'month';
      document.querySelectorAll('.cal-view-btn').forEach(b => b.classList.toggle('active', (b as HTMLElement).dataset.view === 'month'));
      renderCalendar();
    });
  });
}

const monthNamesEn = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];

function showDayDetail(day: number) {
  const detailEl = document.getElementById('cal-detail');
  if (!detailEl) return;
  const dayReminders = calReminders.filter(r => {
    const d = new Date(r.remind_at);
    return d.getDate() === day && d.getMonth() + 1 === calMonth && d.getFullYear() === calYear;
  });
  // 过滤当天更新的便签（按 updated_at 本地日期匹配）
  const dayNotes = [...activeNotes, ...archivedNotes].filter(n => {
    const d = new Date(n.updated_at);
    return d.getDate() === day && d.getMonth() + 1 === calMonth && d.getFullYear() === calYear;
  });
  const lunarText = calLunarMap.get(day) || '';
  const dateHeader = getLocale() === 'zh' ? `${calMonth}/${day} ${lunarText}` : `${calMonth}/${day}`;
  let html = `<div class="cal-detail-title">${dateHeader}</div>`;
  // 提醒区块
  if (dayReminders.length === 0) {
    html += `<div class="cal-empty">${t('hub.noRemindersOnDay')}</div>`;
  } else {
    html += dayReminders.map(r => {
      const dt = new Date(r.remind_at).toLocaleTimeString(getLocaleTag(), { hour: '2-digit', minute: '2-digit' });
      const repeat = r.repeat_type !== 'once' && r.repeat_type !== 'none' ? ` · ${repeatLabel(r.repeat_type)}` : '';
      const status = r.status || 'pending';
      return `<div class="cal-reminder-item"><span class="cal-reminder-status ${status}"></span><span class="cal-reminder-time">${dt}</span><span class="cal-reminder-title">${escapeHtml(r.note_title)}</span><span class="cal-reminder-repeat">${repeat}</span></div>`;
    }).join('');
  }
  // 当天便签区块
  html += `<div class="cal-day-notes-title">${t('hub.dayNotes')}</div>`;
  if (dayNotes.length === 0) {
    html += `<div class="cal-empty">${t('hub.noNotesOnDay')}</div>`;
  } else {
    html += dayNotes.map(n =>
      `<div class="cal-detail-note-item" data-note-id="${n.id}"><span class="cal-detail-note-dot"></span><span class="cal-detail-note-text">${escapeHtml(n.title) || t('note.untitled')}</span></div>`
    ).join('');
  }
  detailEl.innerHTML = html;
  // 便签点击事件：打开便签窗口
  detailEl.querySelectorAll('.cal-detail-note-item').forEach(el => {
    el.addEventListener('click', () => {
      const noteId = (el as HTMLElement).dataset.noteId!;
      invoke('activate_note_by_id', { noteId }).catch(err => console.error('激活便签失败:', err));
    });
  });
  // 添加"创建提醒"按钮
  const createBtn = document.createElement('button');
  createBtn.className = 'btn-secondary cal-create-btn';
  createBtn.style.cssText = 'margin-top:8px;padding:4px 12px;font-size:11px;width:100%;';
  createBtn.textContent = '+ ' + t('hub.createReminder');
  createBtn.addEventListener('click', () => openCreateReminderModal(day));
  detailEl.appendChild(createBtn);
  detailEl.classList.add('show');
}

async function openCreateReminderModal(day: number) {
  const modal = document.getElementById('cal-create-modal') as HTMLElement;
  const dateEl = document.getElementById('cal-modal-date');
  const selectEl = document.getElementById('cal-modal-note-select') as HTMLSelectElement;
  const timeEl = document.getElementById('cal-modal-time') as HTMLInputElement;
  const repeatsEl = document.getElementById('cal-modal-repeats');

  if (dateEl) {
    const lunar = calLunarMap.get(day) || '';
    dateEl.textContent = getLocale() === 'zh' ? `${calYear}年${calMonth}月${day}日 ${lunar}` : `${calYear}-${calMonth}-${day}`;
  }

  // 加载便签列表
  try {
    const notes = await api.getAllNotes();
    if (selectEl) {
      selectEl.innerHTML = notes.map(n => `<option value="${n.id}">${escapeHtml(n.title)}</option>`).join('');
    }
  } catch (e) {
    console.error('加载便签列表失败:', e);
  }

  // 默认时间
  const pad = (n: number) => String(n).padStart(2, '0');
  if (timeEl) {
    timeEl.value = `${calYear}-${pad(calMonth)}-${pad(day)}T09:00`;
  }

  // 重复类型按钮
  calCreateRepeat = 'none';
  if (repeatsEl) {
    const types = [
      { key: 'none', label: t('note.once') },
      { key: 'daily', label: t('note.daily') },
      { key: 'weekly', label: t('note.weekly') },
      { key: 'monthly', label: t('note.monthly') },
      { key: 'lunar_monthly', label: t('note.lunarMonthly') },
    ];
    repeatsEl.innerHTML = types.map(tp => `<button class="rbtn${tp.key === 'none' ? ' active' : ''}" data-repeat="${tp.key}">${tp.label}</button>`).join('');
    repeatsEl.querySelectorAll('.rbtn').forEach(btn => {
      btn.addEventListener('click', () => {
        calCreateRepeat = (btn as HTMLElement).dataset.repeat!;
        repeatsEl.querySelectorAll('.rbtn').forEach(b => b.classList.toggle('active', b === btn));
      });
    });
  }

  modal.style.display = 'flex';
}

function closeCreateReminderModal() {
  const modal = document.getElementById('cal-create-modal') as HTMLElement;
  modal.style.display = 'none';
}

async function createReminderFromCalendar() {
  const selectEl = document.getElementById('cal-modal-note-select') as HTMLSelectElement;
  const timeEl = document.getElementById('cal-modal-time') as HTMLInputElement;
  const noteId = selectEl?.value;
  const timeVal = timeEl?.value;

  if (!noteId) return;
  if (!timeVal) return;

  // datetime-local → ISO
  const dt = new Date(timeVal);
  const iso = dt.toISOString();

  try {
    // 获取便签标题
    const note = await api.getNote(noteId);
    await api.createReminder(noteId, note.title, iso, calCreateRepeat);
    closeCreateReminderModal();
    await renderMonthView();
    if (calSelectedDate) {
      const day = parseInt(calSelectedDate.split('-')[2]);
      showDayDetail(day);
    }
  } catch (e) {
    console.error('创建提醒失败:', e);
    alert('创建提醒失败: ' + e);
  }
}

// ===== 通用设置 =====
let generalSettingsLoaded = false;
async function loadGeneralSettings() {
  if (generalSettingsLoaded) return;
  generalSettingsLoaded = true;

  try {
    const enabled = await isAutoStartEnabled();
    if (enabled) document.getElementById('auto-start')!.classList.add('on');
  } catch (e) { console.error('获取自启状态失败:', e); }

  document.getElementById('auto-start')?.addEventListener('click', async () => {
    const el = document.getElementById('auto-start')!;
    const turningOn = !el.classList.contains('on');
    try {
      if (turningOn) {
        await enableAutoStart();
        el.classList.add('on');
      } else {
        await disableAutoStart();
        el.classList.remove('on');
      }
    } catch (e) { console.error('设置自启失败:', e); }
  });

  // 数据目录路径
  try {
    const dir = await api.getDataDir();
    const dirEl = document.getElementById('data-dir-path');
    if (dirEl) dirEl.textContent = dir;
  } catch (e) { console.error('获取数据目录失败:', e); }

  document.getElementById('open-data-dir')?.addEventListener('click', async () => {
    try {
      await api.openDataDir();
    } catch (e) { console.error('打开数据目录失败:', e); }
  });
}

// ===== AI 配置 =====
let aiConfigLoaded = false;

function showToast(msg: string, type: 'ok' | 'err' = 'ok') {
  const toast = document.createElement('div');
  const bg = type === 'ok' ? '#22c55e' : '#ef4444';
  toast.style.cssText = `position:fixed;top:24px;left:50%;transform:translateX(-50%);padding:10px 20px;border-radius:8px;background:${bg};color:#fff;font-size:13px;font-weight:500;z-index:100000;box-shadow:0 4px 16px rgba(0,0,0,0.2);font-family:inherit;max-width:80vw;`;
  toast.textContent = msg;
  document.body.appendChild(toast);
  setTimeout(() => {
    toast.style.transition = 'opacity 0.3s';
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 300);
  }, 2500);
}

async function loadAiConfig() {
  // 每次进入页面都刷新表单（配置可能在其他地方被修改）
  try {
    const config = await api.getAiConfig();
    (document.getElementById('ai-base-url') as HTMLInputElement).value = config.base_url || '';
    (document.getElementById('ai-api-key') as HTMLInputElement).value = config.api_key || '';
    (document.getElementById('ai-model') as HTMLInputElement).value = config.model || '';
    // 嗅探开关：sniff_enabled 默认 true（后端 serde default 保证）
    const sniffEl = document.getElementById('ai-sniff-enabled');
    if (sniffEl) {
      if (config.sniff_enabled) sniffEl.classList.add('on');
      else sniffEl.classList.remove('on');
    }
  } catch (e) { console.error('加载 AI 配置失败:', e); }

  // 事件只绑定一次
  if (aiConfigLoaded) return;
  aiConfigLoaded = true;

  // 嗅探开关切换（仅切换视觉状态，保存时读取）
  document.getElementById('ai-sniff-enabled')?.addEventListener('click', () => {
    document.getElementById('ai-sniff-enabled')!.classList.toggle('on');
  });

  function showAiStatus(msg: string, type: string) {
    const el = document.getElementById('ai-test-status')!;
    el.className = 'status-card ' + type;
    document.getElementById('ai-test-status-text')!.textContent = msg;
    (el as HTMLElement).style.display = 'flex';
    if (type !== 'loading') setTimeout(() => { (el as HTMLElement).style.display = 'none'; }, 5000);
  }

  document.getElementById('ai-save-btn')?.addEventListener('click', async () => {
    const baseUrl = (document.getElementById('ai-base-url') as HTMLInputElement).value.trim();
    const apiKey = (document.getElementById('ai-api-key') as HTMLInputElement).value.trim();
    const model = (document.getElementById('ai-model') as HTMLInputElement).value.trim();
    const sniffEnabled = document.getElementById('ai-sniff-enabled')!.classList.contains('on');
    try {
      await api.saveAiConfig(baseUrl, apiKey, model, sniffEnabled);
      showToast(t('hub.aiConfigSaved'), 'ok');
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'err');
    }
  });

  document.getElementById('ai-test-btn')?.addEventListener('click', async () => {
    const btn = document.getElementById('ai-test-btn') as HTMLButtonElement;
    const baseUrl = (document.getElementById('ai-base-url') as HTMLInputElement).value.trim();
    const apiKey = (document.getElementById('ai-api-key') as HTMLInputElement).value.trim();
    const model = (document.getElementById('ai-model') as HTMLInputElement).value.trim();
    const sniffEnabled = document.getElementById('ai-sniff-enabled')!.classList.contains('on');
    if (!apiKey) {
      showToast(t('hub.aiNotConfigured'), 'err');
      return;
    }
    btn.textContent = t('hub.testing');
    btn.disabled = true;
    try {
      // 先保存当前表单值，测试连接使用最新配置
      await api.saveAiConfig(baseUrl, apiKey, model, sniffEnabled);
      const result = await api.testAiConnection();
      showAiStatus(t('hub.connectionSuccess') + ': ' + result, 'ok');
      showToast(t('hub.connectionSuccess'), 'ok');
    } catch (e) {
      showAiStatus(t('hub.connectionFailed') + ': ' + e, 'err');
      showToast(t('hub.connectionFailed'), 'err');
    } finally {
      btn.textContent = t('hub.testConnection');
      btn.disabled = false;
    }
  });
}

// ===== AI 报告生成（周报/月报）=====

function formatDateISO(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function getLastWeekRange(): { start: string; end: string } {
  // 上周一到周日（以周一为一周开始）
  const now = new Date();
  const day = now.getDay(); // 0=周日, 1=周一
  const diffToLastMonday = day === 0 ? -6 : 1 - day;
  const monday = new Date(now);
  monday.setDate(now.getDate() + diffToLastMonday - 7);
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  return { start: formatDateISO(monday), end: formatDateISO(sunday) };
}

function getThisMonthRange(): { start: string; end: string } {
  // 本月1号到月末
  const now = new Date();
  const first = new Date(now.getFullYear(), now.getMonth(), 1);
  const last = new Date(now.getFullYear(), now.getMonth() + 1, 0);
  return { start: formatDateISO(first), end: formatDateISO(last) };
}

async function generateReport(periodType: 'weekly' | 'monthly') {
  const btnId = periodType === 'weekly' ? 'btn-generate-weekly-report' : 'btn-generate-monthly-report';
  const btn = document.getElementById(btnId) as HTMLButtonElement;
  if (!btn) return;

  const originalText = btn.textContent;
  btn.disabled = true;
  btn.textContent = t('hub.reportGenerating');

  try {
    const range = periodType === 'weekly' ? getLastWeekRange() : getThisMonthRange();
    const draft = await api.generateReport(periodType, range.start, range.end);

    // 创建新便签并填充内容
    const noteId = await api.createNote();
    await api.updateNoteContent(noteId, draft.content);
    await api.updateNoteTitle(noteId, draft.title);
    await api.openNote(noteId);

    showToast(t('hub.reportGenerated'), 'ok');
    // 刷新便签列表以显示新建的便签
    loadNotes();
  } catch (e) {
    console.error('生成报告失败:', e);
    showToast(t('hub.reportGenerateFailed') + ': ' + e, 'err');
  } finally {
    btn.disabled = false;
    btn.textContent = originalText;
  }
}

document.getElementById('btn-generate-weekly-report')?.addEventListener('click', () => generateReport('weekly'));
document.getElementById('btn-generate-monthly-report')?.addEventListener('click', () => generateReport('monthly'));

// 初始加载
applyLocale();
// 同步窗口标题栏（Tauri 不会自动同步 <title> 标签到标题栏）
getCurrentWindow().setTitle(t('app.settings'));
// 同步语言偏好到后端（托盘菜单等）
invoke('set_locale', { locale: getLocale() });

// 语言切换
document.getElementById('lang-btn')?.addEventListener('click', () => {
  const newLang = getLocale() === 'zh' ? 'en' : 'zh';
  setLocale(newLang);
  invoke('set_locale', { locale: newLang });
  applyLocale();
  getCurrentWindow().setTitle(t('app.settings'));
  const langLabel = document.getElementById('lang-label') as HTMLElement;
  if (langLabel) langLabel.textContent = t('hub.langSwitch');
  const themeLabel = document.getElementById('theme-label') as HTMLElement;
  if (themeLabel) themeLabel.textContent = document.body.classList.contains('dark') ? t('hub.lightMode') : t('hub.darkMode');
  loadNotes();
});

loadNotes().then(() => {
  const overlay = document.getElementById('loading-overlay');
  if (overlay) {
    overlay.style.opacity = '0';
    setTimeout(() => overlay.remove(), 300);
  }
});

// Hub 窗口获得焦点时刷新便签列表（归档/删除等操作后数据可能变化）
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'visible') {
    loadNotes();
  }
});

// ===== 更新检查 =====

let pendingUpdate: Awaited<ReturnType<typeof check>> | null = null;

async function checkForUpdate(silent = false): Promise<void> {
  const statusEl = document.getElementById('update-status');
  const btn = document.getElementById('btn-check-update');
  try {
    if (btn) btn.textContent = '...';
    if (statusEl && !silent) statusEl.textContent = t('hub.updateChecking') || '检查中...';
    const update = await check();
    if (update) {
      pendingUpdate = update;
      if (statusEl) {
        statusEl.textContent = `${t('hub.updateFound') || '发现新版本'} v${update.version}`;
        statusEl.classList.add('has-update');
      }
      showUpdateModal(update);
    } else {
      if (statusEl) {
        statusEl.textContent = t('hub.updateLatest') || '已是最新版本';
        statusEl.classList.remove('has-update');
      }
      if (!silent) {
        // 手动检查时提示
      }
    }
  } catch (e) {
    console.error('检查更新失败:', e);
    if (statusEl && !silent) statusEl.textContent = t('hub.updateCheckFail') || '检查失败';
  } finally {
    if (btn) btn.textContent = t('hub.checkUpdate') || '检查更新';
  }
}

function showUpdateModal(update: NonNullable<Awaited<ReturnType<typeof check>>>) {
  const modal = document.getElementById('update-modal') as HTMLElement;
  const versionEl = document.getElementById('update-modal-version');
  const notesEl = document.getElementById('update-modal-notes');
  if (versionEl) versionEl.textContent = `v${update.version}`;
  if (notesEl) notesEl.textContent = update.body || '';
  modal.style.display = 'flex';
}

function closeUpdateModal() {
  const modal = document.getElementById('update-modal') as HTMLElement;
  modal.style.display = 'none';
}

async function downloadAndInstallUpdate() {
  if (!pendingUpdate) return;
  const downloadBtn = document.getElementById('update-download') as HTMLButtonElement;
  const progressEl = document.getElementById('update-progress') as HTMLElement;
  const progressFill = document.getElementById('progress-fill') as HTMLElement;
  const progressText = document.getElementById('progress-text') as HTMLElement;
  try {
    downloadBtn.disabled = true;
    downloadBtn.textContent = t('hub.updateDownloading') || '下载中...';
    progressEl.style.display = 'block';
    let total = 0;
    let downloaded = 0;
    await pendingUpdate.downloadAndInstall((event: { event: string; data?: { chunkLength?: number; contentLength?: number } }) => {
      switch (event.event) {
        case 'Started':
          total = event.data?.contentLength || 0;
          break;
        case 'Progress':
          downloaded += event.data?.chunkLength || 0;
          if (total > 0) {
            const pct = Math.round((downloaded / total) * 100);
            progressFill.style.width = pct + '%';
            progressText.textContent = `${pct}% (${Math.round(downloaded / 1024 / 1024 * 10) / 10}MB / ${Math.round(total / 1024 / 1024 * 10) / 10}MB)`;
          }
          break;
        case 'Finished':
          progressFill.style.width = '100%';
          progressText.textContent = t('hub.updateInstalling') || '安装中...';
          break;
      }
    });
    // 安装完成，重启应用
    const { restart } = await import('@tauri-apps/plugin-process');
    await restart();
  } catch (e) {
    console.error('下载安装失败:', e);
    downloadBtn.disabled = false;
    downloadBtn.textContent = t('hub.updateDownload') || '下载并安装';
    progressEl.style.display = 'none';
    progressText.textContent = t('hub.updateInstallFail') || '安装失败';
  }
}

document.getElementById('btn-check-update')?.addEventListener('click', () => checkForUpdate(false));
document.getElementById('update-later')?.addEventListener('click', closeUpdateModal);
document.getElementById('update-download')?.addEventListener('click', downloadAndInstallUpdate);

// 启动时延迟 3 秒自动检查更新（静默模式，有更新才弹窗）
setTimeout(() => checkForUpdate(true), 3000);

// Tauri 窗口 focus 时也刷新（并排显示场景）
getCurrentWindow().onFocusChanged(({ payload: focused }) => {
  if (focused) loadNotes();
});

// ===== 批量操作栏事件 =====

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
    // 逐个恢复（暂无 batch_unarchive 命令）
    await Promise.all(ids.map(id => api.unarchiveNote(id)));
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

// 批量改色
batchBar.querySelector('[data-batch-color]')?.addEventListener('click', () => {
  const ids = [...selectedIds];
  if (ids.length === 0) return;
  // 弹出颜色选择
  const overlay = document.createElement('div');
  overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.3);display:flex;align-items:center;justify-content:center;z-index:9999;';
  const panel = document.createElement('div');
  panel.style.cssText = 'background:var(--surface);border-radius:10px;padding:12px;box-shadow:0 8px 28px rgba(0,0,0,0.2);display:flex;gap:8px;flex-wrap:wrap;width:200px;';
  const allColors: Record<string, string> = { amber: '#f59e0b', blue: '#3b82f6', green: '#22c55e', pink: '#ec4899', purple: '#8b5cf6' };
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
});

// 取消多选
batchBar.querySelector('[data-batch-cancel]')?.addEventListener('click', () => {
  selectedIds.clear();
  updateMultiSelectUI();
});

// ===== 模板管理 =====
let tplList: Template[] = [];
let tplSelectedId: string | null = null;

async function loadTemplates() {
  try {
    tplList = await api.getTemplates();
  } catch (e) {
    console.error('加载模板失败:', e);
    tplList = [];
  }
  tplSelectedId = null;
  renderTplList();
  renderTplEditor();
}

function renderTplList() {
  const listEl = document.getElementById('tpl-list')!;
  if (tplList.length === 0) {
    listEl.innerHTML = `<div class="tpl-empty">${t('hub.tplEmpty')}</div>`;
    return;
  }
  listEl.innerHTML = tplList.map(tp =>
    `<div class="tpl-item ${tplSelectedId === tp.id ? 'active' : ''}" data-tpl-id="${escapeHtml(tp.id)}">${escapeHtml(tp.name)}</div>`
  ).join('');
  listEl.querySelectorAll('[data-tpl-id]').forEach(item => {
    item.addEventListener('click', () => {
      tplSelectedId = (item as HTMLElement).dataset.tplId!;
      renderTplList();
      renderTplEditor();
    });
  });
}

function renderTplEditor() {
  const editorEl = document.getElementById('tpl-editor')!;
  const tpl = tplList.find(tp => tp.id === tplSelectedId);
  if (!tpl) {
    editorEl.innerHTML = `<div class="tpl-empty">${t('hub.tplEmpty')}</div>`;
    return;
  }
  editorEl.innerHTML = `
    <input type="text" class="tpl-name-input" id="tpl-name" value="${escapeHtml(tpl.name)}" placeholder="${t('hub.tplName')}" />
    <textarea class="tpl-content-input" id="tpl-content" placeholder="${t('hub.tplContent')}">${escapeHtml(tpl.content)}</textarea>
    <div class="tpl-actions">
      <button class="tpl-action-btn tpl-action-primary" id="tpl-save-btn">${t('hub.tplSave')}</button>
      <button class="tpl-action-btn" id="tpl-create-from-btn">${t('hub.tplCreateFrom')}</button>
      <button class="tpl-action-btn tpl-action-danger" id="tpl-delete-btn">${t('hub.tplDelete')}</button>
    </div>
  `;
  document.getElementById('tpl-save-btn')?.addEventListener('click', async () => {
    const name = (document.getElementById('tpl-name') as HTMLInputElement).value.trim();
    const content = (document.getElementById('tpl-content') as HTMLTextAreaElement).value;
    if (!name) {
      showToast(t('hub.tplNameRequired'), 'err');
      return;
    }
    try {
      const updated: Template = { ...tpl, name, content, updated_at: new Date().toISOString() };
      await api.saveTemplate(updated);
      const idx = tplList.findIndex(tp => tp.id === tpl.id);
      if (idx >= 0) tplList[idx] = updated;
      renderTplList();
      showToast(t('hub.tplSaved'), 'ok');
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'err');
    }
  });
  document.getElementById('tpl-create-from-btn')?.addEventListener('click', async () => {
    try {
      await api.createNoteFromTemplate(tpl.id);
      closeTplDialog();
      showToast(t('hub.tplCreated'), 'ok');
      loadNotes();
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'err');
    }
  });
  document.getElementById('tpl-delete-btn')?.addEventListener('click', async () => {
    if (!confirm(t('hub.tplDeleteConfirm'))) return;
    try {
      await api.deleteTemplate(tpl.id);
      tplList = tplList.filter(tp => tp.id !== tpl.id);
      tplSelectedId = null;
      renderTplList();
      renderTplEditor();
      showToast(t('hub.tplDeleted'), 'ok');
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'err');
    }
  });
}

function openTplDialog() {
  const overlay = document.getElementById('tpl-overlay')!;
  overlay.style.display = 'flex';
  loadTemplates();
}

function closeTplDialog() {
  const overlay = document.getElementById('tpl-overlay')!;
  overlay.style.display = 'none';
}

document.getElementById('btn-templates')?.addEventListener('click', openTplDialog);
document.getElementById('tpl-close')?.addEventListener('click', closeTplDialog);
document.getElementById('tpl-overlay')?.addEventListener('click', (e) => {
  if (e.target === e.currentTarget) closeTplDialog();
});

// 新建模板（前端生成 id，后端 INSERT ON CONFLICT 处理）
document.getElementById('tpl-new')?.addEventListener('click', async () => {
  const now = new Date().toISOString();
  const newTpl: Template = {
    id: 'tpl-' + (crypto.randomUUID?.() ?? Date.now().toString(36) + Math.random().toString(36).slice(2)),
    name: t('hub.tplNew').replace('+ ', '') || '新模板',
    content: '',
    category: 'custom',
    sort_order: tplList.length,
    created_at: now,
    updated_at: now,
  };
  try {
    await api.saveTemplate(newTpl);
    tplList.push(newTpl);
    tplSelectedId = newTpl.id;
    renderTplList();
    renderTplEditor();
    showToast(t('hub.tplNewCreated'), 'ok');
  } catch (e) {
    showToast(t('hub.saveFailed') + ': ' + e, 'err');
  }
});
