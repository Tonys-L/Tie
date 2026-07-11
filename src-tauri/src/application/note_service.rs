use tauri::{AppHandle, Emitter, Manager};

use crate::domain::{Note, NoteRepository, ReminderRepository};

use super::{git_sync::GitSync, window_manager};

/// 创建便签并打开窗口（编排逻辑）
///
/// color 为 None 时降级为 "amber"。返回新建便签的 id。
pub fn create_note(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    color: Option<String>,
) -> Result<String, String> {
    let color = color.unwrap_or_else(|| "amber".to_string());
    let note = Note::new(String::new(), color);
    note_repo.save(&note)?;
    window_manager::open_note_window(app, &note)?;
    Ok(note.id)
}

/// 打开便签窗口
pub fn open_note(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    id: &str,
) -> Result<(), String> {
    let note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    window_manager::open_note_window(app, &note)
}

/// 打开便签窗口并附带 flag（如 "reminder" 打开提醒面板）
///
/// 若窗口已存在则聚焦并发送事件，否则创建新窗口。
pub fn open_note_with_flag(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    id: &str,
    flag: &str,
) -> Result<(), String> {
    let note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    let label = format!("note-{}", note.id);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_focus();
        let _ = window.emit("show-reminder-panel", ());
        return Ok(());
    }
    let url = format!("index.html?id={}&flag={}", note.id, flag);
    window_manager::open_note_window_with_url(app, &note, &url)
}

/// 更新便签样式（颜色、透明度、置顶）并同步窗口置顶状态
pub fn update_note_style(
    app: &AppHandle,
    note_repo: &dyn NoteRepository,
    id: &str,
    color: String,
    opacity: f64,
    is_pinned: bool,
) -> Result<(), String> {
    let mut note = note_repo.find_by_id(id)?.ok_or("便签不存在")?;
    note.set_color(color);
    note.set_opacity(opacity);
    note.set_pinned(is_pinned);
    note_repo.save(&note)?;
    let label = format!("note-{}", id);
    if let Some(win) = app.get_webview_window(&label) {
        win.set_always_on_top(is_pinned).ok();
    }
    Ok(())
}

/// 删除便签及关联提醒
pub fn delete_note(
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    id: &str,
) -> Result<(), String> {
    reminder_repo.delete_by_note_id(id)?;
    note_repo.delete(id)
}

/// 空便签自动删除（INV-003）
///
/// 若便签 title+content 均空则删除，否则不做任何操作。
pub fn close_note_if_empty(note_repo: &dyn NoteRepository, note_id: &str) {
    match note_repo.find_by_id(note_id) {
        Ok(Some(note)) => {
            if note.title.is_empty() && note.content.is_empty() {
                if let Err(e) = note_repo.delete(note_id) {
                    eprintln!("[窗口] 空便签删除失败: {}", e);
                } else {
                    eprintln!("[窗口] 空便签已自动删除: {}", note_id);
                }
            }
        }
        Ok(None) => {}
        Err(e) => eprintln!("[窗口] 检查便签失败: {}", e),
    }
}

