/**
 * 同步设置页：Git 仓库配置 + 同步操作 + 分支创建对话框。
 *
 * 职责边界：
 * - loadSyncConfig 加载一次（懒加载防重复绑定），表单 + 状态卡 + 同步按钮
 * - showSyncStatus 显示状态卡片（success/error/loading，5 秒自动消失）
 * - showBranchCreateDialog 远程分支不存在时提示用户是否创建
 *
 * 被调用方：hub.ts (页面切换时调用)
 * 依赖：@tauri-apps/plugin-shell (open) + api.ts (checkGit/getSyncConfig/saveSyncConfig/syncNotes) + i18n
 */

import { open } from '@tauri-apps/plugin-shell';
import * as api from './api';
import { t } from './i18n';

let syncConfigLoaded = false;

export async function loadSyncConfig(): Promise<void> {
  if (syncConfigLoaded) return;
  syncConfigLoaded = true;

  document.getElementById('gitee-link')?.addEventListener('click', (e) => { e.preventDefault(); open('https://gitee.com/profile/personal_access_tokens'); });
  document.getElementById('github-link')?.addEventListener('click', (e) => { e.preventDefault(); open('https://github.com/settings/tokens'); });

  const gitInstalled = await api.checkGit();
  const gitEl = document.getElementById('git-status')!;
  if (gitInstalled) {
    gitEl.className = 'status-card ok';
    try {
      const config = await api.getSyncConfig();
      const branch = config.branch || 'main';
      document.getElementById('git-status-text')!.textContent = `${t('hub.gitInstalled')} [${branch}]`;
    } catch {
      document.getElementById('git-status-text')!.textContent = t('hub.gitInstalled');
    }
  }
  else { gitEl.className = 'status-card err'; document.getElementById('git-status-text')!.textContent = t('hub.gitNotInstalled'); }
  (gitEl as HTMLElement).style.display = 'flex';

  try {
    const config = await api.getSyncConfig();
    (document.getElementById('repo-url') as HTMLInputElement).value = config.repo_url || '';
    (document.getElementById('username') as HTMLInputElement).value = config.username || '';
    (document.getElementById('token') as HTMLInputElement).value = config.token || '';
    (document.getElementById('branch') as HTMLInputElement).value = config.branch || 'main';
    if (config.auto_sync) document.getElementById('auto-sync')!.classList.add('on');
  } catch (e) { console.error('加载配置失败:', e); }

  document.getElementById('auto-sync')?.addEventListener('click', () => { document.getElementById('auto-sync')!.classList.toggle('on'); });

  function getSyncConfig() {
    return {
      repo_url: (document.getElementById('repo-url') as HTMLInputElement).value.trim(),
      username: (document.getElementById('username') as HTMLInputElement).value.trim(),
      token: (document.getElementById('token') as HTMLInputElement).value.trim(),
      branch: (document.getElementById('branch') as HTMLInputElement).value.trim() || 'main',
      auto_sync: document.getElementById('auto-sync')!.classList.contains('on'),
    };
  }

  document.getElementById('save-btn')?.addEventListener('click', async () => {
    try { await api.saveSyncConfig(getSyncConfig()); showSyncStatus(t('hub.configSaved'), 'success'); }
	    catch (e) { showSyncStatus(t('hub.saveFailed') + ': ' + e, 'error'); }
  });

  document.getElementById('sync-btn')?.addEventListener('click', async () => {
    const btn = document.getElementById('sync-btn') as HTMLElement;
    // 全屏蒙层
    const overlay = document.createElement('div');
    overlay.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.25);backdrop-filter:blur(2px);display:flex;align-items:center;justify-content:center;z-index:9999;';
    overlay.innerHTML = `<span style="color:var(--text-title);font-size:14px;font-weight:500;background:var(--surface);padding:12px 24px;border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,0.15);">${t('hub.syncing')}</span>`;
    document.body.appendChild(overlay);
    btn.style.opacity = '0.6'; btn.style.pointerEvents = 'none';
    try {
      // 先保存配置，再执行同步
      await api.saveSyncConfig(getSyncConfig());
      const result = await api.syncNotes() as string;
      console.log('[同步] 结果:', result);
      const branch = (document.getElementById('branch') as HTMLInputElement)?.value || 'main';
      showSyncStatus(`${result} [${branch}]`, 'success');
    } catch (e: any) {
      console.error('[同步] 失败:', e);
      const errMsg = String(e);
      // 检测分支不存在错误，提示用户是否创建分支
      if (errMsg.startsWith('BRANCH_NOT_FOUND:')) {
        const existingBranches = errMsg.substring('BRANCH_NOT_FOUND:'.length);
        const branchInput = document.getElementById('branch') as HTMLInputElement;
        const branchName = branchInput?.value || 'main';
        // 移除蒙层和恢复按钮
        if (overlay.parentNode) overlay.parentNode.removeChild(overlay);
        btn.style.opacity = ''; btn.style.pointerEvents = '';
        showBranchCreateDialog(branchName, existingBranches, async () => {
          // 用户确认创建分支
          const overlay2 = document.createElement('div');
          overlay2.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.25);backdrop-filter:blur(2px);display:flex;align-items:center;justify-content:center;z-index:9999;';
          overlay2.innerHTML = `<span style="color:var(--text-title);font-size:14px;font-weight:500;background:var(--surface);padding:12px 24px;border-radius:8px;box-shadow:0 4px 16px rgba(0,0,0,0.15);">${t('hub.syncing')}</span>`;
          document.body.appendChild(overlay2);
          btn.style.opacity = '0.6'; btn.style.pointerEvents = 'none';
          try {
            const result2 = await api.syncNotes(true) as string;
            showSyncStatus(result2, 'success');
          } catch (e2: any) {
            showSyncStatus(t('hub.syncFailed') + ': ' + e2, 'error');
          } finally {
            if (overlay2.parentNode) overlay2.parentNode.removeChild(overlay2);
            btn.style.opacity = ''; btn.style.pointerEvents = '';
          }
        });
      } else {
	      showSyncStatus(t('hub.syncFailed') + ': ' + e, 'error');
      }
    } finally {
      if (overlay.parentNode) overlay.parentNode.removeChild(overlay);
      btn.style.opacity = ''; btn.style.pointerEvents = '';
    }
  });

  function showSyncStatus(msg: string, type: string): void {
    const el = document.getElementById('sync-status')!;
    el.className = 'status-card ' + type;
    document.getElementById('sync-status-text')!.textContent = msg;
    (el as HTMLElement).style.display = 'flex';
    if (type !== 'loading') setTimeout(() => { (el as HTMLElement).style.display = 'none'; }, 5000);
  }

  function showBranchCreateDialog(branch: string, existingBranches: string, onConfirm: () => void): void {
    const dialog = document.createElement('div');
    dialog.style.cssText = 'position:fixed;inset:0;background:rgba(0,0,0,0.4);display:flex;align-items:center;justify-content:center;z-index:10000;';
    dialog.innerHTML = `
      <div style="background:var(--surface);border-radius:12px;padding:28px 32px;max-width:440px;box-shadow:0 8px 32px rgba(0,0,0,0.2);">
        <div style="font-size:16px;font-weight:600;color:var(--text-title);margin-bottom:12px;">${t('hub.branchNotFoundTitle')}</div>
        <div style="font-size:13px;color:var(--text-body);line-height:1.6;margin-bottom:8px;">
          ${t('hub.branchNotFoundMsg').replace('{branch}', branch).replace('{existing}', existingBranches)}
        </div>
        <div style="font-size:13px;color:var(--text-body);line-height:1.6;margin-bottom:20px;">
          ${t('hub.branchCreateConfirm')}
        </div>
        <div style="display:flex;gap:12px;justify-content:flex-end;">
          <button id="bc-cancel" style="padding:8px 20px;border:1px solid var(--border);border-radius:6px;background:transparent;color:var(--text-body);cursor:pointer;font-size:13px;">${t('hub.branchCancel')}</button>
          <button id="bc-ok" style="padding:8px 20px;border:none;border-radius:6px;background:var(--accent);color:#fff;cursor:pointer;font-size:13px;">${t('hub.branchCreate')}</button>
        </div>
      </div>
    `;
    document.body.appendChild(dialog);
    dialog.querySelector('#bc-cancel')!.addEventListener('click', () => { dialog.remove(); });
    dialog.querySelector('#bc-ok')!.addEventListener('click', () => { dialog.remove(); onConfirm(); });
  }
}
