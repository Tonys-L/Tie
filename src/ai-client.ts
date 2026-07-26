/**
 * AI 客户端统一入口：配置缓存 + AI 调用包装（loading/error/toast 统一）。
 *
 * 职责边界：
 * - isAiConfigured()：带缓存的"AI 是否已配置"查询（避免每次右键菜单都发起 IPC）
 * - getAiConfigCached()：带缓存的完整配置读取（用于 ai-sniff 等需要 sniff_enabled 等字段的场景）
 * - runAi<T>()：统一包装 AI 调用，处理 loading toast / success toast / error toast
 *
 * 被调用方：
 * - context-menu.ts (isAiConfigured)
 * - ai-sniff.ts (isAiConfigured / getAiConfigCached)
 * - ai-todo-sort.ts (runAi 包装 aiSortTodos)
 * - ai-rewrite.ts (runAi 包装 aiRewriteText)
 *
 * 不被调用方（例外）：
 * - ai-settings.ts：配置页本身，每次刷新表单需要读取最新值，不使用缓存 → 仍直接调用 api.getAiConfig
 *
 * 依赖：api.ts (getAiConfig) + toast.ts (showToast) + i18n (t) + @tauri-apps/api/window (listen)
 *
 * 设计目的：
 * - 把分散在 4 个 AI module 中的 isAiConfigured 检查和 AI 调用错误处理集中到一处（locality）
 * - 缓存避免右键菜单每次打开都发起 IPC 查询（leverage）
 * - ai-config-changed 事件统一清缓存，配置变化后下次查询自动刷新
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import type { AiConfig } from './types';
import { t } from './i18n';
import { showToast } from './toast';
import * as api from './api';
import { AI_CONFIG_CHANGED } from './events';

// ===== 配置缓存 =====

let configCache: AiConfig | null = null;
let cacheTs = 0;
const CACHE_TTL_MS = 5000; // 5 秒内复用缓存，避免右键菜单频繁查询

// 监听 ai-config-changed 事件，清空缓存（Hub 保存配置后立即生效）
let listenerRegistered = false;
function ensureConfigChangeListener(): void {
  if (listenerRegistered) return;
  listenerRegistered = true;
  getCurrentWindow().listen(AI_CONFIG_CHANGED, () => {
    configCache = null;
    cacheTs = 0;
  });
}

/**
 * 读取 AI 配置（带 5 秒缓存，ai-config-changed 事件触发时清空）。
 * 用于需要完整配置字段（如 sniff_enabled）的调用方。
 */
export async function getAiConfigCached(): Promise<AiConfig> {
  ensureConfigChangeListener();
  const now = Date.now();
  if (configCache && now - cacheTs < CACHE_TTL_MS) {
    return configCache;
  }
  const config = await api.getAiConfig();
  configCache = config;
  cacheTs = now;
  return config;
}

/**
 * 查询 AI 是否已配置（api_key 非空）。
 * 带缓存，读取失败视为未配置。
 */
export async function isAiConfigured(): Promise<boolean> {
  try {
    const config = await getAiConfigCached();
    return !!(config && config.api_key && config.api_key.length > 0);
  } catch {
    return false;
  }
}

// ===== AI 调用包装 =====

interface RunAiOptions {
  /** loading toast 文案 key（默认 note.aiProcessing） */
  loadingMsg?: string;
  /** 成功 toast 文案（不传则不显示成功 toast） */
  successMsg?: string;
  /** 错误 toast 文案前缀（默认 note.aiFailed） */
  errorPrefix?: string;
  /** 是否静默失败（不显示 error toast，仅 console.error），默认 false */
  silentError?: boolean;
}

/**
 * 统一包装 AI 调用：loading toast → 调用 → 成功/失败 toast。
 *
 * - 成功：返回结果，可选显示 successMsg
 * - 失败：console.error + 显示 errorPrefix + 错误信息（除非 silentError=true），返回 undefined
 *
 * 用法：
 * ```ts
 * const result = await runAi(() => api.aiSortTodos(todos), {
 *   successMsg: t('note.aiSortDone'),
 * });
 * if (!result) return; // 失败或返回空
 * ```
 */
export async function runAi<T>(
  op: () => Promise<T>,
  opts: RunAiOptions = {},
): Promise<T | undefined> {
  const { loadingMsg, successMsg, errorPrefix, silentError } = opts;
  if (loadingMsg) {
    showToast(loadingMsg, 'info', true);
  }
  try {
    const result = await op();
    if (successMsg) {
      showToast(successMsg, 'success');
    }
    return result;
  } catch (e) {
    console.error('AI 调用失败:', e);
    if (!silentError) {
      const prefix = errorPrefix || t('note.aiFailed');
      showToast(`${prefix}: ${e}`, 'error');
    }
    return undefined;
  }
}