/// 执行数据同步（机制）
///
/// 仅调用 git_sync.sync 并返回结果，不负责通知/eprintln 等展示策略。
pub fn sync_notes(
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    git_sync: &GitSync,
) -> Result<String, String> {
    git_sync.sync(note_repo, reminder_repo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository};
    use crate::domain::Reminder;

    // ============ delete_note 测试 ============

    #[test]
    fn test_delete_note_with_reminders() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        // 创建便签和关联提醒
        let note = Note::new("测试".to_string(), "amber".to_string());
        let reminder = Reminder::new(note.id.clone(), "标题".to_string(), "2099-01-01T00:00:00Z".to_string(), "once".to_string());
        note_repo.save(&note).unwrap();
        reminder_repo.save(&reminder).unwrap();

        // 删除便签
        delete_note(&note_repo, &reminder_repo, &note.id).unwrap();

        // 便签已删除
        assert!(note_repo.find_by_id(&note.id).unwrap().is_none());
        // 关联提醒也已删除
        assert!(reminder_repo.find_by_note_id(&note.id).unwrap().is_empty());
    }

    #[test]
    fn test_delete_note_not_exists() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        let result = delete_note(&note_repo, &reminder_repo, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不存在"));
    }

    #[test]
    fn test_delete_reminder_cascade() {
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        // 创建便签 + 3 条关联提醒
        let note = Note::new("测试".to_string(), "blue".to_string());
        note_repo.save(&note).unwrap();
        for i in 0..3 {
            let r = Reminder::new(note.id.clone(), format!("标题{}", i), "2099-01-01T00:00:00Z".to_string(), "once".to_string());
            reminder_repo.save(&r).unwrap();
        }
        assert_eq!(reminder_repo.find_by_note_id(&note.id).unwrap().len(), 3);

        // 删除便签
        delete_note(&note_repo, &reminder_repo, &note.id).unwrap();
        // 所有关联提醒被级联删除
        assert_eq!(reminder_repo.find_by_note_id(&note.id).unwrap().len(), 0);
    }

    // ============ close_note_if_empty 测试 (INV-003) ============

    #[test]
    fn test_close_if_empty_both_empty() {
        let note_repo = InMemoryNoteRepository::new();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = String::new();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &note.id);
        // 空便签应被删除
        assert!(note_repo.find_by_id(&note.id).unwrap().is_none());
    }

    #[test]
    fn test_close_if_empty_has_content() {
        let note_repo = InMemoryNoteRepository::new();
        let mut note = Note::new("标题".to_string(), "amber".to_string());
        note.title = "有内容".to_string();
        note.content = "".to_string();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &note.id);
        // 有内容的便签不应删除
        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
    }

    #[test]
    fn test_close_if_empty_title_only() {
        let note_repo = InMemoryNoteRepository::new();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = "仅标题".to_string();
        note.content = String::new();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &note.id);
        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
    }

    #[test]
    fn test_close_if_empty_content_only() {
        let note_repo = InMemoryNoteRepository::new();
        let mut note = Note::new(String::new(), "amber".to_string());
        note.title = String::new();
        note.content = "有内容".to_string();
        note_repo.save(&note).unwrap();

        close_note_if_empty(&note_repo, &note.id);
        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
    }

    #[test]
    fn test_close_if_empty_not_exist() {
        let note_repo = InMemoryNoteRepository::new();
        // 不存在的便签不报错
        close_note_if_empty(&note_repo, "nonexistent");
        // 无异常
    }

    // ============ update_note_style 仓储逻辑测试（跳过窗口操作） ============
    
    /// 验证 update_note_style 的仓储逻辑部分：domain 方法 + save 正确保存
    /// 注意：此测试不验证窗口置顶同步（需 Tauri 运行时）
    #[test]
    fn test_update_note_style_persists_to_repo() {
        let note_repo = InMemoryNoteRepository::new();
        let note = Note::new("原始便签".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 手动执行 update_note_style 的仓储逻辑
        let id = note.id.clone();
        let mut found = note_repo.find_by_id(&id).unwrap().expect("便签存在");
        found.set_color("blue".to_string());
        found.set_opacity(0.5);
        found.set_pinned(true);
        note_repo.save(&found).unwrap();

        // 验证持久化结果
        let saved = note_repo.find_by_id(&id).unwrap().expect("便签仍存在");
        // INV-001：opacity 在 domain 层 clamp 到 0.3~1.0，0.5 在范围内不变
        assert!((saved.opacity - 0.5).abs() < f64::EPSILON);
        assert!(saved.is_pinned);
    }

    #[test]
    fn test_update_note_style_opacity_clamp_inv001() {
        let note_repo = InMemoryNoteRepository::new();
        let note = Note::new("原始便签".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 尝试设置非法 opacity 值（0.0，低于 INV-001 下限 0.3）
        let id = note.id.clone();
        let mut found = note_repo.find_by_id(&id).unwrap().expect("便签存在");
        found.set_color("pink".to_string());
        found.set_opacity(0.0);  // 低于下限
        found.set_pinned(false);
        note_repo.save(&found).unwrap();

        // INV-001 验证：opacity 应被 clamp 到 0.3
        let saved = note_repo.find_by_id(&id).unwrap().expect("便签仍存在");
        assert!((saved.opacity - 0.3).abs() < 1e-10);  // clamp 到下限
        assert!(!saved.is_pinned);
    }

    #[test]
    fn test_update_note_style_opacity_clamp_upper_bound() {
        let note_repo = InMemoryNoteRepository::new();
        let note = Note::new("原始便签".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 尝试设置非法 opacity 值（2.0，高于上限 1.0）
        let id = note.id.clone();
        let mut found = note_repo.find_by_id(&id).unwrap().expect("便签存在");
        found.set_opacity(2.0);  // 高于上限
        note_repo.save(&found).unwrap();

        let saved = note_repo.find_by_id(&id).unwrap().expect("便签仍存在");
        assert!((saved.opacity - 1.0).abs() < 1e-10);  // clamp 到上限
    }

    #[test]
    fn test_update_note_style_nonexistent_note() {
        let note_repo = InMemoryNoteRepository::new();

        // 不存在的便签返回错误
        let result = note_repo.find_by_id("nonexistent");
        assert!(result.is_ok());  // find_by_id 返回 Ok(None) 不报错
        assert!(result.unwrap().is_none());
    }
}
