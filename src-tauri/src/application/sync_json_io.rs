use std::path::Path;

use crate::domain::{Note, NoteRepository, Reminder, ReminderRepository};

/// 导出所有便签和提醒为 JSON 文件
pub fn export_to_json(
    sync_dir: &Path,
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
) -> Result<(), String> {
    let notes_dir = sync_dir.join("notes");
    let reminders_dir = sync_dir.join("reminders");
    std::fs::create_dir_all(&notes_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    std::fs::create_dir_all(&reminders_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    // 导出便签（活跃 + 归档）
    let notes = note_repo.find_all().map_err(|e| format!("查询便签失败: {}", e))?;
    let archived = note_repo.find_archived().map_err(|e| format!("查询归档失败: {}", e))?;
    let all_notes: Vec<&Note> = notes.iter().chain(archived.iter()).collect();

    // 清除旧文件（处理已删除的便签）
    clear_dir_json(&notes_dir)?;
    for note in all_notes {
        let json = serde_json::to_string_pretty(note)
            .map_err(|e| format!("序列化便签失败: {}", e))?;
        let path = notes_dir.join(format!("{}.json", note.id));
        std::fs::write(&path, json).map_err(|e| format!("写入文件失败: {}", e))?;
    }

    // 导出提醒
    let reminders = reminder_repo.find_all().map_err(|e| format!("查询提醒失败: {}", e))?;
    clear_dir_json(&reminders_dir)?;
    for reminder in &reminders {
        let json = serde_json::to_string_pretty(reminder)
            .map_err(|e| format!("序列化提醒失败: {}", e))?;
        let path = reminders_dir.join(format!("{}.json", reminder.id));
        std::fs::write(&path, json).map_err(|e| format!("写入文件失败: {}", e))?;
    }

    Ok(())
}

/// 从 JSON 文件导入到数据库（upsert，按 updated_at 取最新）
pub fn import_from_json(
    sync_dir: &Path,
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
) -> Result<usize, String> {
    let mut imported = 0;

    // 导入便签
    let notes_dir = sync_dir.join("notes");
    if notes_dir.exists() {
        for entry in std::fs::read_dir(&notes_dir).map_err(|e| format!("读取目录失败: {}", e))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = std::fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {}", e))?;
            let note: Note = serde_json::from_str(&content).map_err(|e| format!("解析便签失败: {}", e))?;

            // upsert：仅远程比本地新时才覆盖（last-write-wins）
            let should_save = match note_repo.find_by_id(&note.id)? {
                Some(existing) => note.updated_at > existing.updated_at,
                None => true,
            };

            if should_save {
                note_repo.save(&note)?;
                imported += 1;
            }
        }
    }

    // 导入提醒（逻辑与便签一致）
    let reminders_dir = sync_dir.join("reminders");
    if reminders_dir.exists() {
        for entry in std::fs::read_dir(&reminders_dir).map_err(|e| format!("读取目录失败: {}", e))? {
            let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let content = std::fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {}", e))?;
            let reminder: Reminder = serde_json::from_str(&content).map_err(|e| format!("解析提醒失败: {}", e))?;

            let should_save = match reminder_repo.find_by_id(&reminder.id)? {
                Some(existing) => reminder.updated_at > existing.updated_at,
                None => true,
            };

            if should_save {
                reminder_repo.save(&reminder)?;
                imported += 1;
            }
        }
    }

    Ok(imported)
}

/// 清空目录中的 JSON 文件
fn clear_dir_json(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))? {
        if let Ok(e) = entry {
            let _ = std::fs::remove_file(e.path());
        }
    }
    Ok(())
}

