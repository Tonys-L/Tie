# Checklist

## 领域模型墓碑字段
- [x] Note 模型有 `deleted_at: Option<String>` 字段
- [x] Note 模型有 `delete()` 方法（同时设 deleted_at 和 updated_at）
- [x] Note 模型有 `is_deleted()` 方法
- [x] Reminder 模型有 `deleted_at` 字段 + `delete()` + `is_deleted()`
- [x] Template 模型有 `deleted_at` 字段 + `delete()` + `is_deleted()`
- [x] `note.delete()` 后 `updated_at == deleted_at`
- [x] `deleted_at` 字段使用 `#[serde(default, skip_serializing_if = "Option::is_none")]`（向后兼容旧 JSON）

## Repository trait 扩展
- [x] NoteRepository 有 `find_all_including_deleted()` 方法
- [x] NoteRepository 有 `physical_delete(id)` 方法
- [x] ReminderRepository 有 `find_all_including_deleted()` + `physical_delete(id)`
- [x] TemplateRepository 有 `find_all_including_deleted()` + `physical_delete(id)`

## Mock repo 实现
- [x] InMemoryNoteRepository 实现新方法
- [x] InMemoryReminderRepository 实现新方法
- [x] InMemoryTemplateRepository 实现新方法
- [x] mock trait delete 保留硬删除（向后兼容），service 层改用 domain delete() + save 软删除
- [x] mock find_all/find_archived/find_by_note_id 等查询过滤墓碑
- [x] mock find_all_including_deleted 返回含墓碑

## DB Migration
- [x] notes 表有 `deleted_at TEXT` 列
- [x] reminders 表有 `deleted_at TEXT` 列
- [x] templates 表有 `deleted_at TEXT` 列
- [x] 旧 DB（无 deleted_at 列）启动后自动加列
- [x] 新 DB 默认有 deleted_at 列
- [x] 现有数据 deleted_at 为 NULL（视为非墓碑）

## SQLite repo 实现
- [x] sqlite_note_repo save 写 deleted_at 字段
- [x] sqlite_note_repo row_to_note 读 deleted_at 字段
- [x] sqlite_note_repo find_all/find_archived/find_by_id 过滤墓碑
- [x] sqlite_note_repo search_notes 过滤墓碑（FTS5 + WHERE deleted_at IS NULL，LIKE 路径用括号包裹 OR）
- [x] sqlite_note_repo find_all_including_deleted 返回含墓碑
- [x] sqlite_note_repo physical_delete 真正删除
- [x] sqlite_note_repo save 非墓碑时清空 deleted_at（复活场景，ON CONFLICT 显式 excluded.deleted_at）
- [x] sqlite_reminder_repo 同上（无 FTS5，save 改为 ON CONFLICT DO UPDATE）
- [x] sqlite_template_repo 同上（无 FTS5）

## sync_json_io 改造
- [x] import 仲裁用 find_by_id_including_deleted
- [x] import 远程墓碑 updated_at > 本地 → 传播软删除
- [x] import 本地 updated_at 更晚 → 不传播删除
- [x] import 远程非墓碑 updated_at > 本地墓碑 → 复活
- [x] export 用 find_all_including_deleted
- [x] export 写出墓碑 JSON（含 deleted_at 字段）

## service 层改软删除
- [x] note_service delete_note 改用 note.delete() + save
- [x] note_service delete_note 级联 reminder 改用 reminder.delete() + save（不再调 delete_by_note_id）
- [x] note_service batch_delete 委托 delete_note 自动获得软删除
- [x] note_service close_note_if_empty 改用软删除
- [x] reminder_service delete_reminder 改用软删除
- [x] template_service delete_template 改用软删除
- [x] service delete 仍 emit 对应事件（NoteWritten/ReminderWritten/TemplateWritten）

## 墓碑清理
- [x] 有 cleanup_old_tombstones 函数
- [x] 跨 note/reminder/template 三类合计计算
- [x] 按 deleted_at 升序排序（最老在前）
- [x] 超过 50 条阈值时物理删除最老的
- [x] git_sync sync 流程在 sync_data_bidirectional 后调用 cleanup_old_tombstones（阶段 4.5）
- [x] 未超阈值时返回 0 不调 physical_delete

## 不影响现有功能（回归验证）
- [x] 现有 cargo test --lib 全部通过（392 测试全过，0 失败）
- [ ] 便签创建/编辑/删除/归档/搜索正常（需用户手动验证）
- [ ] 提醒创建/触发/贪睡/删除正常（需用户手动验证）
- [ ] 模板 CRUD + 从模板新建便签正常（需用户手动验证）
- [ ] 手动同步正常（含分支创建流程）（需用户手动验证）
- [ ] 托盘菜单同步正常（需用户手动验证）
- [ ] 自动同步（30 秒防抖）正常（需用户手动验证）
- [ ] 应用启动恢复便签窗口正常（需用户手动验证）
- [x] Tauri 命令签名未变更（前端无需改动）
- [x] API 契约未变更

## 知识库同步
- [x] constraints.md 新增 INV-032（墓碑机制）
- [x] constraints.md 修订 INV-011（仲裁含墓碑）
- [x] constraints.md 变更记录填写 2026-08-03 #FEAT-TOMBSTONE
- [x] lessons/README.md 标记 LES-028 已修复
- [x] lessons/README.md 新增 LES-029（墓碑机制实现经验）
- [x] boundaries.md 数据同步能力描述补充墓碑传播
- [x] glossary.md 新增 Tombstone（墓碑）术语
