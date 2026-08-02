# Tasks

按 TDD（测试驱动开发）顺序排列：先写测试 → 写实现 → 验证 → 重构。

## 阶段一：领域模型墓碑字段（TDD）

- [x] Task 1: Note 模型加 `deleted_at` 字段和 `delete()`/`is_deleted()` 方法
  - [x] SubTask 1.1: 先写测试 `note::tests::delete_sets_deleted_at_and_updated_at` 验证 delete 后 `deleted_at.is_some()` 且 `updated_at == deleted_at`
  - [x] SubTask 1.2: 先写测试 `note::tests::is_deleted_reflects_deleted_at` 验证 is_deleted 在 delete 前后返回值
  - [x] SubTask 1.3: 实现 `deleted_at: Option<String>` 字段（`#[serde(default, skip_serializing_if = "Option::is_none")]`）+ `delete()` + `is_deleted()` 方法
  - [x] SubTask 1.4: 运行 `cargo test note::tests` 验证通过

- [x] Task 2: Reminder 模型加 `deleted_at` 字段和 `delete()`/`is_deleted()` 方法（同 Task 1 模式）
  - [x] SubTask 2.1: 先写测试 `reminder::tests::delete_sets_deleted_at_and_updated_at`
  - [x] SubTask 2.2: 实现 `deleted_at` 字段 + `delete()` + `is_deleted()` 方法
  - [x] SubTask 2.3: 运行 `cargo test reminder::tests` 验证通过

- [x] Task 3: Template 模型加 `deleted_at` 字段和 `delete()`/`is_deleted()` 方法（同 Task 1 模式）
  - [x] SubTask 3.1: 先写测试 `template::tests::delete_sets_deleted_at_and_updated_at`
  - [x] SubTask 3.2: 实现 `deleted_at` 字段 + `delete()` + `is_deleted()` 方法
  - [x] SubTask 3.3: 运行 `cargo test template::tests` 验证通过

## 阶段二：Repository trait 扩展

- [x] Task 4: 3 个 Repository trait 新增 `find_all_including_deleted` 和 `physical_delete` 方法
  - [x] SubTask 4.1: `repositories.rs` 中 NoteRepository/ReminderRepository/TemplateRepository trait 新增方法签名（含 `find_by_id_including_deleted` 共 3 个新方法）
  - [x] SubTask 4.2: `cargo build` 验证编译错误指向未实现的 trait 方法

## 阶段三：Mock repo 实现（TDD）

- [x] Task 5: mock_repo 实现 3 类实体的新方法 + 查询过滤墓碑
  - [x] SubTask 5.1: 先写测试 `mock_repo::tests::find_all_excludes_tombstones` 验证 find_all 不返回墓碑
  - [x] SubTask 5.2: 先写测试 `mock_repo::tests::find_all_including_deleted_returns_all` 验证 find_all_including_deleted 返回含墓碑
  - [x] SubTask 5.3: 先写测试 `mock_repo::tests::physical_delete_removes_record` 验证 physical_delete 真正删除
  - [x] SubTask 5.4: 注：trait `delete` 保留硬删除语义（向后兼容），service 层改用 `domain Note::delete() + save` 软删除路径
  - [x] SubTask 5.5: 实现 mock_repo：3 类实体的 `find_all_including_deleted` + `find_by_id_including_deleted` + `physical_delete` + 所有查询过滤墓碑
  - [x] SubTask 5.6: 运行 `cargo test mock_repo::tests` 验证通过

## 阶段四：DB Migration

- [x] Task 6: database.rs 加 `deleted_at TEXT` 列 migration
  - [x] SubTask 6.1: 先写测试 `database::tests::migration_adds_deleted_at_column` 验证旧 DB（无 deleted_at 列）初始化后存在该列
  - [x] SubTask 6.2: 先写测试 `database::tests::fresh_db_has_deleted_at_column` 验证新 DB 有 deleted_at 列
  - [x] SubTask 6.3: 实现 migration：抽取 `has_column` 辅助函数，检查 3 张表的 `deleted_at` 列，不存在则 `ALTER TABLE ADD COLUMN deleted_at TEXT`
  - [x] SubTask 6.4: 运行 `cargo test database::tests` 验证通过

## 阶段五：SQLite repo 实现（TDD）

