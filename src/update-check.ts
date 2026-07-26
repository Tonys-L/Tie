/**
 * 更新检查：启动时静默检查 + 手动检查 + 下载安装进度。
 *
 * 职责边界：
 * - checkForUpdate(silent) 检查更新，silent=true 时只在有更新时弹窗
 * - showUpdateModal / closeUpdateModal 更新弹窗
 * - downloadAndInstallUpdate 下载并安装，显示进度条
 * - 启动时延迟 3 秒自动检查
 *
 * 被调用方：hub.ts (顶层绑定到按钮 + setTimeout)
 * 依赖：@tauri-apps/plugin-updater (check) + @tauri-apps/plugin-process (relaunch) + i18n
 */

import { check } from '@tauri-apps/plugin-updater';
import { t } from './i18n';

let pendingUpdate: Awaited<ReturnType<typeof check>> | null = null;

export async function checkForUpdate(silent = false): Promise<void> {
  const statusEl = document.getElementById('update-status');
  const btn = document.getElementById('btn-check-update');
  try {
    if (btn) btn.textContent = '...';
    if (statusEl && !silent) statusEl.textContent = t('hub.updateChecking') || '检查中...';
    const update = await check();
    if (update) {
      pendingUpdate = update;
      if (statusEl) {
        statusEl.textContent = `${t('hub.updateFound') || '发现新版本'} v${update.version}`;
        statusEl.classList.add('has-update');
      }
      showUpdateModal(update);
    } else {
      if (statusEl) {
        statusEl.textContent = t('hub.updateLatest') || '已是最新版本';
        statusEl.classList.remove('has-update');
      }
    }
  } catch (e) {
    console.error('检查更新失败:', e);
    if (statusEl && !silent) statusEl.textContent = t('hub.updateCheckFail') || '检查失败';
  } finally {
    if (btn) btn.textContent = t('hub.checkUpdate') || '检查更新';
  }
}

function showUpdateModal(update: NonNullable<Awaited<ReturnType<typeof check>>>): void {
  const modal = document.getElementById('update-modal') as HTMLElement;
  const versionEl = document.getElementById('update-modal-version');
  const notesEl = document.getElementById('update-modal-notes');
  if (versionEl) versionEl.textContent = `v${update.version}`;
  if (notesEl) notesEl.textContent = update.body || '';
  modal.style.display = 'flex';
}

export function closeUpdateModal(): void {
  const modal = document.getElementById('update-modal') as HTMLElement;
  modal.style.display = 'none';
}

async function downloadAndInstallUpdate(): Promise<void> {
  if (!pendingUpdate) return;
  const downloadBtn = document.getElementById('update-download') as HTMLButtonElement;
  const progressEl = document.getElementById('update-progress') as HTMLElement;
  const progressFill = document.getElementById('progress-fill') as HTMLElement;
  const progressText = document.getElementById('progress-text') as HTMLElement;
  try {
    downloadBtn.disabled = true;
    downloadBtn.textContent = t('hub.updateDownloading') || '下载中...';
    progressEl.style.display = 'block';
    let total = 0;
    let downloaded = 0;
    await pendingUpdate.downloadAndInstall((event: { event: string; data?: { chunkLength?: number; contentLength?: number } }) => {
      switch (event.event) {
        case 'Started':
          total = event.data?.contentLength || 0;
          break;
        case 'Progress':
          downloaded += event.data?.chunkLength || 0;
          if (total > 0) {
            const pct = Math.round((downloaded / total) * 100);
            progressFill.style.width = pct + '%';
            progressText.textContent = `${pct}% (${Math.round(downloaded / 1024 / 1024 * 10) / 10}MB / ${Math.round(total / 1024 / 1024 * 10) / 10}MB)`;
          }
          break;
        case 'Finished':
          progressFill.style.width = '100%';
          progressText.textContent = t('hub.updateInstalling') || '安装中...';
          break;
      }
    });
    // 安装完成，重启应用
    const { relaunch } = await import('@tauri-apps/plugin-process');
    await relaunch();
  } catch (e) {
    console.error('下载安装失败:', e);
    downloadBtn.disabled = false;
    downloadBtn.textContent = t('hub.updateDownload') || '下载并安装';
    progressEl.style.display = 'none';
    progressText.textContent = t('hub.updateInstallFail') || '安装失败';
  }
}

// 按钮事件绑定
document.getElementById('btn-check-update')?.addEventListener('click', () => checkForUpdate(false));
document.getElementById('update-later')?.addEventListener('click', closeUpdateModal);
document.getElementById('update-download')?.addEventListener('click', downloadAndInstallUpdate);

// 启动时延迟 3 秒自动检查更新（静默模式，有更新才弹窗）
setTimeout(() => checkForUpdate(true), 3000);
