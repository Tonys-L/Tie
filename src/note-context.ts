/**
 * 便签窗口共享状态：当前便签 + 当前提醒 ID。
 *
 * 职责边界：
 * - 集中管理被多个 UI 部件模块共享读写的状态
 * - 提供 getter/setter，避免状态散布在各模块
 * - 不包含任何业务逻辑或 DOM 操作
 *
 * 被调用方：main.ts (初始化)、reminder-panel.ts、ai-sniff.ts、note-renderer.ts 等
 * 依赖：types.ts (Note 类型)
 */

import type { Note } from './types';

let currentNote: Note | null = null;
let currentReminderId: string | null = null;

export function setNote(note: Note | null): void {
  currentNote = note;
}

export function getNote(): Note {
  if (!currentNote) {
    throw new Error('currentNote not initialized');
  }
  return currentNote;
}

export function tryGetNote(): Note | null {
  return currentNote;
}

export function setCurrentReminderId(id: string | null): void {
  currentReminderId = id;
}

export function getCurrentReminderId(): string | null {
  return currentReminderId;
}
