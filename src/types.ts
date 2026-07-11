// ============ 共享类型定义 ============

export interface Note {
  id: string;
  title: string;
  content: string;
  color: string;
  opacity: number;
  window_state: { pos_x: number; pos_y: number; width: number; height: number };
  is_pinned: boolean;
  created_at: string;
  updated_at: string;
}

export interface Reminder {
  id: string;
  note_id: string;
  note_title: string;
  remind_at: string;
  repeat_type: string;
  status: string;
}

export interface SyncConfig {
  repo_url: string;
  username: string;
  token: string;
  branch: string;
  auto_sync: boolean;
}

export interface ShortcutConfig {
  new_note: string;
  show_all: string;
}
