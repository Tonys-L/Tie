/**
 * 日期时间工具 + i18n 标签。
 *
 * 职责：
 * - localISO：Date 转 datetime-local 输入格式 (yyyy-MM-ddTHH:mm)
 * - formatDate：格式化日期为 yyyy/MM/dd HH:mm
 * - formatNoteTime：格式化便签创建时间为简短显示（如 "7/17 10:30"）
 * - quickDate：快捷时间计算（1h/3h/tomorrow/week）
 * - repeatLabel：重复类型标签（基于 i18n）
 *
 * 被调用方：
 * - reminder-dialog.ts (localISO / quickDate / repeatLabel)
 * - notes-list.ts (formatDate)
 * - calendar-view.ts (repeatLabel)
 * - reminder-panel.ts (repeatLabel)
 * - note-renderer.ts (formatNoteTime)
 *
 * 依赖：i18n (t, getLocaleTag)
 *
 * 设计目的：从原 utils.ts 拆出，让日期时间相关函数有独立 seam。
 * formatNoteTime 从 colors.ts 迁入（职责错位修正：时间格式化属于 datetime 而非 colors）。
 */

import { t, getLocaleTag } from './i18n';

/** Date 转本地 datetime-local 输入格式 (yyyy-MM-ddTHH:mm) */
export function localISO(d: Date): string {
  const off = d.getTimezoneOffset();
  return new Date(d.getTime() - off * 60000).toISOString().slice(0, 16);
}

/** 格式化日期为 yyyy/MM/dd HH:mm */
export function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')} ${d.toLocaleTimeString(getLocaleTag(), { hour: '2-digit', minute: '2-digit' })}`;
}

/** 格式化便签创建时间为简短显示（如 "7/17 10:30"） */
export function formatNoteTime(iso: string): string {
  const d = new Date(iso);
  const locale = localStorage.getItem('locale') || 'zh';
  const month = d.getMonth() + 1;
  const day = d.getDate();
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  return locale === 'zh' ? `${month}/${day} ${hh}:${mm}` : `${month}/${day} ${hh}:${mm}`;
}

/** 快捷时间计算：返回目标 Date */
export function quickDate(type: string): Date {
  const now = new Date();
  if (type === '1h') {
    now.setHours(now.getHours() + 1);
  } else if (type === '3h') {
    now.setHours(now.getHours() + 3);
  } else if (type === 'tomorrow') {
    now.setDate(now.getDate() + 1);
    now.setHours(9, 0, 0, 0);
  } else if (type === 'week') {
    const day = now.getDay();
    const days = day === 0 ? 1 : 8 - day;
    now.setDate(now.getDate() + days);
    now.setHours(9, 0, 0, 0);
  }
  return now;
}

/** 重复类型标签（基于 i18n） */
export function repeatLabel(type: string): string {
  const map: Record<string, string> = {
    none: '', once: '',
    daily: t('note.daily'), weekly: t('note.weekly'), monthly: t('note.monthly'),
    lunar_monthly: t('note.lunarMonthly'),
  };
  return map[type] || type;
}
