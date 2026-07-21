/**
 * 删除确认弹窗：覆盖层 + 取消/确认按钮。
 *
 * 职责边界：
 * - 弹窗 UI + 按钮事件
 * - 调用 api.deleteNote 完成删除
 * - 两种调用模式：
 *   · 便签窗口模式（无 onDeleted）：删除后依赖后端 destroy 关闭窗口；失败时降级 close
 *   · Hub 列表模式（有 onDeleted）：删除成功后调用 onDeleted 刷新列表
 *
 * 被调用方：
 * - context-menu.ts（便签窗口右键菜单"删除"，window 模式）
 * - notes-list.ts（Hub 列表删除按钮，hub 模式，onDeleted=loadNotes）
 *
 * 依赖：@tauri-apps/api/window (close/destroy 降级) + api.ts (deleteNote) + i18n
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import { t } from './i18n';
import * as api from './api';

/**
 * 显示删除确认弹窗。
 *
 * @param noteId 待删除便签 ID
 * @param target 弹窗 append 目标（便签窗口传 app 元素，Hub 传 document.body）
 * @param onDeleted 删除成功回调（Hub 模式必填，传入 loadNotes 刷新列表；
 *                  便签窗口模式不传，依赖后端 destroy 关闭窗口）
 */
export function showDeleteConfirm(
  noteId: string,
  target: HTMLElement,
  onDeleted?: () => void,
): void {
  // 已存在则跳过
  if (target.querySelector('.delete-overlay')) return;

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
  target.appendChild(overlay);

  const win = getCurrentWindow();
  overlay.querySelector('.btn-cancel')!.addEventListener('click', () => overlay.remove());
  overlay.querySelector('.btn-confirm')!.addEventListener('click', async () => {
    try {
      await api.deleteNote(noteId);
      // Hub 模式：前端控制刷新；便签窗口模式：依赖后端 destroy 关闭窗口
      if (onDeleted) onDeleted();
    } catch (e) {
      console.error('删除便签失败:', e);
      // 仅便签窗口模式需要降级关闭窗口（数据可能已被删除但窗口仍存在）
      // Hub 模式由调用方决定如何处理失败（通常下次 loadNotes 会刷新）
      if (!onDeleted) {
        try { await win.close(); } catch (_) { win.destroy(); }
      }
    }
  });
}