- [x] Task 7: sqlite_note_repo 实现新方法 + 查询过滤墓碑 + FTS5 search 过滤
  - [x] SubTask 7.1: 先写测试 `sqlite_note_repo::tests::find_all_excludes_tombstones`
  - [x] SubTask 7.2: 先写测试 `sqlite_note_repo::tests::find_all_including_deleted_returns_tombstones`
  - [x] SubTask 7.3: 先写测试 `sqlite_note_repo::tests::search_notes_excludes_tombstones`
  - [x] SubTask 7.4: 先写测试 `sqlite_note_repo::tests::save_tombstone_persists_deleted_at`
  - [x] SubTask 7.5: 先写测试 `sqlite_note_repo::tests::save_non_tombstone_clears_deleted_at`（复活场景：UPDATE 时显式写 deleted_at=NULL）
  - [x] SubTask 7.6: 先写测试 `sqlite_note_repo::tests::physical_delete_removes_record`
  - [x] SubTask 7.7: 实现：save SQL 含 `deleted_at` 列 + row_to_note 读 `deleted_at` + 所有查询加 `WHERE deleted_at IS NULL` + search_notes SQL 加 `WHERE deleted_at IS NULL`（LIKE 路径用括号包裹 OR，FTS5 路径用 n. 前缀）+ 新增 `find_all_including_deleted` + `find_by_id_including_deleted` + `physical_delete`
  - [x] SubTask 7.8: 运行 `cargo test sqlite_note_repo::tests` 验证通过（26 测试全过）

- [x] Task 8: sqlite_reminder_repo 实现新方法 + 查询过滤墓碑（同 Task 7 模式）
  - [x] SubTask 8.1: 先写测试覆盖 find_all/find_by_note_id/find_due/find_next_due_time/find_by_date_range/find_all_including_deleted/physical_delete/save 软删除/复活
  - [x] SubTask 8.2: 实现（无 FTS5），save 改为 ON CONFLICT DO UPDATE 显式覆盖 deleted_at
  - [x] SubTask 8.3: 运行 `cargo test sqlite_reminder_repo::tests` 验证通过（16 测试全过）

- [x] Task 9: sqlite_template_repo 实现新方法 + 查询过滤墓碑（同 Task 7 模式）
  - [x] SubTask 9.1: 先写测试覆盖 find_all/find_by_id/find_all_including_deleted/physical_delete/save 软删除/复活
  - [x] SubTask 9.2: 实现（无 FTS5）
  - [x] SubTask 9.3: 运行 `cargo test sqlite_template_repo::tests` 验证通过（11 测试全过）

## 阶段六：sync_json_io 改造（TDD）

- [x] Task 10: sync_json_io import 仲裁改用 find_by_id_including_deleted
  - [x] SubTask 10.1: 先写测试 `sync_json_io::tests::import_propagates_tombstone`：本地非墓碑 + 远程墓碑 → 本地变墓碑
  - [x] SubTask 10.2: 先写测试 `sync_json_io::tests::import_local_newer_than_remote_tombstone_kept`：本地 updated_at 更晚 → 不传播删除
  - [x] SubTask 10.3: 先写测试 `sync_json_io::tests::import_revives_when_remote_newer`：本地墓碑 + 远程非墓碑 updated_at 更晚 → 复活
  - [x] SubTask 10.4: 实现：import 仲裁逻辑改用 `find_by_id_including_deleted`（3 个实体）
  - [x] SubTask 10.5: 运行 `cargo test sync_json_io::tests` 验证通过

- [x] Task 11: sync_json_io export 改用 find_all_including_deleted
  - [x] SubTask 11.1: 先写测试 `sync_json_io::tests::export_writes_tombstone_json` + `export_includes_tombstones_in_json`
  - [x] SubTask 11.2: 实现：Note 改用 `find_all_including_deleted`（删除 find_archived 合并逻辑），Reminder/Template 同
  - [x] SubTask 11.3: 运行 `cargo test sync_json_io::tests` 验证通过（16 测试全过，含 11 现有 + 5 新增）

## 阶段七：service 层改软删除（TDD）

- [x] Task 12: note_service delete_note 改软删除（含级联 reminder 软删除）
  - [x] SubTask 12.1: 先写测试 `note_service::tests::delete_note_is_soft_delete`：delete 后 find_all 不含该 note，find_all_including_deleted 含墓碑
  - [x] SubTask 12.2: 先写测试 `note_service::tests::delete_note_cascades_soft_delete_reminder`：delete 后关联 reminder 也变墓碑
  - [x] SubTask 12.3: 先写测试 `note_service::tests::delete_note_emits_note_written_event`（保留现有事件 emit 行为）
  - [x] SubTask 12.4: 实现：delete_note 改用 `note.delete()` + `save`；级联 reminder 改用 `reminder.delete()` + `save`（不再调 `delete_by_note_id`）；close_note_if_empty 也改软删除
  - [x] SubTask 12.5: 运行 `cargo test note_service::tests` 验证通过（37 测试全过）

