/**
 * AI 嗅探建议面板：便签保存后自动嗅探并给出建议。
 *
 * 职责边界：
 * - setupAiSniffButton 初始化手动嗅探按钮（灯泡）+ 监听 ai-config-changed 事件
 * - sniffAfterSave 异步防抖调用嗅探（10s 防抖 + 内容变化检查 + sniff_enabled 预检查）
 * - showSuggestionPanel 右侧浮层显示建议列表 + 执行按钮
 * - executeSuggestion 按 type 分发（reminder/todo_split/tidy/style/tag_suggest）
 * - updateNoteContent 同步内存/textarea/查看区/后端的正文替换
 *
 * 被调用方：note-events.ts (setupAiSniffButton)、context-menu.ts (sniffAfterSave via main.ts)
 * 依赖：api.ts (getAiConfig/sniffSuggestions/createReminder/updateNoteContent/updateNoteTags) +
 *       ai-client.ts (isAiConfigured/getAiConfigCached) +
 *       markdown-renderer.ts (renderMarkdown) + tag-bar.ts (refreshTagBar) +
 *       html.ts (escapeHtml) + toast.ts (showToast) + i18n + types.ts (Suggestion/SniffResult)
 */

import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Note, SniffResult, Suggestion } from './types';
import { t } from './i18n';
import { escapeHtml } from './html';
import { showToast } from './toast';
import { isAiConfigured, getAiConfigCached } from './ai-client';
import * as api from './api';
import { renderMarkdown } from './markdown-renderer';
import { refreshTagBar } from './tag-bar';
import { AI_CONFIG_CHANGED } from './events';

// 每个便签最近一次嗅探时间戳（noteId → ts），10 秒内不重复嗅探同一便签
const sniffDebounceMap = new Map<string, number>();
const sniffContentMap = new Map<string, string>();
const SNIFF_DEBOUNCE_MS = 10_000;

// 当前面板自动消失定时器
let sniffPanelTimer: number | null = null;

/**
 * 初始化 AI 手动嗅探按钮：
 * - 异步检查 AI 配置，未配置时保持置灰并提示
 * - 已配置时启用按钮，点击触发 force 嗅探
 * - 加载中按钮禁用并显示"⏳ 处理中..."
 */
export function setupAiSniffButton(note: Note, app: HTMLElement): void {
  const btn = app.querySelector('[data-ai-sniff]') as HTMLButtonElement;
  if (!btn) return;

  // 更新灯泡按钮状态的辅助函数
  function updateSniffBtnState(configured: boolean) {
    btn.disabled = !configured;
    btn.title = configured ? t('hub.aiAssistant') : t('hub.aiNotConfigured');
  }

  // 异步检查 AI 配置（带缓存，ai-config-changed 事件触发时自动失效）
  isAiConfigured()
    .then(configured => {
      updateSniffBtnState(configured);
    });

  // 监听 AI 配置变更事件（Hub 保存配置后实时更新灯泡状态；ai-client 已监听并清缓存，这里只需重新查询）
  getCurrentWindow().listen(AI_CONFIG_CHANGED, () => {
    isAiConfigured()
      .then(configured => {
        updateSniffBtnState(configured);
      });
  });

  btn.addEventListener('click', async () => {
    if (btn.disabled) return;

    // 加载状态：禁止重复点击
    btn.disabled = true;
    const originalHTML = btn.innerHTML;
    btn.innerHTML = `<span style="font-size:11px;">${t('hub.sniffLoading')}</span>`;

    sniffAfterSave(note, true, (suggestions) => {
      // 恢复按钮
      btn.innerHTML = originalHTML;
      // 重新检查配置状态以决定是否启用（配置可能在加载时已就绪）
      // 失败时保持启用：嗅探刚完成说明之前 AI 可用，IPC 失败更可能是临时问题
      getAiConfigCached()
        .then(config => {
          updateSniffBtnState(!!(config && config.api_key && config.api_key.length > 0));
        })
        .catch(() => { updateSniffBtnState(true); });

      // 无建议时给一个轻提示
      if (suggestions.length === 0) {
        showSniffEmptyHint(app);
      }
    });
  });
}

/**
 * 嗅探无建议时的轻量提示（2 秒后自动消失）。
 */
function showSniffEmptyHint(app: HTMLElement): void {
  const existing = app.querySelector('.sniff-empty-hint');
  if (existing) existing.remove();
  const hint = document.createElement('div');
  hint.className = 'sniff-empty-hint';
  hint.textContent = t('hub.sniffNoSuggestions');
  app.appendChild(hint);
  setTimeout(() => hint.remove(), 2000);
}

