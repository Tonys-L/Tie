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
        // 默认过滤墓碑（INV-032）
        Ok(self
            .notes
            .lock()
            .unwrap()
            .get(id)
            .filter(|n| !n.is_deleted())
            .cloned())
    }

    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Note>, String> {
        // 含墓碑，供 sync import 仲裁用（INV-032）
        Ok(self.notes.lock().unwrap().get(id).cloned())
    }

    fn find_all(&self) -> Result<Vec<Note>, String> {
        let notes = self.notes.lock().unwrap();
        let mut result: Vec<Note> = notes
            .values()
            .filter(|n| !n.is_archived && !n.is_deleted())
            .cloned()
            .collect();
        // 与 SqliteNoteRepository::find_all 排序对齐：is_pinned DESC, updated_at DESC
        result.sort_by(|a, b| {
            b.is_pinned
                .cmp(&a.is_pinned)
                .then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        Ok(result)
    }

    fn find_all_including_deleted(&self) -> Result<Vec<Note>, String> {
        // 含墓碑，供 sync export 用（INV-032）
        let notes = self.notes.lock().unwrap();
        let mut result: Vec<Note> = notes.values().cloned().collect();
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(result)
    }

    fn physical_delete(&self, id: &str) -> Result<(), String> {
        // 硬删除，仅供墓碑清理使用（INV-032）
        self.notes.lock().unwrap().remove(id);
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        // 保留向后兼容的硬删除（service 层已改用 Note::delete() + save 软删除）
        self.notes.lock().unwrap().remove(id);
        Ok(())
    }

    fn find_archived(&self) -> Result<Vec<Note>, String> {
        let notes = self.notes.lock().unwrap();
        let mut result: Vec<Note> = notes
            .values()
            .filter(|n| n.is_archived && !n.is_deleted())
            .cloned()
            .collect();
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
                !n.is_deleted() // 过滤墓碑（INV-032）
                    && (n.title.to_lowercase().contains(&q)
                        || n.content.to_lowercase().contains(&q)
                        || n.tags.iter().any(|t| t.to_lowercase().contains(&q)))
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
            .filter(|n| !n.is_deleted()) // 过滤墓碑（INV-032）
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
        // 默认过滤墓碑（INV-032）
        Ok(self
            .reminders
            .lock()
            .unwrap()
            .get(id)
            .filter(|r| !r.is_deleted())
            .cloned())
    }

    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Reminder>, String> {
        // 含墓碑，供 sync import 仲裁用（INV-032）
        Ok(self.reminders.lock().unwrap().get(id).cloned())
    }

    fn find_all(&self) -> Result<Vec<Reminder>, String> {
        let reminders = self.reminders.lock().unwrap();
        let mut result: Vec<Reminder> = reminders
            .values()
            .filter(|r| !r.is_deleted())
            .cloned()
            .collect();
        // 与 SqliteReminderRepository::find_all 排序对齐：remind_at ASC
        result.sort_by(|a, b| a.remind_at.cmp(&b.remind_at));
        Ok(result)
    }

    fn find_all_including_deleted(&self) -> Result<Vec<Reminder>, String> {
        // 含墓碑，供 sync export 用（INV-032）
        let reminders = self.reminders.lock().unwrap();
        let mut result: Vec<Reminder> = reminders.values().cloned().collect();
        result.sort_by(|a, b| a.remind_at.cmp(&b.remind_at));
        Ok(result)
    }

    fn find_by_note_id(&self, note_id: &str) -> Result<Vec<Reminder>, String> {
        let reminders = self.reminders.lock().unwrap();
        let mut result: Vec<Reminder> = reminders
            .values()
            .filter(|r| r.note_id == note_id && !r.is_deleted())
            .cloned()
            .collect();
        // 与 SqliteReminderRepository::find_by_note_id 排序对齐：remind_at ASC
        result.sort_by(|a, b| a.remind_at.cmp(&b.remind_at));
        Ok(result)
    }

    fn physical_delete(&self, id: &str) -> Result<(), String> {
        // 硬删除，仅供墓碑清理使用（INV-032）
        self.reminders.lock().unwrap().remove(id);
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        // 保留向后兼容的硬删除（service 层已改用 Reminder::delete() + save 软删除）
        self.reminders.lock().unwrap().remove(id);
        Ok(())
    }

    fn delete_by_note_id(&self, note_id: &str) -> Result<(), String> {
        // 保留向后兼容的硬删除（service 层已改用逐个 Reminder::delete() + save 软删除）
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
            .filter(|r| !r.is_deleted() && r.is_due(now)) // 过滤墓碑（INV-032）
            .cloned()
            .collect())
    }

    fn find_next_due_time(&self) -> Result<Option<String>, String> {
        let reminders = self.reminders.lock().unwrap();
        let pending: Vec<&Reminder> = reminders
            .values()
            .filter(|r| {
                !r.is_deleted() // 过滤墓碑
                    && matches!(r.status, super::ReminderStatus::Pending)
            })
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
                !r.is_deleted() // 过滤墓碑（INV-032）
                    && {
                        let t = r.effective_time();
                        t >= start && t < end
                    }
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
        let mut result: Vec<Template> = templates
            .values()
            .filter(|t| !t.is_deleted()) // 过滤墓碑（INV-032）
            .cloned()
            .collect();
        // 与 SqliteTemplateRepository::find_all 排序对齐：sort_order ASC, created_at ASC
        result.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(result)
    }

    fn find_all_including_deleted(&self) -> Result<Vec<Template>, String> {
        // 含墓碑，供 sync export 用（INV-032）
        let templates = self.templates.lock().unwrap();
        let mut result: Vec<Template> = templates.values().cloned().collect();
        result.sort_by(|a, b| {
            a.sort_order
                .cmp(&b.sort_order)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        Ok(result)
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Template>, String> {
        // 默认过滤墓碑（INV-032）
        Ok(self
            .templates
            .lock()
            .unwrap()
            .get(id)
            .filter(|t| !t.is_deleted())
            .cloned())
    }

    fn find_by_id_including_deleted(&self, id: &str) -> Result<Option<Template>, String> {
        // 含墓碑，供 sync import 仲裁用（INV-032）
        Ok(self.templates.lock().unwrap().get(id).cloned())
    }

    fn physical_delete(&self, id: &str) -> Result<(), String> {
        // 硬删除，仅供墓碑清理使用（INV-032）
        self.templates.lock().unwrap().remove(id);
        Ok(())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        // 保留向后兼容的硬删除（service 层已改用 Template::delete() + save 软删除）
        self.templates.lock().unwrap().remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Note mock 墓碑测试 ============

    #[test]
    fn find_all_excludes_tombstones() {
        // find_all 不返回墓碑
        let repo = InMemoryNoteRepository::new();
        let mut note1 = Note::new("活跃".to_string(), "amber".to_string());
        note1.id = "n1".to_string();
        let mut note2 = Note::new("墓碑".to_string(), "amber".to_string());
        note2.id = "n2".to_string();
        note2.delete();

        repo.save(&note1).unwrap();
        repo.save(&note2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "n1");
    }

    #[test]
    fn find_all_including_deleted_returns_all() {
        // find_all_including_deleted 返回含墓碑
        let repo = InMemoryNoteRepository::new();
        let mut note1 = Note::new("活跃".to_string(), "amber".to_string());
        note1.id = "n1".to_string();
        let mut note2 = Note::new("墓碑".to_string(), "amber".to_string());
        note2.id = "n2".to_string();
        note2.delete();

        repo.save(&note1).unwrap();
        repo.save(&note2).unwrap();

        let all = repo.find_all_including_deleted().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn find_by_id_excludes_tombstone() {
        // find_by_id 对墓碑返回 None
        let repo = InMemoryNoteRepository::new();
        let mut note = Note::new("墓碑".to_string(), "amber".to_string());
        note.id = "n1".to_string();
        note.delete();
        repo.save(&note).unwrap();

        assert!(repo.find_by_id("n1").unwrap().is_none());
        // find_by_id_including_deleted 返回墓碑
        assert!(repo.find_by_id_including_deleted("n1").unwrap().is_some());
    }

    #[test]
    fn physical_delete_removes_record() {
        // physical_delete 真正删除（连墓碑都不留）
        let repo = InMemoryNoteRepository::new();
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.id = "n1".to_string();
        note.delete();
        repo.save(&note).unwrap();

        repo.physical_delete("n1").unwrap();
        assert!(repo.find_by_id_including_deleted("n1").unwrap().is_none());
    }

    #[test]
    fn search_notes_excludes_tombstones() {
        // search_notes 过滤墓碑
        let repo = InMemoryNoteRepository::new();
        let mut note = Note::new("测试关键词".to_string(), "amber".to_string());
        note.id = "n1".to_string();
        note.delete();
        repo.save(&note).unwrap();

        let result = repo.search_notes("关键词").unwrap();
        assert!(result.is_empty());
    }

    // ============ Reminder mock 墓碑测试 ============

    #[test]
    fn reminder_find_all_excludes_tombstones() {
        let repo = InMemoryReminderRepository::new();
        let mut r1 = Reminder::new("note-1".to_string(), "活跃".to_string(), "2026-07-03T08:00:00Z".to_string(), "once".to_string());
        r1.id = "r1".to_string();
        let mut r2 = Reminder::new("note-1".to_string(), "墓碑".to_string(), "2026-07-03T08:00:00Z".to_string(), "once".to_string());
        r2.id = "r2".to_string();
        r2.delete();

        repo.save(&r1).unwrap();
        repo.save(&r2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "r1");

        let all_inc = repo.find_all_including_deleted().unwrap();
        assert_eq!(all_inc.len(), 2);
    }

    #[test]
    fn reminder_physical_delete_removes_record() {
        let repo = InMemoryReminderRepository::new();
        let mut r = Reminder::new("note-1".to_string(), "测试".to_string(), "2026-07-03T08:00:00Z".to_string(), "once".to_string());
        r.id = "r1".to_string();
        r.delete();
        repo.save(&r).unwrap();

        repo.physical_delete("r1").unwrap();
        assert!(repo.find_by_id_including_deleted("r1").unwrap().is_none());
    }

    // ============ Template mock 墓碑测试 ============

    #[test]
    fn template_find_all_excludes_tombstones() {
        let repo = InMemoryTemplateRepository::new();
        let mut t1 = Template::new("活跃".to_string(), "内容".to_string());
        t1.id = "t1".to_string();
        let mut t2 = Template::new("墓碑".to_string(), "内容".to_string());
        t2.id = "t2".to_string();
        t2.delete();

        repo.save(&t1).unwrap();
        repo.save(&t2).unwrap();

        let all = repo.find_all().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "t1");

        let all_inc = repo.find_all_including_deleted().unwrap();
        assert_eq!(all_inc.len(), 2);
    }

    #[test]
    fn template_physical_delete_removes_record() {
        let repo = InMemoryTemplateRepository::new();
        let mut t = Template::new("测试".to_string(), "内容".to_string());
        t.id = "t1".to_string();
        t.delete();
        repo.save(&t).unwrap();

        repo.physical_delete("t1").unwrap();
        assert!(repo.find_by_id_including_deleted("t1").unwrap().is_none());
    }
}
