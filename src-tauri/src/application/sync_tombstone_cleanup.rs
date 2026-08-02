//! 墓碑清理模块（INV-032）
//!
//! 跨 note/reminder/template 三类合计计算墓碑数量，超过阈值时按 deleted_at 降序
//! 物理删除最老的墓碑，避免墓碑无限增长。
//! 仅供 git_sync::sync 在 import 完成后调用。

use crate::domain::{NoteRepository, ReminderRepository, TemplateRepository};

/// 墓碑清理阈值（跨三类合计）
pub const TOMBSTONE_THRESHOLD: usize = 50;

/// 清理超阈值墓碑，返回清理数量
///
/// 流程：
/// 1. 查询三类实体的 find_all_including_deleted
/// 2. 过滤出墓碑（is_deleted() == true）
/// 3. 合并为统一列表（用枚举标记类型 + deleted_at 排序）
/// 4. 若总数 <= 阈值，返回 0
/// 5. 若总数 > 阈值，按 deleted_at 升序（最老在前）取前 (总数 - 阈值) 条，调 physical_delete
pub fn cleanup_old_tombstones(
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    template_repo: &dyn TemplateRepository,
) -> Result<usize, String> {
    cleanup_old_tombstones_with_threshold(
        note_repo,
        reminder_repo,
        template_repo,
        TOMBSTONE_THRESHOLD,
    )
}

/// 带自定义阈值的清理函数（便于测试）
pub fn cleanup_old_tombstones_with_threshold(
    note_repo: &dyn NoteRepository,
    reminder_repo: &dyn ReminderRepository,
    template_repo: &dyn TemplateRepository,
    threshold: usize,
) -> Result<usize, String> {
    // 收集所有墓碑
    let mut tombstones: Vec<TombstoneEntry> = Vec::new();

    for note in note_repo
        .find_all_including_deleted()?
        .into_iter()
        .filter(|n| n.is_deleted())
    {
        tombstones.push(TombstoneEntry {
            id: note.id,
            deleted_at: note.deleted_at.unwrap_or_default(),
            kind: EntityKind::Note,
        });
    }

    for reminder in reminder_repo
        .find_all_including_deleted()?
        .into_iter()
        .filter(|r| r.is_deleted())
    {
        tombstones.push(TombstoneEntry {
            id: reminder.id,
            deleted_at: reminder.deleted_at.unwrap_or_default(),
            kind: EntityKind::Reminder,
        });
    }

    for template in template_repo
        .find_all_including_deleted()?
        .into_iter()
        .filter(|t| t.is_deleted())
    {
        tombstones.push(TombstoneEntry {
            id: template.id,
            deleted_at: template.deleted_at.unwrap_or_default(),
            kind: EntityKind::Template,
        });
    }

    // 未超阈值不清理
    if tombstones.len() <= threshold {
        return Ok(0);
    }

    // 按 deleted_at 升序排序（最老在前）
    tombstones.sort_by(|a, b| a.deleted_at.cmp(&b.deleted_at));

    // 取最老的 (总数 - 阈值) 条物理删除
    let to_cleanup = tombstones.len() - threshold;
    for entry in tombstones.iter().take(to_cleanup) {
        match entry.kind {
            EntityKind::Note => note_repo.physical_delete(&entry.id)?,
            EntityKind::Reminder => reminder_repo.physical_delete(&entry.id)?,
            EntityKind::Template => template_repo.physical_delete(&entry.id)?,
        }
    }

    Ok(to_cleanup)
}

/// 墓碑条目（用于统一排序和清理）
struct TombstoneEntry {
    id: String,
    deleted_at: String,
    kind: EntityKind,
}

/// 实体类型标记
enum EntityKind {
    Note,
    Reminder,
    Template,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::mock_repo::{
        InMemoryNoteRepository, InMemoryReminderRepository, InMemoryTemplateRepository,
    };
    use crate::domain::{Note, Reminder, Template};
    use chrono::NaiveDate;

    /// 创建 note 墓碑，deleted_at 由参数指定（便于控制排序）
    fn make_note_tombstone(id: &str, deleted_at: &str) -> Note {
        let mut note = Note::new(format!("note-{}", id), "amber".to_string());
        note.id = id.to_string();
        note.delete();
        // 覆盖时间戳以便测试控制排序
        note.deleted_at = Some(deleted_at.to_string());
        note.updated_at = deleted_at.to_string();
        note
    }

