import { open } from '@tauri-apps/plugin-shell';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import type { Reminder, ShortcutConfig } from './types';
import * as api from './api';
import { COLOR_MAP, escapeHtml, formatDate, localISO, quickDate, repeatLabel } from './utils';
import { initLocale, t, applyLocale, getLocale, setLocale } from './i18n';

initLocale();

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

// ===== 页面切换 =====
document.querySelectorAll('.nav-item').forEach(item => {
  item.addEventListener('click', () => {
    document.querySelectorAll('.nav-item').forEach(n => n.classList.remove('active'));
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    item.classList.add('active');
    const page = document.getElementById('page-' + item.getAttribute('data-page'));
    if (page) page.classList.add('active');
    if (item.getAttribute('data-page') === 'notes') loadNotes();
    if (item.getAttribute('data-page') === 'sync') loadSyncConfig();
    if (item.getAttribute('data-page') === 'shortcuts') loadShortcutConfig();
  });
});

// ===== 便签管理 =====
let currentTab = 'active';
let activeNotes: any[] = [];
let archivedNotes: any[] = [];
let searchQuery = '';
const listEl = document.getElementById('list')!;
const searchInput = document.getElementById('search') as HTMLInputElement;

document.querySelectorAll('.mgr-tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.mgr-tab').forEach(t => t.classList.remove('active'));
    tab.classList.add('active');
    currentTab = tab.getAttribute('data-tab') || 'active';
    if (searchInput) { searchInput.value = ''; searchQuery = ''; }
    renderList();
  });
});

searchInput?.addEventListener('input', () => {
  searchQuery = searchInput.value.toLowerCase().trim();
  renderList();
});

async function loadNotes() {
  try {
    const [active, archived] = await Promise.all([api.getAllNotes(), api.getArchivedNotes()]);
    activeNotes = active as any[];
    archivedNotes = archived as any[];
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
    renderList();
  } catch(e) { console.error('加载失败:', e); }
}

function renderList() {
  let notes: any[];
  let isSearchMode = false;
  if (searchQuery) {
    isSearchMode = true;
    notes = [...activeNotes, ...archivedNotes].filter(n =>
      (n.title || '').toLowerCase().includes(searchQuery) ||
      (n.content || '').toLowerCase().includes(searchQuery)
    );
  } else if (currentTab === 'reminders') {
    notes = [...activeNotes, ...archivedNotes].filter(n => (n._reminderCount || 0) > 0);
  } else {
    notes = currentTab === 'active' ? activeNotes : archivedNotes;
  }
  if (notes.length === 0) {
    const emptyText = searchQuery ? t('hub.noMatch')
      : currentTab === 'reminders' ? t('hub.noReminders')
      : currentTab === 'active' ? t('hub.noActive') : t('hub.noArchived');
    listEl.innerHTML = `<div class="empty-state"><svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg><span>${emptyText}</span></div>`;
    return;
  }
  const sorted = [...notes].sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
  listEl.innerHTML = sorted.map(n => {
    const color = COLOR_MAP[n.color] || COLOR_MAP.amber;
    const title = n.title || t('hub.noTitle');
    const preview = (n.content || '').replace(/[#*`>\-\[\]]/g, '').slice(0, 60) || t('hub.noContent');
    const isArchived = archivedNotes.some(a => a.id === n.id);
    const showTag = isSearchMode || currentTab === 'reminders';
    const tag = showTag ? (isArchived ? `<span class="note-tag archived">${t('hub.archived')}</span>` : `<span class="note-tag active">${t('hub.activeNotes')}</span>`) : '';
    const dateStr = formatDate(n.updated_at);
    const actionBtn = isArchived
	      ? `<button class="act-btn restore" data-restore="${n.id}" title="${t('hub.restore')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg></button>`
	      : `<button class="act-btn reminder" data-reminder="${n.id}" data-title="${escapeHtml(title)}" title="${t('hub.reminders')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg></button><button class="act-btn archive" data-archive="${n.id}" title="${t('note.archive')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="21 8 21 21 3 21 3 8"/><rect x="1" y="3" width="22" height="5"/><line x1="10" y1="12" x2="14" y2="12"/></svg></button>`;
    const reminderBadge = n._reminderCount > 0 ? `<span class="reminder-badge">${n._reminderCount}</span>` : '';
    return `<div class="note-item" data-id="${n.id}"><div class="note-color" style="background:${color}"></div><div class="note-text"><div class="note-title">${escapeHtml(title)} ${tag}</div><div class="note-preview">${escapeHtml(preview)}</div></div>${reminderBadge}<span class="note-date">${dateStr}</span><div class="note-actions">${actionBtn}<button class="act-btn delete" data-delete="${n.id}" title="${t('note.delete')}"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg></button></div></div>`;
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
    api.openNote(noteItem.dataset.id!);
  }
});

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

  // 保存
  dialog.querySelector('#rm-save')!.addEventListener('click', async () => {
    const input = dialog.querySelector('#rm-datetime') as HTMLInputElement;
    const dt = new Date(input.value);
    if (isNaN(dt.getTime())) return;
    try {
      await api.createReminder(noteId, noteTitle, dt.toISOString(), selectedRepeat);
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
      const dt = new Date(r.remind_at).toLocaleString('zh-CN', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
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
  if (gitInstalled) { gitEl.className = 'status-card ok'; document.getElementById('git-status-text')!.textContent = t('hub.gitInstalled'); }
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
    catch (e) { showSyncStatus('保存失败: ' + e, 'err'); }
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
      showSyncStatus(result, 'ok');
    } catch (e) {
      console.error('[同步] 失败:', e);
      showSyncStatus('同步失败: ' + e, 'err');
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
  } catch (e) { console.error('加载快捷键配置失败:', e); }

  function getShortcutConfig(): ShortcutConfig {
    return {
      new_note: (document.getElementById('shortcut-new-note') as HTMLInputElement).value.trim().toLowerCase(),
      show_all: (document.getElementById('shortcut-show-all') as HTMLInputElement).value.trim().toLowerCase(),
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
    if (!config.new_note || !config.show_all) {
      showShortcutStatus(t('hub.shortcutEmpty'), 'err');
      return;
    }
    try {
      await api.saveShortcutConfig(config);
      showShortcutStatus(t('hub.shortcutSaved'), 'ok');
    } catch (e) {
      showShortcutStatus('保存失败: ' + e, 'err');
    }
  });

  document.getElementById('shortcut-reset-btn')?.addEventListener('click', () => {
    (document.getElementById('shortcut-new-note') as HTMLInputElement).value = 'ctrl+shift+n';
    (document.getElementById('shortcut-show-all') as HTMLInputElement).value = 'ctrl+shift+s';
  });
}

// 初始加载
applyLocale();
// 同步语言偏好到后端（托盘菜单等）
invoke('set_locale', { locale: getLocale() });

// 语言切换
document.getElementById('lang-btn')?.addEventListener('click', () => {
  const newLang = getLocale() === 'zh' ? 'en' : 'zh';
  setLocale(newLang);
  invoke('set_locale', { locale: newLang });
  applyLocale();
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

// Tauri 窗口 focus 时也刷新（并排显示场景）
getCurrentWindow().onFocusChanged(({ payload: focused }) => {
  if (focused) loadNotes();
});