/**
 * 便签保存后调用嗅探：异步、防抖、静默失败。
 * 命中建议则显示右侧 AI 建议面板。
 * 仅在内容变化时触发，避免无谓 AI 调用。
 *
 * force=true 时绕过防抖和内容变化检查（手动触发），
 * 失败时通过可选回调通知调用方。
 */
export async function sniffAfterSave(note: Note, force: boolean = false, onDone?: (suggestions: Suggestion[]) => void): Promise<void> {
  if (!force) {
    // 内容未变则跳过
    const lastContent = sniffContentMap.get(note.id);
    if (lastContent === note.content) { if (onDone) onDone([]); return; }
    // 防抖：10 秒内不重复嗅探同一便签
    const now = Date.now();
    const last = sniffDebounceMap.get(note.id) || 0;
    if (now - last < SNIFF_DEBOUNCE_MS) { if (onDone) onDone([]); return; }
    // 前端预检查嗅探开关：关闭则直接跳过，不发起 IPC 调用
    try {
      const config = await getAiConfigCached();
      if (!config.sniff_enabled) { if (onDone) onDone([]); return; }
    } catch { /* 读取配置失败则继续，后端会再次校验 */ }
    sniffDebounceMap.set(note.id, now);
    sniffContentMap.set(note.id, note.content);
  }

  // AI 分析完全异步，不阻塞保存流程；失败静默
  api.sniffSuggestions(note.content)
    .then(suggestions => {
      if (suggestions && suggestions.length > 0) {
        showSuggestionPanel(note, suggestions);
      }
      if (onDone) onDone(suggestions || []);
    })
    .catch(err => {
      console.error('AI分析失败:', err);
      // 失败时给用户提示，而非静默显示"未发现可优化项"
      const msg = typeof err === 'string' ? err : (err as Error).message || String(err);
      showToast(t('hub.sniffFailed') + ': ' + msg, 'error');
      if (onDone) onDone([]);
    });
}

/**
 * 在便签窗口右侧显示 AI 建议面板（半透明浮层，不占主编辑区）。
 * 同一时间只保留一个面板；10 秒后自动消失。
 */
function showSuggestionPanel(note: Note, suggestions: Suggestion[]): void {
  // 移除已有面板
  const existing = document.querySelector('.sniff-panel');
  if (existing) existing.remove();
  if (sniffPanelTimer !== null) {
    clearTimeout(sniffPanelTimer);
    sniffPanelTimer = null;
  }

  const panel = document.createElement('div');
  panel.className = 'sniff-panel';
  panel.innerHTML = `
    <div class="sniff-panel-header">
      <span class="sniff-panel-title">${t('hub.aiSuggestions')}</span>
      <button class="sniff-panel-close" data-panel-close title="${t('note.close')}">&times;</button>
    </div>
    <div class="sniff-panel-list">
      ${suggestions.map((s, i) => `
        <div class="sniff-item" data-item-index="${i}">
          <div class="sniff-item-title">${escapeHtml(s.title)}</div>
          <div class="sniff-item-desc">${escapeHtml(s.description)}</div>
          <button class="sniff-item-exec" data-exec-index="${i}">${t('hub.execute')}</button>
        </div>
      `).join('')}
    </div>
  `;
  document.body.appendChild(panel);

  const removePanel = () => {
    panel.remove();
    if (sniffPanelTimer !== null) {
      clearTimeout(sniffPanelTimer);
      sniffPanelTimer = null;
    }
  };

  // 关闭按钮
  panel.querySelector('[data-panel-close]')!.addEventListener('click', removePanel);

  // 执行按钮分发
  suggestions.forEach((suggestion, i) => {
    const execBtn = panel.querySelector(`[data-exec-index="${i}"]`) as HTMLButtonElement;
    execBtn.addEventListener('click', async () => {
      const item = panel.querySelector(`[data-item-index="${i}"]`) as HTMLElement;
      execBtn.disabled = true;
      try {
        await executeSuggestion(note, suggestion);
        // 成功：该项变为绿色"已执行"状态，2 秒后面板消失
        item.classList.add('executed');
        item.innerHTML = `
          <div class="sniff-item-title">${escapeHtml(suggestion.title)}</div>
          <div class="sniff-item-done">${t('hub.executed')}</div>
        `;
        // 执行成功后不消失，用户可能还要执行其他建议
      } catch (e) {
        // 失败：该项显示红色错误提示，恢复按钮可点击
        console.error('执行建议失败:', e);
        item.classList.add('failed');
        const errDiv = document.createElement('div');
        errDiv.className = 'sniff-item-error';
        errDiv.textContent = String(e);
        // 避免重复追加错误提示
        if (!item.querySelector('.sniff-item-error')) {
          item.appendChild(errDiv);
        }
        execBtn.disabled = false;
      }
    });
  });

  // 不自动消失：只在用户点击关闭、再次分析、或关闭便签时消失
  if (sniffPanelTimer !== null) {
    clearTimeout(sniffPanelTimer);
    sniffPanelTimer = null;
  }
}

