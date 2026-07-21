/**
 * 提醒设置弹窗（Hub 内嵌）：在 Hub 页面内为便签创建/删除提醒。
 *
 * 职责边界：
 * - showReminderDialog 显示弹窗（快捷时间 + 自定义时间 + 重复类型 + 已有提醒列表）
 * - loadExistingReminders 折叠展示已有 pending 提醒（点击 × 删除）
 * - 不打开便签窗口，纯 Hub 页面内操作
 *
 * 被调用方：hub.ts (renderList 中的"提醒"按钮)
 * 依赖：api.ts (createReminder/getReminders/deleteReminder) + utils.ts (escapeHtml/localISO/quickDate/repeatLabel) +
 *       i18n (t/getLocaleTag) + types (Reminder) + 外部传入 onNotesChanged 回调
 */

import type { Reminder } from './types';
import { t, getLocaleTag } from './i18n';
import { escapeHtml, localISO, quickDate, repeatLabel } from './utils';
import * as api from './api';

/**
 * 显示提醒设置弹窗。
 * @param onNotesChanged 提醒创建/删除后调用（用于刷新便签列表中的提醒图标）
 */
export function showReminderDialog(noteId: string, noteTitle: string, onNotesChanged: () => void): void {
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
  loadExistingReminders(noteId, onNotesChanged);

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
      onNotesChanged();
    } catch (e) { console.error('创建提醒失败:', e); }
  });
}

async function loadExistingReminders(noteId: string, onNotesChanged: () => void): Promise<void> {
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
          loadExistingReminders(noteId, onNotesChanged);
          onNotesChanged();
        } catch (e) { console.error('删除提醒失败:', e); }
      });
    });
  } catch (e) { console.error('加载提醒失败:', e); }
}
