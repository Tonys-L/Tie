use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::domain::{Note, NoteRepository, Reminder, ReminderRepository, Template, TemplateRepository};

/// 导出所有便签、提醒和模板为 JSON 文件（含墓碑）
///
/// 内部委托 `export_entity_to_json` 泛型函数处理"查询→清空→序列化→写文件"的通用流程。
///
/// **墓碑机制（INV-032，LES-028 修复）**：使用 `find_all_including_deleted` 查询全部记录
/// （含墓碑和归档），让墓碑 JSON 写出到 sync 目录，其他设备 import 时能感知删除。
/// Note 的 `find_all_including_deleted` 返回活跃 + 归档 + 墓碑，不再需要单独查询 `find_archived`。
/// 墓碑 note 的 `deleted_at` 字段用 `#[serde(skip_serializing_if = "Option::is_none")]`，
/// 所以非墓碑 note 的 JSON 不含 `deleted_at` 字段，墓碑 note 的 JSON 含 `deleted_at` 字段。
pub fn export_to_json(
    sync_dir: &Path,
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    template_repo: &dyn TemplateRepository,
) -> Result<(), String> {
    // 导出便签（含墓碑和归档，find_all_including_deleted 返回全部）
    let all_notes = note_repo
        .find_all_including_deleted()
        .map_err(|e| format!("查询便签失败: {}", e))?;
    export_entity_to_json(&sync_dir.join("notes"), &all_notes, "便签", |n| n.id.as_str())?;

    // 导出提醒（含墓碑）
    let reminders = reminder_repo
        .find_all_including_deleted()
        .map_err(|e| format!("查询提醒失败: {}", e))?;
    export_entity_to_json(&sync_dir.join("reminders"), &reminders, "提醒", |r| r.id.as_str())?;

    // 导出模板（含墓碑）
    let templates = template_repo
        .find_all_including_deleted()
        .map_err(|e| format!("查询模板失败: {}", e))?;
    export_entity_to_json(&sync_dir.join("templates"), &templates, "模板", |t| t.id.as_str())?;

    Ok(())
}

/// 从 JSON 文件导入到数据库（upsert，按 updated_at 取最新，含墓碑）
///
/// 内部委托 `import_entity_from_json` 泛型函数处理"遍历目录→解析→仲裁→save"的通用流程。
/// 仲裁规则：`item.updated_at > existing.updated_at` 时覆盖（last-write-wins，INV-011）。
///
/// **墓碑机制（INV-032，LES-028 修复）**：使用 `find_by_id_including_deleted` 查询本地记录，
/// 让墓碑的 `updated_at`（= 删除时间）参与 last-write-wins 仲裁。这样：
/// - 远程墓碑 updated_at 更新 → 覆盖本地非墓碑（传播删除）
/// - 远程非墓碑 updated_at 更新 → 覆盖本地墓碑（复活）
/// - 本地墓碑/非墓碑 updated_at 更新 → 跳过（保留本地）
/// Note/Reminder/Template 的 `delete()` 同时设 `deleted_at` 和 `updated_at` 为 now，
/// 所以仲裁只需比较 `updated_at`，无需 import 改仲裁核心逻辑。
pub fn import_from_json(
    sync_dir: &Path,
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    template_repo: &dyn TemplateRepository,
) -> Result<usize, String> {
    let mut imported = 0;

    imported += import_entity_from_json::<Note, dyn NoteRepository>(
        &sync_dir.join("notes"),
        note_repo,
        "便签",
        |n| n.id.as_str(),
        |n| n.updated_at.as_str(),
        |repo, id| repo.find_by_id_including_deleted(id),
        |repo, item| repo.save(item),
    )?;

    imported += import_entity_from_json::<Reminder, dyn ReminderRepository>(
        &sync_dir.join("reminders"),
        reminder_repo,
        "提醒",
        |r| r.id.as_str(),
        |r| r.updated_at.as_str(),
        |repo, id| repo.find_by_id_including_deleted(id),
        |repo, item| repo.save(item),
    )?;

    imported += import_entity_from_json::<Template, dyn TemplateRepository>(
        &sync_dir.join("templates"),
        template_repo,
        "模板",
        |t| t.id.as_str(),
        |t| t.updated_at.as_str(),
        |repo, id| repo.find_by_id_including_deleted(id),
        |repo, item| repo.save(item),
    )?;

    Ok(imported)
}

