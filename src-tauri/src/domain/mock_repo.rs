use std::collections::HashMap;
use std::sync::Mutex;
use chrono::Datelike;

use super::{Note, NoteQuery, NoteRepository, Reminder, ReminderQuery, ReminderRepository, Template, TemplateRepository};

/// In-memory Note 仓储（仅用于测试）
///
/// 同时实现 [`NoteRepository`]（聚合 CRUD）和 [`NoteQuery`]（读投影），
/// 测试时可作为任一 trait object 注入（CQRS 风味拆分，ADR-010）。
pub struct InMemoryNoteRepository {
    notes: Mutex<HashMap<String, Note>>,
}

impl InMemoryNoteRepository {
    pub fn new() -> Self {
        Self {
            notes: Mutex::new(HashMap::new()),
        }
    }
}

impl NoteRepository for InMemoryNoteRepository {
    fn save(&self, note: &Note) -> Result<(), String> {
        self.notes
            .lock()
            .unwrap()
            .insert(note.id.clone(), note.clone());
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Note>, String> {
        Ok(self.notes.lock().unwrap().get(id).cloned())
    }

    fn find_all(&self) -> Result<Vec<Note>, String> {
        let notes = self.notes.lock().unwrap();
        let mut result: Vec<Note> = notes.values().filter(|n| !n.is_archived).cloned().collect();
        // 与 SqliteNoteRepository::find_all 排序对齐：is_pinned DESC, updated_at DESC
        result.sort_by(|a, b| {
            b.is_pinned
                .cmp(&a.is_pinned)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        Ok(result)
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        // 幂等删除：与 SqliteNoteRepository 对齐（DELETE WHERE id=? 删除 0 行也返回 Ok）。
        // 存在性守卫由 service 层负责（避免对不存在的项 emit 事件）。
        self.notes.lock().unwrap().remove(id);
        Ok(())
    }

    fn find_archived(&self) -> Result<Vec<Note>, String> {
        let notes = self.notes.lock().unwrap();
        let mut result: Vec<Note> = notes.values().filter(|n| n.is_archived).cloned().collect();
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(result)
    }
}

impl NoteQuery for InMemoryNoteRepository {
    fn search_notes(&self, query: &str) -> Result<Vec<Note>, String> {
        let q = query.to_lowercase();
        let notes = self.notes.lock().unwrap();
        let mut result: Vec<Note> = notes
            .values()
            .filter(|n| {
                n.title.to_lowercase().contains(&q)
                    || n.content.to_lowercase().contains(&q)
                    || n.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .cloned()
            .collect();
        // 与 SqliteNoteRepository::search_notes 排序对齐：is_pinned DESC, updated_at DESC
        result.sort_by(|a, b| {
            b.is_pinned
                .cmp(&a.is_pinned)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        Ok(result)
    }

    fn find_activity_by_month(&self, year: i32, month: u32) -> Result<Vec<u32>, String> {
        let notes = self.notes.lock().unwrap();
        let mut days: Vec<u32> = notes
            .values()
            .filter_map(|n| {
                for ts in &[&n.created_at, &n.updated_at] {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                        if dt.year() == year && dt.month() == month {
                            return Some(dt.day() as u32);
                        }
                    }
                }
                None
            })
            .collect();
        days.sort();
        days.dedup();
        Ok(days)
    }
}

/// In-memory Reminder 仓储（仅用于测试）
///
/// 同时实现 [`ReminderRepository`]（聚合 CRUD）和 [`ReminderQuery`]（读投影），
/// 测试时可作为任一 trait object 注入（CQRS 风味拆分，ADR-010）。
pub struct InMemoryReminderRepository {
    reminders: Mutex<HashMap<String, Reminder>>,
}

impl InMemoryReminderRepository {
    pub fn new() -> Self {
        Self {
            reminders: Mutex::new(HashMap::new()),
        }
    }
}

impl ReminderRepository for InMemoryReminderRepository {
    fn save(&self, reminder: &Reminder) -> Result<(), String> {
        self.reminders
            .lock()
            .unwrap()
            .insert(reminder.id.clone(), reminder.clone());
        Ok(())
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Reminder>, String> {
        Ok(self.reminders.lock().unwrap().get(id).cloned())
    }

    fn find_all(&self) -> Result<Vec<Reminder>, String> {
        let reminders = self.reminders.lock().unwrap();
        let mut result: Vec<Reminder> = reminders.values().cloned().collect();
        // 与 SqliteReminderRepository::find_all 排序对齐：remind_at ASC
        result.sort_by(|a, b| a.remind_at.cmp(&b.remind_at));
        Ok(result)
    }

    fn find_by_note_id(&self, note_id: &str) -> Result<Vec<Reminder>, String> {
        let reminders = self.reminders.lock().unwrap();
        let mut result: Vec<Reminder> = reminders
            .values()
            .filter(|r| r.note_id == note_id)
            .cloned()
            .collect();
        // 与 SqliteReminderRepository::find_by_note_id 排序对齐：remind_at ASC
        result.sort_by(|a, b| a.remind_at.cmp(&b.remind_at));
        Ok(result)
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        // 幂等删除：与 SqliteReminderRepository 对齐。
        self.reminders.lock().unwrap().remove(id);
        Ok(())
    }

    fn delete_by_note_id(&self, note_id: &str) -> Result<(), String> {
        let mut reminders = self.reminders.lock().unwrap();
        let ids: Vec<String> = reminders
            .values()
            .filter(|r| r.note_id == note_id)
            .map(|r| r.id.clone())
            .collect();
        for id in ids {
            reminders.remove(&id);
        }
        Ok(())
    }
}

impl ReminderQuery for InMemoryReminderRepository {
    fn find_due(&self, now: &str) -> Result<Vec<Reminder>, String> {
        Ok(self
            .reminders
            .lock()
            .unwrap()
            .values()
            .filter(|r| r.is_due(now))
            .cloned()
            .collect())
    }

    fn find_next_due_time(&self) -> Result<Option<String>, String> {
        let reminders = self.reminders.lock().unwrap();
        let pending: Vec<&Reminder> = reminders
            .values()
            .filter(|r| matches!(r.status, super::ReminderStatus::Pending))
            .collect();
        if pending.is_empty() {
            Ok(None)
        } else {
            let min_time = pending
                .iter()
                .map(|r| r.effective_time())
                .min()
                .unwrap()
                .to_string();
            Ok(Some(min_time))
        }
    }

    fn find_by_date_range(&self, start: &str, end: &str) -> Result<Vec<Reminder>, String> {
        Ok(self
            .reminders
            .lock()
            .unwrap()
            .values()
            .filter(|r| {
                let t = r.effective_time();
                t >= start && t < end
            })
            .cloned()
            .collect())
    }
}

/// In-memory Template 仓储（仅用于测试）
pub struct InMemoryTemplateRepository {
    templates: Mutex<HashMap<String, Template>>,
}

impl InMemoryTemplateRepository {
    pub fn new() -> Self {
        Self {
            templates: Mutex::new(HashMap::new()),
        }
    }
}

impl TemplateRepository for InMemoryTemplateRepository {
    fn save(&self, template: &Template) -> Result<(), String> {
        self.templates
            .lock()
            .unwrap()
            .insert(template.id.clone(), template.clone());
        Ok(())
    }

    fn find_all(&self) -> Result<Vec<Template>, String> {
        let templates = self.templates.lock().unwrap();
        let mut result: Vec<Template> = templates.values().cloned().collect();
        // 与 SqliteTemplateRepository::find_all 排序对齐：sort_order ASC, created_at ASC
        result.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(result)
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Template>, String> {
        Ok(self.templates.lock().unwrap().get(id).cloned())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        // 幂等删除：与 SqliteTemplateRepository 对齐。
        self.templates.lock().unwrap().remove(id);
        Ok(())
    }
}
