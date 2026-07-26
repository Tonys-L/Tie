/**
 * 模板 UI：空便签快捷条 + 模板选择浮层（从模板新建/应用模板到当前便签）。
 *
 * 职责边界：
 * - setupTemplateQuickBar 空便签时显示快捷条，点击填充内容
 * - showTemplateDialog 通用浮层（标题 + 模板列表 + 选择回调）
 * - showTemplatePicker 「从模板新建便签」入口
 * - showTemplateApplier 「应用模板到当前便签」入口（非破坏性追加）
 *
 * 被调用方：note-events.ts (setupTemplateQuickBar)、context-menu.ts (showTemplatePicker/Applier)
 * 依赖：api.ts (getTemplates/createNoteFromTemplate/updateNoteContent) + markdown-renderer.ts +
 *       ai-todo-sort.ts (clearSortedMark + setupTodoSortButton) + html.ts (escapeHtml) + toast.ts (showToast) + i18n
 */

import type { Note, Template } from './types';
import { t } from './i18n';
import { escapeHtml } from './html';
import { showToast } from './toast';
import * as api from './api';
import { renderMarkdown } from './markdown-renderer';
import { clearSortedMark, setupTodoSortButton } from './ai-todo-sort';

/**
 * 空便签模板快捷条：当 note.content 为空时在 content-area 顶部显示模板按钮列表。
 * - 点击模板按钮 → 填充内容、保存、切回查看模式渲染 markdown、隐藏快捷条
 * - 用户输入内容后隐藏快捷条；内容被清空后重新显示
 * - 无模板时不显示
 */
export function setupTemplateQuickBar(note: Note, app: HTMLElement): void {
  const contentArea = app.querySelector('.content-area') as HTMLElement;
  if (!contentArea) return;

  const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;
  const contentView = app.querySelector('[data-content-view]') as HTMLElement;

  // 创建快捷条（absolute 定位，不影响内容布局）
  const bar = document.createElement('div');
  bar.className = 'tpl-quick-bar';
  bar.style.display = 'none';
  contentArea.appendChild(bar);

  // 控制显隐：内容为空且有模板时显示
  const updateVisibility = (hasTemplates: boolean) => {
    const isEmpty = !note.content.trim();
    bar.style.display = (isEmpty && hasTemplates) ? 'flex' : 'none';
  };

  // 监听 textarea 输入：内容变化时更新显隐
  textarea.addEventListener('input', () => {
    note.content = textarea.value;
    updateVisibility(true);
  });

  // 监听 textarea blur：内容可能被清空（用户编辑后删除所有内容）
  textarea.addEventListener('blur', () => {
    updateVisibility(true);
  });

  // 加载模板列表
  api.getTemplates()
    .then(templates => {
      if (templates.length === 0) {
        updateVisibility(false);
        return;
      }
      bar.innerHTML = `
        <span class="tpl-quick-label">${t('note.tplQuickTitle')}</span>
        <div class="tpl-quick-list">
          ${templates.map(tp =>
            `<button class="tpl-quick-btn" data-tpl-id="${escapeHtml(tp.id)}">${escapeHtml(tp.name)}</button>`
          ).join('')}
        </div>
      `;
      // 绑定模板按钮点击
      bar.querySelectorAll('[data-tpl-id]').forEach(btn => {
        btn.addEventListener('click', (ev) => {
          ev.stopPropagation();
          const tplId = (btn as HTMLElement).dataset.tplId!;
          const tpl = templates.find(tp => tp.id === tplId);
          if (!tpl) return;
          // 填充内容
          textarea.value = tpl.content;
          note.content = tpl.content;
          api.updateNoteContent(note.id, tpl.content);
          // 隐藏快捷条
          bar.style.display = 'none';
          // 切回查看模式并重新渲染 markdown
          if (contentView) {
            contentView.innerHTML = renderMarkdown(tpl.content);
            contentView.style.display = 'block';
            textarea.style.display = 'none';
          }
          // 清除待办排序标记，允许新内容重新检测排序按钮
          clearSortedMark(note.id);
          setupTodoSortButton(note, app);
        });
      });
      updateVisibility(true);
    })
    .catch(err => {
      console.error('加载模板失败:', err);
      updateVisibility(false);
    });
}

