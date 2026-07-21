/**
 * 删除确认弹窗：覆盖层 + 取消/确认按钮。
 *
 * 职责边界：
 * - 弹窗 UI + 按钮事件
 * - 调用 api.deleteNote 完成删除
 * - 删除失败时降级关闭窗口（数据可能已被后端删除但窗口残留）
 *
 * 被调用方：context-menu.ts (右键菜单"删除")
 * 依赖：@tauri-apps/api/window (close/destroy 降级) + api.ts (deleteNote) + i18n
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import { t } from './i18n';
import * as api from './api';

export function showDeleteConfirm(noteId: string, app: HTMLElement): void {
  // 已存在则跳过
  if (app.querySelector('.delete-overlay')) return;

  const overlay = document.createElement('div');
  overlay.className = 'delete-overlay';
  overlay.innerHTML = `
	    <div class="delete-dialog">
	      <p>${t('note.deleteConfirm')}</p>
	      <div class="delete-actions">
	        <button class="btn-cancel">${t('note.cancel')}</button>
	        <button class="btn-confirm">${t('note.deleteBtn')}</button>
	      </div>
	    </div>
	  `;
  app.appendChild(overlay);

  const win = getCurrentWindow();
  overlay.querySelector('.btn-cancel')!.addEventListener('click', () => overlay.remove());
  overlay.querySelector('.btn-confirm')!.addEventListener('click', async () => {
    try {
      await api.deleteNote(noteId);
      // 后端已通过 destroy 关闭窗口，无需前端再关
    } catch (e) {
      console.error('删除便签失败:', e);
      // 删除失败时也尝试关闭窗口（数据可能已被删除但窗口仍存在）
      try { await win.close(); } catch (_) { win.destroy(); }
    }
  });
}
