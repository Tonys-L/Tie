/**
 * 提醒表单共享逻辑（候选2：消除 reminder-panel 与 reminder-dialog 的重复）。
 *
 * 职责：
 * - setupQuickTimeButtons：快捷时间按钮（1h/3h/tomorrow/week）统一委托 quickDate
 * - setupRepeatButtons：重复类型按钮选择器，返回获取当前选择的函数
 *
 * 被调用方：
 * - reminder-panel.ts (便签窗口内嵌面板)
 * - reminder-dialog.ts (Hub 页面弹窗)
 *
 * 依赖：datetime.ts (quickDate)
 *
 * 设计目的：两处 datetime 控件不同（DateTimeSegmentPicker vs 原生 input），
 * 故通过 setValue 回调参数化；重复按钮逻辑完全相同故直接抽取。
 */

import { quickDate } from './datetime';

/**
 * 设置快捷时间按钮点击：点击后通过 setValue 回调写入目标控件。
 * @param container 按钮所在容器
 * @paramsetValue 写入回调（DateTimeSegmentPicker.setValue 或 input.value = localISO）
 */
export function setupQuickTimeButtons(
  container: HTMLElement,
  setValue: (date: Date) => void,
): void {
  container.querySelectorAll('[data-quick]').forEach((btn) => {
    btn.addEventListener('click', () => {
      const type = (btn as HTMLElement).dataset.quick!;
      setValue(quickDate(type));
    });
  });
}

/**
 * 设置重复类型按钮选择器：点击切换 active 态，返回获取当前选择的函数。
 * @param container 按钮所在容器
 * @returns 获取当前选择的 repeat type（'none'/'daily'/'weekly'/'monthly'/'lunar_monthly'）
 */
export function setupRepeatButtons(container: HTMLElement): () => string {
  let selectedRepeat = 'none';
  container.querySelectorAll('[data-repeat]').forEach((btn) => {
    btn.addEventListener('click', () => {
      container.querySelectorAll('[data-repeat]').forEach((b) => b.classList.remove('active'));
      btn.classList.add('active');
      selectedRepeat = (btn as HTMLElement).dataset.repeat!;
    });
  });
  return () => selectedRepeat;
}
