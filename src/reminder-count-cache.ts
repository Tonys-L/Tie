/**
 * 提醒数量缓存（候选4：消除 notes-list.ts 中 _reminderCount 缓散布）。
 *
 * 职责：
 * - bulkLoadCounts：并行查询多条便签的 pending 提醒数量并缓存
 * - getCount：同步读取单条便签的提醒数量（未加载返回 0）
 * - hasCount：是否已缓存（避免重复加载）
 * - invalidate：清除单条或全部缓存
 *
 * 被调用方：notes-list.ts (loadNotes / searchInput / renderList)
 * 依赖：api.ts (getReminders)
 *
 * 设计目的：
 * - 消除 Note 对象上的 _reminderCount 派生字段（类型污染，用 any[] 规避检查）
 * - 集中缓存写入/读取/失效逻辑，避免散布在 4 个函数
 * - 避免 N+1 查询（10 条便签 = 10 次 IPC → 批量并行 + 缓存）
 */

import * as api from './api';

const cache = new Map<string, number>();

/**
 * 并行查询多条便签的 pending 提醒数量并缓存。
 * 已缓存的便签跳过（避免重复查询）。
 */
export async function bulkLoadCounts(noteIds: string[]): Promise<void> {
  await Promise.allSettled(
    noteIds
      .filter((id) => !cache.has(id))
      .map(async (id) => {
        try {
          const reminders = await api.getReminders(id);
          const count = (reminders as unknown[]).filter(
            (r) => (r as { status: string }).status === 'pending',
          ).length;
          cache.set(id, count);
        } catch {
          cache.set(id, 0);
        }
      }),
  );
}

/**
 * 同步读取单条便签的提醒数量。
 * 未加载返回 0（调用方应先调 bulkLoadCounts）。
 */
export function getCount(noteId: string): number {
  return cache.get(noteId) ?? 0;
}

/**
 * 是否已缓存该便签的提醒数量。
 */
export function hasCount(noteId: string): boolean {
  return cache.has(noteId);
}

/**
 * 清除缓存：传 noteId 清除单条，不传清除全部。
 */
export function invalidate(noteId?: string): void {
  if (noteId) {
    cache.delete(noteId);
  } else {
    cache.clear();
  }
}

/**
 * 统计有提醒的便签数量（用于 Hub 顶部 count-reminders 徽章）。
 */
export function countNotesWithReminders(noteIds: string[]): number {
  return noteIds.filter((id) => getCount(id) > 0).length;
}
