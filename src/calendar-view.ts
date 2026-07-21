/**
 * 日历视图（Hub 页面）：月视图/年视图 + 当日提醒 + 当日便签 + 农历。
 *
 * 职责边界：
 * - loadCalendar 懒加载绑定翻页/视图切换按钮，渲染当前月/年
 * - renderMonthView 月视图：并行加载提醒/农历/便签活动，渲染日格 + 提醒预览 + 便签点
 * - renderYearView 年视图：并行加载全年提醒，渲染 12 个月缩略
 * - showDayDetail 当日详情：提醒列表 + 当天更新便签列表（点击激活便签）
 *
 * 被调用方：hub.ts (页面切换时调用)
 * 依赖：@tauri-apps/api/core (invoke) + api.ts + utils.ts (escapeHtml/repeatLabel) +
 *       i18n (t/getLocale/getLocaleTag) + types (Reminder) +
 *       notes-list.ts (getActiveNotes/getArchivedNotes 读取便签用于 showDayDetail)
 */

import { invoke } from '@tauri-apps/api/core';
import type { Reminder } from './types';
import * as api from './api';
import { escapeHtml, repeatLabel } from './utils';
import { t, getLocale, getLocaleTag } from './i18n';
import { getActiveNotes, getArchivedNotes } from './notes-list';

// ===== 模块级 state =====
let calLoaded = false;
let calYear = new Date().getFullYear();
let calMonth = new Date().getMonth() + 1;
let calReminders: Reminder[] = [];
let calSelectedDate: string | null = null;
let calView: 'month' | 'year' = 'month';
let calLunarMap = new Map<number, string>();
let calNoteActivityDays = new Set<number>();

const monthNamesEn = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];

/** 加载日历视图（首次进入时绑定按钮，后续仅渲染） */
export async function loadCalendar(): Promise<void> {
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
  }
  await renderCalendar();
}

async function renderCalendar(): Promise<void> {
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

async function renderMonthView(): Promise<void> {
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

async function renderYearView(): Promise<void> {
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

/** 当日详情：提醒列表 + 当天更新便签列表 */
function showDayDetail(day: number): void {
  const detailEl = document.getElementById('cal-detail');
  if (!detailEl) return;
  const dayReminders = calReminders.filter(r => {
    const d = new Date(r.remind_at);
    return d.getDate() === day && d.getMonth() + 1 === calMonth && d.getFullYear() === calYear;
  });
  // 过滤当天更新的便签（按 updated_at 本地日期匹配）
  const dayNotes = [...getActiveNotes(), ...getArchivedNotes()].filter(n => {
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
  detailEl.classList.add('show');
}