/// 泛型导出：把实体列表序列化为 JSON 文件写入目录
///
/// 流程：创建目录 → 清空旧 JSON → 逐个序列化 + 写文件。
/// `get_id` 闭包用于生成文件名（`{id}.json`）。
fn export_entity_to_json<T: Serialize>(
    dir: &Path,
    items: &[T],
    entity_name: &str,
    get_id: impl Fn(&T) -> &str,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}", e))?;
    clear_dir_json(dir)?;
    for item in items {
        let json = serde_json::to_string_pretty(item)
            .map_err(|e| format!("序列化{}失败: {}", entity_name, e))?;
        let path = dir.join(format!("{}.json", get_id(item)));
        std::fs::write(&path, json).map_err(|e| format!("写入文件失败: {}", e))?;
    }
    Ok(())
}

/// 泛型导入：从目录读取 JSON 文件，反序列化后按 last-write-wins 仲裁 upsert
///
/// 流程：目录不存在返回 0 → 遍历 .json 文件 → 反序列化 → find_by_id 仲裁 → save。
/// 仲裁：`item.updated_at > existing.updated_at` 时覆盖；本地不存在时直接 save。
///
/// 泛型 + 闭包参数化差异点（id/updated_at/find_by_id/save），让三种实体的导入逻辑统一为一处。
fn import_entity_from_json<T, R: ?Sized>(
    dir: &Path,
    repo: &R,
    entity_name: &str,
    get_id: impl Fn(&T) -> &str,
    get_updated_at: impl Fn(&T) -> &str,
    find_by_id: impl Fn(&R, &str) -> Result<Option<T>, String>,
    save: impl Fn(&R, &T) -> Result<(), String>,
) -> Result<usize, String>
where
    T: DeserializeOwned,
{
    if !dir.exists() {
        return Ok(0);
    }
    let mut imported = 0;
    for entry in std::fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))? {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let content = std::fs::read_to_string(&path).map_err(|e| format!("读取文件失败: {}", e))?;
        let item: T = serde_json::from_str(&content)
            .map_err(|e| format!("解析{}失败: {}", entity_name, e))?;

        let should_save = match find_by_id(repo, get_id(&item))? {
            Some(existing) => get_updated_at(&item) > get_updated_at(&existing),
            None => true,
        };

        if should_save {
            save(repo, &item)?;
            imported += 1;
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
///
/// 用 `serde_json::Value` 解析后取字段，避免手写 `find("\"updated_at\"")` 切片
/// 在字段名出现在字符串值内（如 `content` 含 `"updated_at"` 子串）时误匹配。
/// 此函数喂给 `git_ops::resolve_json_conflict`（INV-011 last-write-wins），
/// 误匹配会导致冲突仲裁方向错误 → 数据丢失。
///
/// 解析失败或字段缺失时返回空字符串（保持旧行为，git_ops 兜底取 ours 版本）。
pub fn extract_updated_at(json: &str) -> String {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    value
        .get("updated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mock_repo::{InMemoryNoteRepository, InMemoryReminderRepository, InMemoryTemplateRepository};
    use crate::domain::{Note, Reminder, Template};

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
        let template_repo = InMemoryTemplateRepository::new();

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

        let template = Template::new("会议记录".to_string(), "## 会议记录".to_string());
        template_repo.save(&template).unwrap();

        // 导出
        export_to_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();

        // 导入到新仓储
        let note_repo2 = InMemoryNoteRepository::new();
        let reminder_repo2 = InMemoryReminderRepository::new();
        let template_repo2 = InMemoryTemplateRepository::new();
        let imported = import_from_json(&dir, &note_repo2, &reminder_repo2, &template_repo2).unwrap();

        assert_eq!(imported, 3); // 1 note + 1 reminder + 1 template

        // 验证便签
        let found_note = note_repo2.find_by_id(&note.id).unwrap();
        assert!(found_note.is_some());
        assert_eq!(found_note.unwrap().color.as_str(), "amber");

        // 验证提醒
        let found_reminder = reminder_repo2.find_by_id(&reminder.id).unwrap();
        assert!(found_reminder.is_some());
        assert_eq!(found_reminder.unwrap().note_title, "测试便签");

        // 验证模板
        let found_template = template_repo2.find_by_id(&template.id).unwrap();
        assert!(found_template.is_some());
        assert_eq!(found_template.unwrap().name, "会议记录");

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
        let template_repo = InMemoryTemplateRepository::new();
        let imported = import_from_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();
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
        let template_repo = InMemoryTemplateRepository::new();
        let imported = import_from_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();
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

    /// 字段名 "updated_at" 出现在字符串值内时，旧手写切片会误匹配，
    /// 导致 git_ops 冲突仲裁取错版本（INV-011 数据完整性风险）。
    #[test]
    fn test_extract_updated_at_field_name_in_string_value() {
        // content 字段值含 "updated_at" 子串，真正的 updated_at 在最后
        let json = r#"{"id":"abc","content":"the updated_at field is tricky","updated_at":"2026-07-13T10:00:00Z"}"#;
        let extracted = extract_updated_at(json);
        assert_eq!(extracted, "2026-07-13T10:00:00Z");
    }

    /// 非法 JSON 返回空字符串（git_ops 兜底取 ours 版本）
    #[test]
    fn test_extract_updated_at_invalid_json() {
        let json = r#"not a json"#;
        let extracted = extract_updated_at(json);
        assert_eq!(extracted, "");
    }

    /// updated_at 字段类型非字符串时返回空（防御性）
    #[test]
    fn test_extract_updated_at_non_string_field() {
        let json = r#"{"id":"abc","updated_at":123}"#;
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
        let template_repo = InMemoryTemplateRepository::new();
        let note = Note::new("测试".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        // 导出：应清除旧文件
        export_to_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();

        assert!(!notes_dir.join("old-deleted.json").exists());
        assert!(notes_dir.join(format!("{}.json", note.id)).exists());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_template_sync_roundtrip() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 准备模板数据
        let tpl1 = Template::new("会议记录".to_string(), "## 会议".to_string());
        let tpl2 = Template::new("待办".to_string(), "- [ ] ".to_string());
        template_repo.save(&tpl1).unwrap();
        template_repo.save(&tpl2).unwrap();

        // 导出
        export_to_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();

        // 导入到新仓储
        let template_repo2 = InMemoryTemplateRepository::new();
        let note_repo2 = InMemoryNoteRepository::new();
        let reminder_repo2 = InMemoryReminderRepository::new();
        let imported = import_from_json(&dir, &note_repo2, &reminder_repo2, &template_repo2).unwrap();
        assert_eq!(imported, 2);

        // 验证模板内容
        let all = template_repo2.find_all().unwrap();
        assert_eq!(all.len(), 2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_template_import_arbitration() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();

        // 本地有旧模板
        let template_repo = InMemoryTemplateRepository::new();
        let mut old_tpl = Template::new("会议".to_string(), "旧内容".to_string());
        old_tpl.updated_at = "2026-07-01T00:00:00Z".to_string();
        template_repo.save(&old_tpl).unwrap();

        // 远程有新模板
        let mut new_tpl = old_tpl.clone();
        new_tpl.update_content("会议".to_string(), "新内容".to_string());
        new_tpl.updated_at = "2026-07-02T00:00:00Z".to_string();
        let templates_dir = dir.join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        let json = serde_json::to_string_pretty(&new_tpl).unwrap();
        std::fs::write(templates_dir.join(format!("{}.json", new_tpl.id)), json).unwrap();

        // 导入：远程更新 → 覆盖
        let imported = import_from_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();
        assert_eq!(imported, 1);

        let found = template_repo.find_by_id(&old_tpl.id).unwrap().unwrap();
        assert_eq!(found.content, "新内容");

        std::fs::remove_dir_all(&dir).ok();
    }

    // ============ 墓碑机制测试 (INV-032 墓碑 / INV-011 last-write-wins 含墓碑) ============
    // 以下 5 个测试覆盖 LES-028 修复：让墓碑参与 import 仲裁 + export 写出墓碑 JSON。

    /// import 传播墓碑：远程墓碑 updated_at 更新 → 覆盖本地非墓碑（LES-028 修复）
    ///
    /// 验证点：本地非墓碑被远程墓碑覆盖后，find_by_id 返回 None（过滤墓碑），
    /// find_by_id_including_deleted 返回 Some 且 deleted_at.is_some()。
    #[test]
    fn import_propagates_tombstone() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 本地有非墓碑 note（updated_at = "2026-07-01T00:00:00Z"）
        let mut local_note = Note::new("本地便签".to_string(), "amber".to_string());
        local_note.id = "n1".to_string();
        local_note.updated_at = "2026-07-01T00:00:00Z".to_string();
        note_repo.save(&local_note).unwrap();

        // 远程 JSON 是墓碑 note（updated_at = "2026-07-02T00:00:00Z", deleted_at = "2026-07-02T00:00:00Z"）
        let mut remote_note = local_note.clone();
        remote_note.deleted_at = Some("2026-07-02T00:00:00Z".to_string());
        remote_note.updated_at = "2026-07-02T00:00:00Z".to_string();
        let notes_dir = dir.join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        let json = serde_json::to_string_pretty(&remote_note).unwrap();
        std::fs::write(notes_dir.join("n1.json"), json).unwrap();

        // import：远程墓碑 updated_at 更新 → 覆盖本地
        let imported = import_from_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();
        assert_eq!(imported, 1);

        // 验证本地 note 变为墓碑
        assert!(
            note_repo.find_by_id("n1").unwrap().is_none(),
            "find_by_id 应过滤墓碑返回 None"
        );
        let with_tomb = note_repo
            .find_by_id_including_deleted("n1")
            .unwrap()
            .expect("墓碑应存在");
        assert!(with_tomb.deleted_at.is_some(), "deleted_at 应为 Some");
        assert_eq!(with_tomb.deleted_at.as_deref(), Some("2026-07-02T00:00:00Z"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// import 本地更新于远程墓碑 → 本地非墓碑保留（删除不传播，相当于撤销）
    ///
    /// 验证点：本地 updated_at 比远程墓碑新 → 跳过 save → 本地仍为非墓碑。
    #[test]
    fn import_local_newer_than_remote_tombstone_kept() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 本地有非墓碑 note（updated_at = "2026-07-03T00:00:00Z"，比远程墓碑新）
        let mut local_note = Note::new("本地便签".to_string(), "amber".to_string());
        local_note.id = "n1".to_string();
        local_note.updated_at = "2026-07-03T00:00:00Z".to_string();
        note_repo.save(&local_note).unwrap();

        // 远程 JSON 是墓碑 note（updated_at = "2026-07-02T00:00:00Z"）
        let mut remote_note = local_note.clone();
        remote_note.deleted_at = Some("2026-07-02T00:00:00Z".to_string());
        remote_note.updated_at = "2026-07-02T00:00:00Z".to_string();
        let notes_dir = dir.join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        let json = serde_json::to_string_pretty(&remote_note).unwrap();
        std::fs::write(notes_dir.join("n1.json"), json).unwrap();

        // import：本地更新 → 跳过
        let imported = import_from_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();
        assert_eq!(imported, 0);

        // 验证本地 note 仍为非墓碑
        let found = note_repo
            .find_by_id("n1")
            .unwrap()
            .expect("note 应存在");
        assert!(
            found.deleted_at.is_none(),
            "deleted_at 应为 None（本地非墓碑保留）"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// import 远程非墓碑更新 → 复活本地墓碑（LES-028 修复）
    ///
    /// 验证点：本地墓碑被远程非墓碑覆盖后，find_by_id 返回 Some 且 deleted_at.is_none()。
    #[test]
    fn import_revives_when_remote_newer() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 本地有墓碑 note（updated_at = "2026-07-01T00:00:00Z", deleted_at = "2026-07-01T00:00:00Z"）
        let mut local_note = Note::new("被删便签".to_string(), "amber".to_string());
        local_note.id = "n1".to_string();
        local_note.deleted_at = Some("2026-07-01T00:00:00Z".to_string());
        local_note.updated_at = "2026-07-01T00:00:00Z".to_string();
        note_repo.save(&local_note).unwrap();

        // 远程 JSON 是非墓碑 note（updated_at = "2026-07-02T00:00:00Z", deleted_at = None）
        let mut remote_note = local_note.clone();
        remote_note.deleted_at = None;
        remote_note.updated_at = "2026-07-02T00:00:00Z".to_string();
        let notes_dir = dir.join("notes");
        std::fs::create_dir_all(&notes_dir).unwrap();
        let json = serde_json::to_string_pretty(&remote_note).unwrap();
        std::fs::write(notes_dir.join("n1.json"), json).unwrap();

        // import：远程非墓碑更新 → 复活本地墓碑
        let imported = import_from_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();
        assert_eq!(imported, 1);

        // 验证本地 note 复活
        let found = note_repo
            .find_by_id("n1")
            .unwrap()
            .expect("复活后 find_by_id 应返回 Some");
        assert!(found.deleted_at.is_none(), "deleted_at 应为 None（已复活）");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// export 写出墓碑 JSON：DB 中墓碑 note 的 JSON 文件含 deleted_at 字段且非 null（LES-028 修复）
    ///
    /// 验证点：调 note.delete() + save 后 export，sync/notes/{id}.json 含 deleted_at 字段。
    #[test]
    fn export_writes_tombstone_json() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // DB 中有墓碑 note（先 delete 再 save）
        let mut note = Note::new("将被删除".to_string(), "amber".to_string());
        note.id = "n1".to_string();
        note.delete();
        note_repo.save(&note).unwrap();

        // export
        export_to_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();

        // 读取 sync/notes/n1.json 验证含 deleted_at 字段且不为 null
        let json_path = dir.join("notes").join("n1.json");
        assert!(json_path.exists(), "墓碑 note 的 JSON 文件应存在");
        let content = std::fs::read_to_string(&json_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&content).unwrap();
        let deleted_at = value.get("deleted_at").expect("JSON 应含 deleted_at 字段");
        assert!(!deleted_at.is_null(), "deleted_at 不应为 null");
        assert!(deleted_at.as_str().is_some(), "deleted_at 应为字符串");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// export 包含墓碑：1 活跃 + 1 墓碑 → sync/notes 下 2 个 JSON 文件（LES-028 修复）
    ///
    /// 验证点：墓碑 note 的 JSON 文件存在（不被 find_all_including_deleted 过滤）。
    #[test]
    fn export_includes_tombstones_in_json() {
        let dir = temp_dir();
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 1 条活跃 note
        let mut active = Note::new("活跃便签".to_string(), "amber".to_string());
        active.id = "n-active".to_string();
        note_repo.save(&active).unwrap();

        // 1 条墓碑 note
        let mut tombstone = Note::new("墓碑便签".to_string(), "blue".to_string());
        tombstone.id = "n-tomb".to_string();
        tombstone.delete();
        note_repo.save(&tombstone).unwrap();

        // export
        export_to_json(&dir, &note_repo, &reminder_repo, &template_repo).unwrap();

        // sync/notes 下应有 2 个 JSON 文件（含墓碑）
        let notes_dir = dir.join("notes");
        let active_path = notes_dir.join("n-active.json");
        let tomb_path = notes_dir.join("n-tomb.json");
        assert!(active_path.exists(), "活跃 note JSON 应存在");
        assert!(tomb_path.exists(), "墓碑 note JSON 应存在");

        std::fs::remove_dir_all(&dir).ok();
    }
}