- [x] Task 13: reminder_service delete_reminder 改软删除
  - [x] SubTask 13.1: 先写测试 `reminder_service::tests::delete_reminder_is_soft_delete` + 3 个相关测试
  - [x] SubTask 13.2: 实现：delete_reminder 改用 `reminder.delete()` + `save`
  - [x] SubTask 13.3: 运行 `cargo test reminder_service::tests` 验证通过（15 测试全过）

- [x] Task 14: template_service delete_template 改软删除
  - [x] SubTask 14.1: 先写测试 `template_service::tests::delete_template_is_soft_delete` + 2 个相关测试
  - [x] SubTask 14.2: 实现：delete_template 改用 `template.delete()` + `save`
  - [x] SubTask 14.3: 运行 `cargo test template_service::tests` 验证通过（8 测试全过）

## 阶段八：墓碑清理（TDD）

- [x] Task 15: 新增墓碑清理模块 sync_tombstone_cleanup.rs
  - [x] SubTask 15.1: 先写测试 `tombstone_cleanup::tests::under_threshold_no_cleanup`：30 条墓碑 → 返回 0，不调 physical_delete
  - [x] SubTask 15.2: 先写测试 `tombstone_cleanup::tests::over_threshold_deletes_oldest`：60 条墓碑阈值 50 → 删除最老 10 条，返回 10
  - [x] SubTask 15.3: 先写测试 `tombstone_cleanup::tests::mixed_three_types_aggregated`：note+reminder+template 合计计算
  - [x] SubTask 15.4: 实现 cleanup_old_tombstones 函数（+ cleanup_old_tombstones_with_threshold 便于测试）
  - [x] SubTask 15.5: 运行 `cargo test sync_tombstone_cleanup::tests` 验证通过（6 测试全过）

- [x] Task 16: git_sync sync 流程触发墓碑清理
  - [x] SubTask 16.1: 在 `git_sync::sync` 的 `sync_data_bidirectional` 之后（阶段 4.5）调用 `cleanup_old_tombstones`（阈值 50）
  - [x] SubTask 16.2: 运行 `cargo test git_sync::tests` 验证现有测试不破（11 测试全过）

## 阶段九：全量验证

- [x] Task 17: 全量测试通过
  - [x] SubTask 17.1: 运行 `cargo build` 确认无编译错误
  - [x] SubTask 17.2: 运行 `cargo test --lib` 全量通过（392 测试全过，0 失败）
  - [x] SubTask 17.3: 手动启动应用验证由用户后续执行（API 契约未变更，前端无需改动）

## 阶段十：知识库同步

- [x] Task 18: 知识库更新
  - [x] SubTask 18.1: `constraints.md` 修订 INV-011（仲裁含墓碑）+ 更新 INV-032（墓碑清理已实施）+ 变更记录
  - [x] SubTask 18.2: `lessons/README.md` LES-028 状态更新为"已修复" + 新增 LES-029（墓碑机制实现经验）+ 变更记录
  - [x] SubTask 18.3: `boundaries.md` 数据同步能力描述补充墓碑传播机制 + 对应代码补充 sync_tombstone_cleanup.rs
  - [x] SubTask 18.4: `glossary.md` 新增"Tombstone（墓碑）"术语

# Task Dependencies

- Task 2/3 独立于 Task 1，可并行
- Task 4 依赖 Task 1/2/3（trait 引用领域模型字段）
- Task 5 依赖 Task 4
- Task 6 独立（DB migration 不依赖领域模型）
- Task 7/8/9 依赖 Task 4 + Task 6，三类可并行
- Task 10/11 依赖 Task 7/8/9
- Task 12/13/14 依赖 Task 5（mock 用于测试）+ Task 7/8/9
- Task 15 依赖 Task 5 + Task 7/8/9（mock + sqlite 实现可用）
- Task 16 依赖 Task 10/11 + Task 15
- Task 17 依赖所有前序任务
- Task 18 依赖 Task 17 验证通过
