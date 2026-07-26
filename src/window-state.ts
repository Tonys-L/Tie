/**
 * 窗口状态持久化：监听窗口移动/缩放，防抖保存到后端。
 *
 * 职责边界：
 * - 监听 Tauri 窗口 onMoved/onResized 事件
 * - 防抖 500ms + 最小尺寸校验（宽>=200 高>=150）
 * - 关闭标志位由外部调用 setClosing(true) 设置（关闭窗口时避免保存极小尺寸）
 *
 * 被调用方：main.ts (initNoteWindow)
 * 依赖：@tauri-apps/api/window (getCurrentWindow) + api.ts (updateNoteWindowState)
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import * as api from './api';

// 关闭标志：窗口关闭过程中不再保存状态，避免保存极小尺寸
let isClosing = false;

export function setClosing(value: boolean): void {
  isClosing = value;
}

export function setupWindowEvents(id: string): void {
  const win = getCurrentWindow();
  let saveTimeout: ReturnType<typeof setTimeout> | undefined;

  const saveWindowState = () => {
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(async () => {
      if (isClosing) return; // 关闭中不保存，避免写入极小尺寸
      try {
        const pos = await win.outerPosition();
        const size = await win.outerSize();
        // 最小尺寸校验：宽>=200 高>=150，防止保存异常值
        if (size.width < 200 || size.height < 150) return;
        await api.updateNoteWindowState(id, pos.x, pos.y, size.width, size.height);
      } catch (e) {
        console.error('保存窗口状态失败:', e);
      }
    }, 500);
  };

  win.onMoved(() => saveWindowState());
  win.onResized(() => saveWindowState());
}
