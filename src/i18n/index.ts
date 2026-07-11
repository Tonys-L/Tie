import zh from './zh';
import en from './en';

const locales: Record<string, any> = { zh, en };
let current = 'zh';

export function getLocale(): string { return current; }

export function setLocale(lang: string): void {
  current = lang;
  localStorage.setItem('locale', lang);
}

export function initLocale(): void {
  const saved = localStorage.getItem('locale');
  if (saved && locales[saved]) { current = saved; return; }
  current = navigator.language.toLowerCase().startsWith('zh') ? 'zh' : 'en';
}

export function t(key: string, params?: Record<string, string | number>): string {
  const keys = key.split('.');
  let val: any = locales[current];
  for (const k of keys) { val = val?.[k]; }
  if (typeof val !== 'string') val = key;
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      val = val.replace(`{${k}}`, String(v));
    });
  }
  return val;
}

export function getLocaleTag(): string {
  return current === 'zh' ? 'zh-CN' : 'en-US';
}

export function applyLocale(): void {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.getAttribute('data-i18n')!);
  });
  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    (el as HTMLInputElement).placeholder = t(el.getAttribute('data-i18n-placeholder')!);
  });
  document.querySelectorAll('[data-i18n-title]').forEach(el => {
    (el as HTMLElement).title = t(el.getAttribute('data-i18n-title')!);
  });
}