/**
 * 显示模板选择浮层（通用）。
 * @param title 浮层标题
 * @param onSelect 选择模板后的回调（传入 Template）
 */
export async function showTemplateDialog(
  title: string,
  app: HTMLElement,
  onSelect: (tpl: Template) => Promise<void>
): Promise<void> {
  // 移除已有浮层
  app.querySelector('.tpl-picker-overlay')?.remove();

  const overlay = document.createElement('div');
  overlay.className = 'tpl-picker-overlay';

  const dialog = document.createElement('div');
  dialog.className = 'tpl-picker-dialog';
  dialog.innerHTML = `
    <div class="tp-header">
      <span>${title}</span>
      <button class="tp-close">&times;</button>
    </div>
    <div class="tp-list" data-tp-list>
      <div class="tp-loading">...</div>
    </div>
  `;
  overlay.appendChild(dialog);
  app.appendChild(overlay);

  // 关闭事件
  const close = () => overlay.remove();
  dialog.querySelector('.tp-close')!.addEventListener('click', close);
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) close();
  });

  // 加载模板列表
  const listEl = dialog.querySelector('[data-tp-list]') as HTMLElement;
  try {
    const templates = await api.getTemplates();
    if (templates.length === 0) {
      listEl.innerHTML = `<div class="tp-empty">${t('note.tplQuickEmpty')}</div>`;
      return;
    }
    listEl.innerHTML = templates.map(tp =>
      `<button class="tp-item" data-tp-id="${escapeHtml(tp.id)}">${escapeHtml(tp.name)}</button>`
    ).join('');
    listEl.querySelectorAll('[data-tp-id]').forEach(btn => {
      btn.addEventListener('click', async () => {
        const tplId = (btn as HTMLElement).dataset.tpId!;
        const tpl = templates.find(tp => tp.id === tplId);
        if (!tpl) return;
        try {
          await onSelect(tpl);
          close();
        } catch (e) {
          showToast(t('note.executeFailed') + ': ' + e, 'error');
        }
      });
    });
  } catch (e) {
    listEl.innerHTML = `<div class="tp-empty">${t('note.tplQuickEmpty')}</div>`;
    console.error('加载模板失败:', e);
  }
}

/**
 * 「从模板新建便签」：调用后端创建新便签并打开新窗口，当前便签不受影响。
 */
export async function showTemplatePicker(_note: Note, app: HTMLElement): Promise<void> {
  await showTemplateDialog(t('note.tplPickerTitle'), app, async (tpl) => {
    await api.createNoteFromTemplate(tpl.id);
    showToast(t('note.tplCreated'), 'success');
  });
}

/**
 * 「应用模板到当前便签」：在当前便签 content 末尾追加 `\n\n` + 模板内容，保存并重新渲染。
 * - 非破坏性：不覆盖已有内容
 * - 触发 update_note_content 保存
 * - 重新渲染 content-view 的 markdown
 */
export async function showTemplateApplier(note: Note, app: HTMLElement): Promise<void> {
  await showTemplateDialog(t('note.tplApplierTitle'), app, async (tpl) => {
    // 在末尾追加模板内容（用空行分隔）
    const separator = note.content.trim() ? '\n\n' : '';
    const newContent = note.content + separator + tpl.content;
    // 更新内存 + UI + 后端
    note.content = newContent;
    const textarea = app.querySelector('[data-content]') as HTMLTextAreaElement;
    const contentView = app.querySelector('[data-content-view]') as HTMLElement;
    if (textarea) textarea.value = newContent;
    if (contentView) contentView.innerHTML = renderMarkdown(newContent);
    api.updateNoteContent(note.id, newContent);
    // 清除待办排序标记，允许新内容重新检测排序按钮
    clearSortedMark(note.id);
    setupTodoSortButton(note, app);
    showToast(t('note.tplApplied'), 'success');
  });
}