    /// 创建 reminder 墓碑
    fn make_reminder_tombstone(id: &str, deleted_at: &str) -> Reminder {
        let mut r = Reminder::new(
            "note-x".to_string(),
            format!("r-{}", id),
            "2026-01-01T00:00:00Z".to_string(),
            "once".to_string(),
        );
        r.id = id.to_string();
        r.delete();
        r.deleted_at = Some(deleted_at.to_string());
        r.updated_at = deleted_at.to_string();
        r
    }

    /// 创建 template 墓碑
    fn make_template_tombstone(id: &str, deleted_at: &str) -> Template {
        let mut t = Template::new(format!("tpl-{}", id), "content".to_string());
        t.id = id.to_string();
        t.delete();
        t.deleted_at = Some(deleted_at.to_string());
        t.updated_at = deleted_at.to_string();
        t
    }

    /// 从起始日期生成 count 个连续日期（ISO 8601 格式，便于字符串排序）
    fn generate_dates(start: &str, count: usize) -> Vec<String> {
        let base = NaiveDate::parse_from_str(start, "%Y-%m-%d").unwrap();
        (0..count)
            .map(|i| {
                let d = base + chrono::Duration::days(i as i64);
                format!("{}T00:00:00Z", d.format("%Y-%m-%d"))
            })
            .collect()
    }

    /// 统计三类 repo 中剩余墓碑总数
    fn count_all_tombstones(
        note_repo: &InMemoryNoteRepository,
        reminder_repo: &InMemoryReminderRepository,
        template_repo: &InMemoryTemplateRepository,
    ) -> usize {
        let n = note_repo
            .find_all_including_deleted()
            .unwrap()
            .iter()
            .filter(|n| n.is_deleted())
            .count();
        let r = reminder_repo
            .find_all_including_deleted()
            .unwrap()
            .iter()
            .filter(|r| r.is_deleted())
            .count();
        let t = template_repo
            .find_all_including_deleted()
            .unwrap()
            .iter()
            .filter(|t| t.is_deleted())
            .count();
        n + r + t
    }

    #[test]
    fn under_threshold_no_cleanup() {
        // 30 条墓碑，阈值 50 → 不清理
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        let dates = generate_dates("2026-01-01", 30);
        for (i, dt) in dates.iter().enumerate() {
            let id = format!("n{}", i);
            let note = make_note_tombstone(&id, dt);
            note_repo.save(&note).unwrap();
        }

        let cleaned = cleanup_old_tombstones_with_threshold(
            &note_repo,
            &reminder_repo,
            &template_repo,
            50,
        )
        .unwrap();

        assert_eq!(cleaned, 0);
        assert_eq!(count_all_tombstones(&note_repo, &reminder_repo, &template_repo), 30);
    }

    #[test]
    fn over_threshold_deletes_oldest() {
        // 60 条 note 墓碑（deleted_at 从 2026-01-01 起 60 天），阈值 50 → 清理 10 条最老的
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        let dates = generate_dates("2026-01-01", 60);
        for (i, dt) in dates.iter().enumerate() {
            let id = format!("n{}", i);
            let note = make_note_tombstone(&id, dt);
            note_repo.save(&note).unwrap();
        }

        let cleaned = cleanup_old_tombstones_with_threshold(
            &note_repo,
            &reminder_repo,
            &template_repo,
            50,
        )
        .unwrap();

        assert_eq!(cleaned, 10);
        // 剩余 50 条墓碑
        assert_eq!(count_all_tombstones(&note_repo, &reminder_repo, &template_repo), 50);

        // 最老的 10 条（n0..n9，对应 2026-01-01 ~ 2026-01-10）应被物理删除
        for i in 0..10 {
            let id = format!("n{}", i);
            assert!(
                note_repo.find_by_id_including_deleted(&id).unwrap().is_none(),
                "最老的墓碑 {} 应被物理删除",
                id
            );
        }
        // 最新 50 条（n10..n59）应保留
        for i in 10..60 {
            let id = format!("n{}", i);
            assert!(
                note_repo.find_by_id_including_deleted(&id).unwrap().is_some(),
                "较新的墓碑 {} 应保留",
                id
            );
        }
    }

