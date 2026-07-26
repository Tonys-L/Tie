/**
 * AI 设置页（Hub 页面）：AI 配置加载/保存/测试 + 周报/月报生成。
 *
 * 职责边界：
 * - loadAiConfig 每次进入页面刷新表单，事件只绑定一次（防重复）
 * - 嗅探开关切换（视觉状态，保存时读取）
 * - 保存/测试连接按钮
 * - generateReport 生成周报/月报 → 创建新便签 → 打开窗口 → 刷新列表
 *
 * 被调用方：hub.ts (页面切换时调用 loadAiConfig) + 顶层按钮绑定（模块加载时执行）
 * 依赖：api.ts (getAiConfig/saveAiConfig/testAiConnection/generateReport/createNote/...) +
 *       toast.ts (showToast) + i18n (t) + notes-list.ts (loadNotes 刷新便签列表)
 */

import * as api from './api';
import { showToast } from './toast';
import { t } from './i18n';
import { loadNotes } from './notes-list';

// ===== 模块级 state =====
let aiConfigLoaded = false;

/** 加载 AI 配置表单（每次进入页面刷新），事件只绑定一次 */
export async function loadAiConfig(): Promise<void> {
  // 每次进入页面都刷新表单（配置可能在其他地方被修改）
  try {
    const config = await api.getAiConfig();
    (document.getElementById('ai-base-url') as HTMLInputElement).value = config.base_url || '';
    (document.getElementById('ai-api-key') as HTMLInputElement).value = config.api_key || '';
    (document.getElementById('ai-model') as HTMLInputElement).value = config.model || '';
    // 嗅探开关：sniff_enabled 默认 true（后端 serde default 保证）
    const sniffEl = document.getElementById('ai-sniff-enabled');
    if (sniffEl) {
      if (config.sniff_enabled) sniffEl.classList.add('on');
      else sniffEl.classList.remove('on');
    }
  } catch (e) { console.error('加载 AI 配置失败:', e); }

  // 事件只绑定一次
  if (aiConfigLoaded) return;
  aiConfigLoaded = true;

  // 嗅探开关切换（仅切换视觉状态，保存时读取）
  document.getElementById('ai-sniff-enabled')?.addEventListener('click', () => {
    document.getElementById('ai-sniff-enabled')!.classList.toggle('on');
  });

  document.getElementById('ai-save-btn')?.addEventListener('click', async () => {
    const baseUrl = (document.getElementById('ai-base-url') as HTMLInputElement).value.trim();
    const apiKey = (document.getElementById('ai-api-key') as HTMLInputElement).value.trim();
    const model = (document.getElementById('ai-model') as HTMLInputElement).value.trim();
    const sniffEnabled = document.getElementById('ai-sniff-enabled')!.classList.contains('on');
    try {
      await api.saveAiConfig(baseUrl, apiKey, model, sniffEnabled);
      showToast(t('hub.aiConfigSaved'), 'success');
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'error');
    }
  });

  document.getElementById('ai-test-btn')?.addEventListener('click', async () => {
    const btn = document.getElementById('ai-test-btn') as HTMLButtonElement;
    const baseUrl = (document.getElementById('ai-base-url') as HTMLInputElement).value.trim();
    const apiKey = (document.getElementById('ai-api-key') as HTMLInputElement).value.trim();
    const model = (document.getElementById('ai-model') as HTMLInputElement).value.trim();
    const sniffEnabled = document.getElementById('ai-sniff-enabled')!.classList.contains('on');
    if (!apiKey) {
      showToast(t('hub.aiNotConfigured'), 'error');
      return;
    }
    btn.textContent = t('hub.testing');
    btn.disabled = true;
    try {
      // 先保存当前表单值，测试连接使用最新配置
      await api.saveAiConfig(baseUrl, apiKey, model, sniffEnabled);
      const result = await api.testAiConnection();
      showAiStatus(t('hub.connectionSuccess') + ': ' + result, 'success');
      showToast(t('hub.connectionSuccess'), 'success');
    } catch (e) {
      showAiStatus(t('hub.connectionFailed') + ': ' + e, 'error');
      showToast(t('hub.connectionFailed'), 'error');
    } finally {
      btn.textContent = t('hub.testConnection');
      btn.disabled = false;
    }
  });
}

/** 显示 AI 测试状态卡片（5 秒自动消失） */
function showAiStatus(msg: string, type: string): void {
  const el = document.getElementById('ai-test-status')!;
  el.className = 'status-card ' + type;
  document.getElementById('ai-test-status-text')!.textContent = msg;
  (el as HTMLElement).style.display = 'flex';
  if (type !== 'loading') setTimeout(() => { (el as HTMLElement).style.display = 'none'; }, 5000);
}

// ===== AI 报告生成（周报/月报）=====

function formatDateISO(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

/** 上周一到周日（以周一为一周开始） */
function getLastWeekRange(): { start: string; end: string } {
  const now = new Date();
  const day = now.getDay(); // 0=周日, 1=周一
  const diffToLastMonday = day === 0 ? -6 : 1 - day;
  const monday = new Date(now);
  monday.setDate(now.getDate() + diffToLastMonday - 7);
  const sunday = new Date(monday);
  sunday.setDate(monday.getDate() + 6);
  return { start: formatDateISO(monday), end: formatDateISO(sunday) };
}

/** 本月1号到月末 */
function getThisMonthRange(): { start: string; end: string } {
  const now = new Date();
  const first = new Date(now.getFullYear(), now.getMonth(), 1);
  const last = new Date(now.getFullYear(), now.getMonth() + 1, 0);
  return { start: formatDateISO(first), end: formatDateISO(last) };
}

/** 生成周报/月报 → 创建新便签 → 打开窗口 → 刷新列表 */
async function generateReport(periodType: 'weekly' | 'monthly'): Promise<void> {
  const btnId = periodType === 'weekly' ? 'btn-generate-weekly-report' : 'btn-generate-monthly-report';
  const btn = document.getElementById(btnId) as HTMLButtonElement;
  if (!btn) return;

  const originalText = btn.textContent;
  btn.disabled = true;
  btn.textContent = t('hub.reportGenerating');

  try {
    const range = periodType === 'weekly' ? getLastWeekRange() : getThisMonthRange();
    const draft = await api.generateReport(periodType, range.start, range.end);

    // 创建新便签并填充内容
    const noteId = await api.createNote();
    await api.updateNoteContent(noteId, draft.content);
    await api.updateNoteTitle(noteId, draft.title);
    await api.openNote(noteId);

    showToast(t('hub.reportGenerated'), 'success');
    // 刷新便签列表以显示新建的便签
    loadNotes();
  } catch (e) {
    console.error('生成报告失败:', e);
    showToast(t('hub.reportGenerateFailed') + ': ' + e, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = originalText;
  }
}

// 顶层按钮绑定（模块加载时执行）
document.getElementById('btn-generate-weekly-report')?.addEventListener('click', () => generateReport('weekly'));
document.getElementById('btn-generate-monthly-report')?.addEventListener('click', () => generateReport('monthly'));