/**
 * 根据 suggestion.type 分发执行建议。
 * - reminder：调用 create_reminder，从 data 提取 start_time/title/repeat_type
 *   - start_time 格式 "YYYY-MM-DD HH:mm" → ISO
 *   - repeat_type === 'once' 映射为 'none'（后端要求）
 *   - 标题优先用 data.title，兜底 note.title
 * - todo_split：把字符串数组转为待办清单 Markdown，替换便签正文
 * - tidy：用规整后的文本替换便签正文
 * - style：用切换后的文本替换便签正文
 * - tag_suggest：把推荐标签追加到便签（去重，domain 层兜底限制）
 */
async function executeSuggestion(note: Note, suggestion: Suggestion): Promise<void> {
  switch (suggestion.type) {
    case 'reminder':
      await executeReminder(note, suggestion.data);
      break;
    case 'todo_split':
      await executeTodoSplit(note, suggestion.data);
      break;
    case 'tidy':
      await executeTidy(note, suggestion.data);
      break;
    case 'style':
      await executeStyle(note, suggestion.data);
      break;
    case 'tag_suggest':
      await executeTagSuggest(note, suggestion.data);
      break;
    default:
      throw new Error(`${t('hub.executeFailed')}: ${(suggestion as { type: string }).type}`);
  }
}

/**
 * 更新便签正文：同步内存/textarea/查看区/后端。
 * 用于 todo_split/tidy/style 三种建议的正文替换。
 */
function updateNoteContent(note: Note, newContent: string): void {
  note.content = newContent;
  const textarea = document.querySelector('[data-content]') as HTMLTextAreaElement | null;
  const contentView = document.querySelector('[data-content-view]') as HTMLElement | null;
  if (textarea) textarea.value = newContent;
  if (contentView) contentView.innerHTML = renderMarkdown(newContent);
  api.updateNoteContent(note.id, newContent);
}

/**
 * reminder：调用 create_reminder 创建提醒。
 * - start_time 格式 "YYYY-MM-DD HH:mm" → ISO
 * - repeat_type === 'once' 映射为 'none'（后端要求）
 * - 标题优先用 data.title，兜底 note.title
 */
async function executeReminder(note: Note, data: SniffResult): Promise<void> {
  const dt = new Date(data.start_time.replace(' ', 'T'));
  if (isNaN(dt.getTime())) {
    throw new Error('invalid start_time: ' + data.start_time);
  }
  const noteTitle = data.title || note.title || t('app.note');
  const repeatType = data.repeat_type === 'once' ? 'none' : data.repeat_type;
  await api.createReminder(note.id, noteTitle, dt.toISOString(), repeatType);
}

/**
 * todo_split：把字符串数组转为 GFM 待办清单 Markdown，替换便签正文。
 */
async function executeTodoSplit(note: Note, todos: string[]): Promise<void> {
  if (!Array.isArray(todos) || todos.length === 0) {
    throw new Error('empty todos');
  }
  const newContent = todos.map(todo => `- [ ] ${todo}`).join('\n');
  updateNoteContent(note, newContent);
}

/**
 * tidy：用规整后的文本替换便签正文。
 */
async function executeTidy(note: Note, tidyText: string): Promise<void> {
  if (typeof tidyText !== 'string' || !tidyText.trim()) {
    throw new Error('empty tidy text');
  }
  updateNoteContent(note, tidyText);
}

/**
 * style：用切换文风后的文本替换便签正文。
 */
async function executeStyle(note: Note, data: { style_type: string; styled_text: string }): Promise<void> {
  if (!data || !data.styled_text || !data.styled_text.trim()) {
    throw new Error('empty styled text');
  }
  updateNoteContent(note, data.styled_text);
}

/**
 * tag_suggest：把推荐标签追加到便签（前端去重，domain 层兜底限制数量/长度）。
 */
async function executeTagSuggest(note: Note, tags: string[]): Promise<void> {
  if (!Array.isArray(tags) || tags.length === 0) {
    throw new Error('empty tags');
  }
  // 前端去重：过滤掉便签已有的标签
  const existing = new Set(note.tags);
  const newTags = tags.filter(tag => tag && !existing.has(tag));
  if (newTags.length === 0) {
    // 全部已存在：无需调用后端，视为成功
    return;
  }
  const merged = [...note.tags, ...newTags];
  note.tags = merged;
  refreshTagBar(note);
  await api.updateNoteTags(note.id, merged);
}