    #[test]
    fn mixed_three_types_aggregated() {
        // 20 note + 20 reminder + 20 template = 60 条，阈值 50 → 清理 10 条最老的（跨三类合计）
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 三类各 20 条，deleted_at 交错分布：
        // note:     2026-01-01 ~ 2026-01-20 (day 0..19)
        // reminder: 2026-01-11 ~ 2026-01-30 (day 10..29)
        // template: 2026-01-21 ~ 2026-02-09 (day 20..39)
        let note_dates = generate_dates("2026-01-01", 20);
        let reminder_dates = generate_dates("2026-01-11", 20);
        let template_dates = generate_dates("2026-01-21", 20);

        for (i, dt) in note_dates.iter().enumerate() {
            note_repo
                .save(&make_note_tombstone(&format!("n{}", i), dt))
                .unwrap();
        }
        for (i, dt) in reminder_dates.iter().enumerate() {
            reminder_repo
                .save(&make_reminder_tombstone(&format!("r{}", i), dt))
                .unwrap();
        }
        for (i, dt) in template_dates.iter().enumerate() {
            template_repo
                .save(&make_template_tombstone(&format!("t{}", i), dt))
                .unwrap();
        }

        let cleaned = cleanup_old_tombstones_with_threshold(
            &note_repo,
            &reminder_repo,
            &template_repo,
            50,
        )
        .unwrap();

        assert_eq!(cleaned, 10);
        assert_eq!(count_all_tombstones(&note_repo, &reminder_repo, &template_repo), 50);

        // 最老的 10 条对应 2026-01-01 ~ 2026-01-10（均为 note，day 0..9）
        for i in 0..10 {
            assert!(
                note_repo
                    .find_by_id_including_deleted(&format!("n{}", i))
                    .unwrap()
                    .is_none(),
                "note n{} (day {}) 应被清理",
                i,
                i
            );
        }
        // note n10..n19 保留
        for i in 10..20 {
            assert!(
                note_repo
                    .find_by_id_including_deleted(&format!("n{}", i))
                    .unwrap()
                    .is_some(),
                "note n{} 应保留",
                i
            );
        }
    }

    #[test]
    fn no_tombstones_returns_zero() {
        // 0 条墓碑，阈值 50 → 返回 0
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        // 放一些活跃实体（非墓碑），不应被清理
        let note = Note::new("活跃".to_string(), "amber".to_string());
        note_repo.save(&note).unwrap();

        let cleaned = cleanup_old_tombstones_with_threshold(
            &note_repo,
            &reminder_repo,
            &template_repo,
            50,
        )
        .unwrap();

        assert_eq!(cleaned, 0);
        assert_eq!(count_all_tombstones(&note_repo, &reminder_repo, &template_repo), 0);
        // 活跃实体仍在
        assert!(note_repo.find_by_id(&note.id).unwrap().is_some());
    }

    #[test]
    fn exactly_at_threshold_no_cleanup() {
        // 50 条墓碑，阈值 50 → 等于阈值不清理
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        let dates = generate_dates("2026-01-01", 50);
        for (i, dt) in dates.iter().enumerate() {
            note_repo
                .save(&make_note_tombstone(&format!("n{}", i), dt))
                .unwrap();
        }

        let cleaned = cleanup_old_tombstones_with_threshold(
            &note_repo,
            &reminder_repo,
            &template_repo,
            50,
        )
        .unwrap();

        assert_eq!(cleaned, 0);
        assert_eq!(count_all_tombstones(&note_repo, &reminder_repo, &template_repo), 50);
    }

    #[test]
    fn cleanup_preserves_newest() {
        // 55 条墓碑（deleted_at 从 2026-01-01 起 55 天），阈值 50 → 清理 5 条最老的，保留最新 50 条
        let note_repo = InMemoryNoteRepository::new();
        let reminder_repo = InMemoryReminderRepository::new();
        let template_repo = InMemoryTemplateRepository::new();

        let dates = generate_dates("2026-01-01", 55);
        for (i, dt) in dates.iter().enumerate() {
            note_repo
                .save(&make_note_tombstone(&format!("n{}", i), dt))
                .unwrap();
        }

        let cleaned = cleanup_old_tombstones_with_threshold(
            &note_repo,
            &reminder_repo,
            &template_repo,
            50,
        )
        .unwrap();

        assert_eq!(cleaned, 5);
        assert_eq!(count_all_tombstones(&note_repo, &reminder_repo, &template_repo), 50);

        // 最老 5 条（n0..n4，2026-01-01 ~ 2026-01-05）被删除
        for i in 0..5 {
            assert!(
                note_repo
                    .find_by_id_including_deleted(&format!("n{}", i))
                    .unwrap()
                    .is_none(),
                "最老的墓碑 n{} 应被清理",
                i
            );
        }
        // 最新 50 条（n5..n54）保留
        for i in 5..55 {
            assert!(
                note_repo
                    .find_by_id_including_deleted(&format!("n{}", i))
                    .unwrap()
                    .is_some(),
                "较新的墓碑 n{} 应保留",
                i
            );
        }
    }
}
