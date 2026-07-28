/**
 * 通用设置页：开机自启 + 数据目录显示/打开。
 *
 * 职责边界：
 * - loadGeneralSettings 加载一次（懒加载防重复绑定），自启开关 + 数据目录
 *
 * 被调用方：hub.ts (页面切换时调用)
 * 依赖：@tauri-apps/plugin-autostart + api.ts (getDataDir/openDataDir) + i18n
 */

import { enable as enableAutoStart, disable as disableAutoStart, isEnabled as isAutoStartEnabled } from '@tauri-apps/plugin-autostart';
import { invoke } from '@tauri-apps/api/core';
import * as api from './api';

let generalSettingsLoaded = false;

export async function loadGeneralSettings(): Promise<void> {
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

  // 置顶免疫显示桌面开关
  try {
    const pinEnabled = await invoke<boolean>('get_pin_desktop');
    if (pinEnabled) document.getElementById('pin-desktop')!.classList.add('on');
  } catch (e) { console.error('获取置顶免疫状态失败:', e); }

  document.getElementById('pin-desktop')?.addEventListener('click', async () => {
    const el = document.getElementById('pin-desktop')!;
    const turningOn = !el.classList.contains('on');
    try {
      const result = await invoke<boolean>('set_pin_desktop', { enabled: turningOn });
      if (result) el.classList.add('on');
      else el.classList.remove('on');
    } catch (e) { console.error('设置置顶免疫失败:', e); }
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
