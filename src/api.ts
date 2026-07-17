// ============ API 层：统一封装所有 invoke 调用 ============

import { invoke } from '@tauri-apps/api/core';
import type { Note, Reminder, SyncConfig, ShortcutConfig, AiConfig, ReminderDraft, Suggestion, ReportDraft } from './types';

// ---- 便签 ----

export const createNote = (color?: string) => invoke<string>('create_note', { color });
export const getNote = (id: string) => invoke<Note>('get_note', { id });
export const getAllNotes = () => invoke<Note[]>('get_all_notes');
export const getArchivedNotes = () => invoke<Note[]>('get_archived_notes');
export const openNote = (id: string) => invoke('open_note', { id });
export const updateNoteContent = (id: string, content: string) => invoke('update_note_content', { id, content });
export const updateNoteTitle = (id: string, title: string) => invoke('update_note_title', { id, title });
export const updateNoteStyle = (id: string, color: string, opacity: number, isPinned: boolean) =>
  invoke('update_note_style', { id, color, opacity, isPinned });
export const updateNoteWindowState = (id: string, posX: number, posY: number, width: number, height: number) =>
  invoke('update_note_window_state', { id, posX, posY, width, height });
export const deleteNote = (id: string) => invoke('delete_note', { id });
export const archiveNote = (id: string) => invoke('archive_note', { id });
export const unarchiveNote = (id: string) => invoke('unarchive_note', { id });
export const searchNotes = (query: string) => invoke<Note[]>('search_notes', { query });
export const updateNoteTags = (id: string, tags: string[]) => invoke('update_note_tags', { id, tags });

// ---- 提醒 ----

export const createReminder = (noteId: string, noteTitle: string, remindAt: string, repeatType: string) =>
  invoke('create_reminder', { noteId, noteTitle, remindAt, repeatType });
export const getReminders = (noteId: string) => invoke<Reminder[]>('get_reminders', { noteId });
export const snoozeReminder = (id: string, minutes: number) => invoke('snooze_reminder', { id, minutes });
export const dismissReminder = (id: string) => invoke('dismiss_reminder', { id });
export const deleteReminder = (id: string) => invoke('delete_reminder', { id });
export const getRemindersByMonth = (year: number, month: number) =>
  invoke<Reminder[]>('get_reminders_by_month', { year, month });

export interface LunarDateInfo {
  day: number;
  lunar_text: string;
}

export const getLunarDates = (year: number, month: number) =>
  invoke<LunarDateInfo[]>('get_lunar_dates', { year, month });

export const getNotesActivityByMonth = (year: number, month: number) =>
  invoke<number[]>('get_notes_activity_by_month', { year, month });

// ---- 同步 ----

export const getSyncConfig = () => invoke<SyncConfig>('get_sync_config');
export const saveSyncConfig = (config: SyncConfig) => invoke('save_sync_config', { config });
export const syncNotes = (createBranch?: boolean) => invoke<string>('sync_notes', { createBranch });
export const checkGit = () => invoke<boolean>('check_git');

// ---- 快捷键 ----

export const getShortcutConfig = () => invoke<ShortcutConfig>('get_shortcut_config');
export const saveShortcutConfig = (config: ShortcutConfig) => invoke('save_shortcut_config', { config });

// ---- 数据目录 ----

export const getDataDir = () => invoke<string>('get_data_dir');
export const openDataDir = () => invoke('open_data_dir');

// ---- AI ----

export const getAiConfig = () => invoke<AiConfig>('get_ai_config');
export const saveAiConfig = (baseUrl: string, apiKey: string, model: string, sniffEnabled: boolean) =>
  invoke('save_ai_config', { baseUrl, apiKey, model, sniffEnabled });
export const testAiConnection = () => invoke<string>('test_ai_connection');
export const parseReminderNatural = (text: string) =>
  invoke<ReminderDraft>('parse_reminder_natural', { text });
export const sniffSuggestions = (content: string) =>
  invoke<Suggestion[]>('sniff_suggestions', { content });
export const generateReport = (periodType: 'weekly' | 'monthly', startDate: string, endDate: string) =>
  invoke<ReportDraft>('generate_report', { periodType, startDate, endDate });
