/**
 * Toast 提示组件（底部居中轻量通知）。
 *
 * 职责：
 * - showToast(message, type, persistent)：新提示自动替换已有提示
 * - type：info（灰）/success（绿）/error（红）
 * - persistent=true 时不自动消失（用于 loading 状态，由后续 toast 替换）
 *
 * 被调用方：
 * - main.ts / hub.ts（入口）
 * - ai-todo-sort.ts / ai-settings.ts / ai-sniff.ts / ai-rewrite.ts
 * - context-menu.ts / template-manager.ts / template-ui.ts
 *
 * 依赖：无
 *
 * 设计目的：从原 utils.ts 拆出，让 toast UI 有独立 seam，避免每次引入 toast 都拉入整个 utils。
 * main.ts 与 hub.ts 共用此实现，避免 UX 不一致。
 */

/** 轻量 toast 提示（底部居中），新提示自动替换已有提示 */
export function showToast(
  message: string,
  type: 'info' | 'success' | 'error' = 'info',
  persistent: boolean = false,
): void {
  const existing = document.querySelector('.app-toast');
  if (existing) existing.remove();
  const toast = document.createElement('div');
  toast.className = 'app-toast';
  toast.textContent = message;
  const bg = type === 'error' ? '#dc2626' : type === 'success' ? '#16a34a' : '#475569';
  toast.style.cssText = `position:fixed;bottom:24px;left:50%;transform:translateX(-50%);padding:10px 20px;border-radius:8px;background:${bg};color:#fff;font-size:13px;font-weight:500;z-index:100000;box-shadow:0 4px 16px rgba(0,0,0,0.2);font-family:inherit;max-width:80vw;`;
  document.body.appendChild(toast);
  if (!persistent) {
    setTimeout(() => {
      toast.style.transition = 'opacity 0.3s';
      toast.style.opacity = '0';
      setTimeout(() => toast.remove(), 300);
    }, 2500);
  }
}
