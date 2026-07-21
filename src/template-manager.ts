/**
 * 模板管理（Hub 页面）：模板增删改查 + 从模板新建便签。
 *
 * 职责边界：
 * - loadTemplates 加载模板列表，渲染列表 + 编辑器
 * - renderTplList 渲染左侧模板列表（高亮选中项）
 * - renderTplEditor 渲染右侧编辑器（名称/内容/保存/从模板新建/删除）
 * - openTplDialog/closeTplDialog 打开/关闭模板管理浮层
 * - 顶层按钮绑定：btn-templates / tpl-close / tpl-overlay / tpl-new
 *
 * 被调用方：hub.ts (顶层按钮绑定在模块加载时执行)
 * 依赖：api.ts (getTemplates/saveTemplate/deleteTemplate/createNoteFromTemplate) +
 *       utils.ts (escapeHtml/showToast) + i18n (t) + types (Template) +
 *       notes-list.ts (loadNotes 从模板新建后刷新便签列表)
 */

import type { Template } from './types';
import * as api from './api';
import { escapeHtml, showToast } from './utils';
import { t } from './i18n';
import { loadNotes } from './notes-list';

// ===== 模块级 state =====
let tplList: Template[] = [];
let tplSelectedId: string | null = null;

/** 加载模板列表，刷新列表 + 编辑器 */
async function loadTemplates(): Promise<void> {
  try {
    tplList = await api.getTemplates();
  } catch (e) {
    console.error('加载模板失败:', e);
    tplList = [];
  }
  tplSelectedId = null;
  renderTplList();
  renderTplEditor();
}

/** 渲染左侧模板列表 */
function renderTplList(): void {
  const listEl = document.getElementById('tpl-list')!;
  if (tplList.length === 0) {
    listEl.innerHTML = `<div class="tpl-empty">${t('hub.tplEmpty')}</div>`;
    return;
  }
  listEl.innerHTML = tplList.map(tp =>
    `<div class="tpl-item ${tplSelectedId === tp.id ? 'active' : ''}" data-tpl-id="${escapeHtml(tp.id)}">${escapeHtml(tp.name)}</div>`
  ).join('');
  listEl.querySelectorAll('[data-tpl-id]').forEach(item => {
    item.addEventListener('click', () => {
      tplSelectedId = (item as HTMLElement).dataset.tplId!;
      renderTplList();
      renderTplEditor();
    });
  });
}

/** 渲染右侧编辑器（名称/内容/保存/从模板新建/删除） */
function renderTplEditor(): void {
  const editorEl = document.getElementById('tpl-editor')!;
  const tpl = tplList.find(tp => tp.id === tplSelectedId);
  if (!tpl) {
    editorEl.innerHTML = `<div class="tpl-empty">${t('hub.tplEmpty')}</div>`;
    return;
  }
  editorEl.innerHTML = `
    <input type="text" class="tpl-name-input" id="tpl-name" value="${escapeHtml(tpl.name)}" placeholder="${t('hub.tplName')}" />
    <textarea class="tpl-content-input" id="tpl-content" placeholder="${t('hub.tplContent')}">${escapeHtml(tpl.content)}</textarea>
    <div class="tpl-actions">
      <button class="tpl-action-btn tpl-action-primary" id="tpl-save-btn">${t('hub.tplSave')}</button>
      <button class="tpl-action-btn" id="tpl-create-from-btn">${t('hub.tplCreateFrom')}</button>
      <button class="tpl-action-btn tpl-action-danger" id="tpl-delete-btn">${t('hub.tplDelete')}</button>
    </div>
  `;
  document.getElementById('tpl-save-btn')?.addEventListener('click', async () => {
    const name = (document.getElementById('tpl-name') as HTMLInputElement).value.trim();
    const content = (document.getElementById('tpl-content') as HTMLTextAreaElement).value;
    if (!name) {
      showToast(t('hub.tplNameRequired'), 'error');
      return;
    }
    try {
      const updated: Template = { ...tpl, name, content, updated_at: new Date().toISOString() };
      await api.saveTemplate(updated);
      const idx = tplList.findIndex(tp => tp.id === tpl.id);
      if (idx >= 0) tplList[idx] = updated;
      renderTplList();
      showToast(t('hub.tplSaved'), 'success');
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'error');
    }
  });
  document.getElementById('tpl-create-from-btn')?.addEventListener('click', async () => {
    try {
      await api.createNoteFromTemplate(tpl.id);
      closeTplDialog();
      showToast(t('hub.tplCreated'), 'success');
      loadNotes();
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'error');
    }
  });
  document.getElementById('tpl-delete-btn')?.addEventListener('click', async () => {
    if (!confirm(t('hub.tplDeleteConfirm'))) return;
    try {
      await api.deleteTemplate(tpl.id);
      tplList = tplList.filter(tp => tp.id !== tpl.id);
      tplSelectedId = null;
      renderTplList();
      renderTplEditor();
      showToast(t('hub.tplDeleted'), 'success');
    } catch (e) {
      showToast(t('hub.saveFailed') + ': ' + e, 'error');
    }
  });
}

function openTplDialog(): void {
  const overlay = document.getElementById('tpl-overlay')!;
  overlay.style.display = 'flex';
  loadTemplates();
}

function closeTplDialog(): void {
  const overlay = document.getElementById('tpl-overlay')!;
  overlay.style.display = 'none';
}

// ===== 顶层按钮绑定（模块加载时执行）=====

document.getElementById('btn-templates')?.addEventListener('click', openTplDialog);
document.getElementById('tpl-close')?.addEventListener('click', closeTplDialog);
document.getElementById('tpl-overlay')?.addEventListener('click', (e) => {
  if (e.target === e.currentTarget) closeTplDialog();
});

// 新建模板（前端生成 id，后端 INSERT ON CONFLICT 处理）
document.getElementById('tpl-new')?.addEventListener('click', async () => {
  const now = new Date().toISOString();
  const newTpl: Template = {
    id: 'tpl-' + (crypto.randomUUID?.() ?? Date.now().toString(36) + Math.random().toString(36).slice(2)),
    name: t('hub.tplNew').replace('+ ', '') || '新模板',
    content: '',
    category: 'custom',
    sort_order: tplList.length,
    created_at: now,
    updated_at: now,
  };
  try {
    await api.saveTemplate(newTpl);
    tplList.push(newTpl);
    tplSelectedId = newTpl.id;
    renderTplList();
    renderTplEditor();
    showToast(t('hub.tplNewCreated'), 'success');
  } catch (e) {
    showToast(t('hub.saveFailed') + ': ' + e, 'error');
  }
});
