/**
 * Hub 窗口入口（设置中心）：页面切换编排 + 全局事件 + 初始加载。
 *
 * 职责边界：
 * - 通知点击 → 激活便签
 * - 主题切换 + 关于页版本号
 * - 页面切换编排（notes/calendar/general/sync/ai/shortcuts）
 * - 初始加载 + 语言切换 + visibilitychange/focus 刷新便签列表
 *
 * 不负责：具体业务实现（已按页面/组件拆分到独立模块，见下方 imports）
 *
 * 被调用方：HTML 入口（hub.html）加载本模块
 * 依赖：notes-list (loadNotes) + calendar-view (loadCalendar) + ai-settings (loadAiConfig) +
 *       general-settings + shortcut-settings + sync-settings + template-manager (side effect) +
 *       update-check (side effect) + i18n + @tauri-apps/api (window/core/event)
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen } from '@tauri-apps/api/event';

// ===== 共享模块 =====
import { initLocale, t, applyLocale, getLocale, setLocale } from './i18n';
import * as api from './api';

// ===== Hub 页面模块（按设置页 tab 分组）=====
import { loadNotes } from './notes-list';              // 便签列表页
import { loadCalendar } from './calendar-view';        // 日历视图页
import { loadGeneralSettings } from './general-settings'; // 通用设置页
import { loadSyncConfig } from './sync-settings';      // 同步设置页
import { loadAiConfig } from './ai-settings';          // AI 设置页
import { loadShortcutConfig } from './shortcut-settings'; // 快捷键设置页

// ===== side-effect 模块（顶层绑定按钮，无 named export）=====
import './template-manager';   // 模板管理弹窗（按钮绑定在模块加载时执行）
import './update-check';       // 更新检查（按钮绑定在模块加载时执行）

initLocale();

// ===== 通知点击 → 激活对应便签 =====
listen('tauri://notification', (event: any) => {
  const noteId = event?.payload?.data?.note_id || event?.payload?.note_id;
  if (noteId) {
    api.activateNoteById(noteId).catch(err => console.error('激活便签失败:', err));
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

// ===== 关于页动态版本号 =====
(async () => {
  try {
    const { getVersion } = await import('@tauri-apps/api/app');
    const v = await getVersion();
    const el = document.getElementById('about-version');
    if (el) el.textContent = `v${v}`;
  } catch {}
})();

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

// ===== 初始加载 =====
applyLocale();
// 同步窗口标题栏（Tauri 不会自动同步 <title> 标签到标题栏）
getCurrentWindow().setTitle(t('app.settings'));
// 同步语言偏好到后端（托盘菜单等）
api.setLocale(getLocale());

// 语言切换
document.getElementById('lang-btn')?.addEventListener('click', () => {
  const newLang = getLocale() === 'zh' ? 'en' : 'zh';
  setLocale(newLang);
  api.setLocale(newLang);
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

// Tauri 窗口 focus 时也刷新（并排显示场景）
getCurrentWindow().onFocusChanged(({ payload: focused }) => {
  if (focused) loadNotes();
});
