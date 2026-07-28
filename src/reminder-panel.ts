/**
 * 提醒面板：在便签窗口右侧展示提醒设置浮层。
 *
 * 职责边界：
 * - showReminderPanel 浮层 UI（快捷时间 + 自定义时间 + 重复类型 + 已有提醒列表）
 * - loadReminders 加载并折叠展示已有 pending 提醒（点击 × 删除）
 * - updateReminderBtnState 同步底部提醒按钮橙色状态
 *
 * 被调用方：note-events.ts (data-reminder 按钮点击)
 * 依赖：api.ts (createReminder/getReminders/deleteReminder) + datetime-picker + datetime.ts (repeatLabel) + i18n
 */

import type { Note } from './types';
import { t, getLocaleTag } from './i18n';
import { repeatLabel } from './datetime';
import * as api from './api';
import { DateTimeSegmentPicker } from './datetime-picker';
import { setupQuickTimeButtons, setupRepeatButtons } from './reminder-form';

export function showReminderPanel(note: Note, app: HTMLElement): void {
  if (app.querySelector('.reminder-overlay')) return;

  const overlay = document.createElement('div');
  overlay.className = 'reminder-overlay';

  // 默认提醒时间：当前时间
  const defaultTime = new Date();

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
	      <div class="rd-datetime-row">
	        <div class="rd-datetime" data-remind-at-dts tabindex="0"></div>
	      </div>
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
  const closeOverlay = () => {
    dts.destroy();
    overlay.remove();
  };
  overlay.querySelector('[data-reminder-close]')!.addEventListener('click', closeOverlay);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) closeOverlay();
  });

  // 初始化日期时间分段选择器：单输入框 yyyy-MM-dd HH:mm
  // 点击段高亮，滚轮/上下箭头调值，左右箭头切段，数字键直接输入
  const dtsContainer = overlay.querySelector('[data-remind-at-dts]') as HTMLElement;
  const dts = new DateTimeSegmentPicker(dtsContainer, {
    initialValue: new Date(defaultTime),
  });

  // 快捷时间按钮（委托 reminder-form 共享逻辑）
  setupQuickTimeButtons(overlay, (date) => dts.setValue(date));

  // 重复选择（委托 reminder-form 共享逻辑）
  const getRepeat = setupRepeatButtons(overlay);

  // AI 自然语言解析已移至便签保存后的自动嗅探气泡，此处仅保留手动表单
  const reminderTitle = note.title || t('app.note');

  // 保存
  overlay.querySelector('[data-save-reminder]')!.addEventListener('click', async () => {
    const dt = dts.getValue();
    if (isNaN(dt.getTime())) return;
    // 界面只到分钟，显式把秒和毫秒设为 0，与界面精度对齐
    dt.setSeconds(0, 0);
    const remindAt = dt.toISOString();
    try {
      await api.createReminder(note.id, reminderTitle, remindAt, getRepeat());
      // 新建提醒成功，更新提醒按钮状态为橙色
      const btn = app.querySelector('.reminder-btn');
      if (btn) btn.classList.add('has-reminder');
      dts.destroy();
      overlay.remove();
    } catch (e) {
      console.error('创建提醒失败:', e);
    }
  });
}

/** 更新提醒按钮状态：有活跃提醒则橙色，否则默认灰色 */
export function updateReminderBtnState(noteId: string): void {
  api.getReminders(noteId)
    .then(reminders => {
      const btn = document.querySelector('.reminder-btn');
      if (!btn) return;
      const hasActive = reminders.some(r => r.status === 'pending');
      btn.classList.toggle('has-reminder', hasActive);
    })
    .catch(() => {});
}

async function loadReminders(noteId: string, container: HTMLElement): Promise<void> {
  try {
    const reminders = await api.getReminders(noteId);
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
          await api.deleteReminder(id);
          loadReminders(noteId, container);
          // 删除后检查是否还有活跃提醒，更新按钮状态
          updateReminderBtnState(noteId);
        } catch (err) {
          console.error('删除提醒失败:', err);
        }
      });
    });
  } catch (e) {
    console.error('加载提醒失败:', e);
  }
}
