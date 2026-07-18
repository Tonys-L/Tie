// ============ 共享类型定义 ============

export interface Note {
  id: string;
  title: string;
  content: string;
  color: string;
  opacity: number;
  window_state: { pos_x: number; pos_y: number; width: number; height: number };
  is_pinned: boolean;
  is_archived: boolean;
  tags: string[];
  created_at: string;
  updated_at: string;
  /** 搜索高亮片段（仅搜索结果填充，FTS5 snippet 生成，含 <mark> 标签） */
  highlight?: string;
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
  toggle_hub: string;
}

/** 便签模板（用户自定义） */
export interface Template {
  id: string;
  name: string;
  content: string;
  category: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

// ---- AI ----

export interface AiConfig {
  base_url: string;
  api_key: string;
  model: string;
  sniff_enabled: boolean;
}

export interface ReminderDraft {
  title: string;
  start_time: string;
  repeat_type: string;
  repeat_day: number | null;
}

// reminder 类型的嗅探结果（作为 Suggestion.data 的结构）
export interface SniffResult {
  detected: boolean;
  time_text: string;
  start_time: string;
  title: string;
  repeat_type: string;
  repeat_day: number | null;
}

// AI 嗅探返回的通用建议项（后端 sniff_suggestions 命令返回 Vec<Suggestion>）
export interface Suggestion {
  type: string;           // "reminder" / 未来 "todo_split" / "tidy" 等
  title: string;          // 简短标题，如"添加提醒"
  description: string;    // 详细描述，如"检测到"明天上午9点"，可添加提醒"
  data: any;              // 类型相关数据（reminder 类型为 SniffResult）
}

// AI 生成的报告草稿（后端 generate_report 命令返回）
export interface ReportDraft {
  title: string;    // 如 "2026-07-13 ~ 07-19 周报" 或 "2026-07 月报"
  content: string;  // Markdown 内容
}
