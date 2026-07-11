// ============ 共享工具函数 ============

import { t } from './i18n';

/** HTML 转义 */
export function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

/** Date 转本地 datetime-local 输入格式 (yyyy-MM-ddTHH:mm) */
export function localISO(d: Date): string {
  const off = d.getTimezoneOffset();
  return new Date(d.getTime() - off * 60000).toISOString().slice(0, 16);
}

/** 格式化日期为 yyyy/MM/dd HH:mm */
export function formatDate(iso: string): string {
  const d = new Date(iso);
  return `${d.getFullYear()}/${String(d.getMonth() + 1).padStart(2, '0')}/${String(d.getDate()).padStart(2, '0')} ${d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}`;
}

/** Hub 列表用的颜色映射 */
export const COLOR_MAP: Record<string, string> = {
  amber: '#fde047', blue: '#93c5fd', pink: '#f9a8d4', green: '#6ee7b7', white: '#e5e7eb',
};

/** 便签窗口用的颜色配置（含 rgba 背景生成器） */
export const COLORS: Record<string, { bg: (a: number) => string; dot: string }> = {
  amber: { bg: (a) => `rgba(254, 249, 195, ${a})`, dot: '#fde047' },
  blue:  { bg: (a) => `rgba(219, 234, 254, ${a})`, dot: '#93c5fd' },
  pink:  { bg: (a) => `rgba(252, 231, 243, ${a})`, dot: '#f9a8d4' },
  green: { bg: (a) => `rgba(209, 250, 229, ${a})`, dot: '#6ee7b7' },
  white: { bg: (a) => `rgba(255, 255, 255, ${a})`, dot: '#d1d5db' },
};

/** 重复类型标签（基于 i18n） */
export function repeatLabel(type: string): string {
  const map: Record<string, string> = {
    none: '', once: '',
    daily: t('note.daily'), weekly: t('note.weekly'), monthly: t('note.monthly'),
  };
  return map[type] || type;
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