/// 从 JSON 字符串中提取 updated_at 字段值
pub fn extract_updated_at(json: &str) -> String {
    if let Some(idx) = json.find("\"updated_at\"") {
        let rest = &json[idx..];
        // rest = `"updated_at":"value",...`
        // 跳过键名，找到冒号
        if let Some(colon_idx) = rest.find(':') {
            let after_colon = &rest[colon_idx + 1..];
            // after_colon = `"value",...` 或 ` "value",...`
            // 找到值的开始引号
            if let Some(open_quote) = after_colon.find('"') {
                let val_rest = &after_colon[open_quote + 1..];
                // val_rest = `value",...`
                if let Some(val_end) = val_rest.find('"') {
                    return val_rest[..val_end].to_string();
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository};
    use crate::domain::{Note, Reminder};

    /// 创建临时目录
    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tie_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_export_import_roundtrip() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        // 准备数据
        let note = Note::new("测试".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        let reminder = Reminder::new(
            note.id.clone(),
            "测试便签".to_string(),
            "2026-07-13T10:00:00Z".to_string(),
            "once".to_string(),
        );
        reminder_repo.save(&reminder).unwrap();

        // 导出
        export_to_json(&dir, &note_repo, &reminder_repo).unwrap();

        // 导入到新仓储
        let note_repo2 = InMemoryNoteRepository::new();
        let reminder_repo2 = InMemoryReminderRepository::new();
        let imported = import_from_json(&dir, &note_repo2, &reminder_repo2).unwrap();

        assert_eq!(imported, 2); // 1 note + 1 reminder

        // 验证便签
        let found_note = note_repo2.find_by_id(&note.id).unwrap();
        assert!(found_note.is_some());
        assert_eq!(found_note.unwrap().color.as_str(), "amber");

        // 验证提醒
        let found_reminder = reminder_repo2.find_by_id(&reminder.id).unwrap();
        assert!(found_reminder.is_some());
        assert_eq!(found_reminder.unwrap().note_title, "测试便签");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_import_updated_at_arbitration() {
        let dir = temp_dir();

        // 本地有一条旧数据
        let note_repo = InMemoryNoteRepository::new();
        let mut old_note = Note::new("测试".to_string(), "amber".to_string());
        old_note.update_content("旧内容".to_string());
        old_note.updated_at = "2026-07-01T00:00:00Z".to_string();
        note_repo.save(&old_note).unwrap();

        // JSON 文件中有一条更新的数据
        let mut new_note = old_note.clone();
        new_note.update_content("新内容".to_string());
        new_note.updated_at = "2026-07-02T00:00:00Z".to_string();
        let notes_dir = dir.join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        let json = serde_json::to_string_pretty(&new_note).unwrap();
        std::fs::write(notes_dir.join(format!("{}.json", new_note.id)), json).unwrap();

        // 导入：远程更新 → 覆盖本地
        let reminder_repo = InMemoryReminderRepository::new();
        let imported = import_from_json(&dir, &note_repo, &reminder_repo).unwrap();
        assert_eq!(imported, 1);

        let found = note_repo.find_by_id(&old_note.id).unwrap().unwrap();
        assert_eq!(found.content, "新内容");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_import_older_data_skipped() {
        let dir = temp_dir();

        // 本地有新数据
        let note_repo = InMemoryNoteRepository::new();
        let mut new_note = Note::new("测试".to_string(), "amber".to_string());
        new_note.update_content("新内容".to_string());
        new_note.updated_at = "2026-07-02T00:00:00Z".to_string();
        note_repo.save(&new_note).unwrap();

        // JSON 文件中是旧数据
        let mut old_note = new_note.clone();
        old_note.update_content("旧内容".to_string());
        old_note.updated_at = "2026-07-01T00:00:00Z".to_string();
        let notes_dir = dir.join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        let json = serde_json::to_string_pretty(&old_note).unwrap();
        std::fs::write(notes_dir.join(format!("{}.json", old_note.id)), json).unwrap();

        // 导入：本地更新 → 跳过
        let reminder_repo = InMemoryReminderRepository::new();
        let imported = import_from_json(&dir, &note_repo, &reminder_repo).unwrap();
        assert_eq!(imported, 0); // 本地更新，不覆盖

        let found = note_repo.find_by_id(&new_note.id).unwrap().unwrap();
        assert_eq!(found.content, "新内容");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_updated_at() {
        let json = r#"{"id":"abc","updated_at":"2026-07-13T10:00:00Z","content":"test"}"#;
        let extracted = extract_updated_at(json);
        assert_eq!(extracted, "2026-07-13T10:00:00Z");
    }

    #[test]
    fn test_extract_updated_at_not_found() {
        let json = r#"{"id":"abc","content":"test"}"#;
        let extracted = extract_updated_at(json);
        assert_eq!(extracted, "");
    }

    #[test]
    fn test_export_clears_old_files() {
        let dir = temp_dir();
        let notes_dir = dir.join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();

        // 写入一个旧 JSON 文件
        std::fs::write(notes_dir.join("old-deleted.json"), r#"{"id":"old-deleted"}"#).unwrap();

        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let note = Note::new("测试".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 导出：应清除旧文件
        export_to_json(&dir, &note_repo, &reminder_repo).unwrap();

        assert!(!notes_dir.join("old-deleted.json").exists());
        assert!(notes_dir.join(format!("{}.json", note.id)).exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
