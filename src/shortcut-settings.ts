/**
 * 快捷键设置页：加载/保存/重置快捷键配置。
 *
 * 职责边界：
 * - loadShortcutConfig 加载一次（懒加载防重复绑定），3 个输入框 + 保存/重置按钮
 * - 默认值：ctrl+shift+n / ctrl+shift+s / ctrl+shift+h
 *
 * 被调用方：hub.ts (页面切换时调用)
 * 依赖：api.ts (getShortcutConfig/saveShortcutConfig) + types.ts (ShortcutConfig) + i18n
 */

import type { ShortcutConfig } from './types';
import * as api from './api';
import { t } from './i18n';

let shortcutConfigLoaded = false;

export async function loadShortcutConfig(): Promise<void> {
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

  function showShortcutStatus(msg: string, type: string): void {
    const el = document.getElementById('shortcut-status')!;
    el.className = 'status-card ' + type;
    document.getElementById('shortcut-status-text')!.textContent = msg;
    (el as HTMLElement).style.display = 'flex';
    setTimeout(() => { (el as HTMLElement).style.display = 'none'; }, 3000);
  }

  document.getElementById('shortcut-save-btn')?.addEventListener('click', async () => {
    const config = getShortcutConfig();
    if (!config.new_note || !config.show_all || !config.toggle_hub) {
      showShortcutStatus(t('hub.shortcutEmpty'), 'error');
      return;
    }
    try {
      await api.saveShortcutConfig(config);
      showShortcutStatus(t('hub.shortcutSaved'), 'success');
    } catch (e) {
      showShortcutStatus(t('hub.saveFailed') + ': ' + e, 'error');
    }
  });

  document.getElementById('shortcut-reset-btn')?.addEventListener('click', () => {
    (document.getElementById('shortcut-new-note') as HTMLInputElement).value = 'ctrl+shift+n';
    (document.getElementById('shortcut-show-all') as HTMLInputElement).value = 'ctrl+shift+s';
    (document.getElementById('shortcut-toggle-hub') as HTMLInputElement).value = 'ctrl+shift+h';
  });
}
