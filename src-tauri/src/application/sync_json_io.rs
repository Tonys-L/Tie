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
        if let Some(start) = rest.find('"') {
            let after_key = &rest[start + 1..];
            if let Some(val_start) = after_key.find('"') {
                let val_rest = &after_key[val_start + 1..];
                if let Some(val_end) = val_rest.find('"') {
                    return val_rest[..val_end].to_string();
                }
            }
        }
    }
    String::new()
}
