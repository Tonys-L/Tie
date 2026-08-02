# 墓碑同步机制 Spec

## Why

跨设备删除传播失效（LES-028）：设备 A 删除便签后，设备 B 同步时 `import_from_json` 只遍历存在的 JSON 文件，对"缺失"的文件视而不见，导致 DB 中保留被删便签；随后 export 重新写出 JSON，删除被撤销。根因是同步协议缺少"删除事件"载体，硬删除不留痕迹无法参与 last-write-wins 仲裁。

## What Changes

- Note/Reminder/Template 领域模型新增 `deleted_at: Option<String>` 字段
- 新增 `delete()` 方法执行软删除：同时设 `deleted_at = now` 和 `updated_at = now`（确保 last-write-wins 仲裁无需改动核心逻辑）
- 新增 `is_deleted()` 查询方法
- Repository trait 新增 `find_all_including_deleted()`（供 export 写出墓碑 JSON）和 `physical_delete()`（仅供墓碑清理使用）
- Repository 所有现有查询方法默认过滤墓碑（`WHERE deleted_at IS NULL`），FTS5 search 同样过滤
- `sync_json_io::import_from_json` 仲裁改用 `find_by_id_including_deleted`，让墓碑参与仲裁
- `sync_json_io::export_to_json` 改用 `find_all_including_deleted`，写出墓碑 JSON
- service 层（note/reminder/template）的 delete 操作改用 domain 的 `delete()` + `save`，不再调 `repo.delete`
- 新增墓碑清理：sync 流程 import 完成后执行，50 条阈值（跨三类合计），按 `deleted_at` 降序物理删除最老的
- database.rs migration：3 张表 ALTER TABLE ADD COLUMN `deleted_at TEXT`
- 知识库：新增 INV-032（墓碑机制），修订 INV-011（仲裁含墓碑），LES-028 标记已修复

## Impact

- Affected specs: 数据同步能力（boundaries.md）、INV-011（last-write-wins）、INV-024（先拉后推）
- Affected code:
  - `src-tauri/src/domain/{note,reminder,template}.rs` — 新增字段和方法
  - `src-tauri/src/domain/repositories.rs` — 3 个 trait 新增方法
  - `src-tauri/src/domain/mock_repo.rs` — 实现新方法 + delete 改软删除
  - `src-tauri/src/infrastructure/database.rs` — migration 加列
  - `src-tauri/src/infrastructure/sqlite_{note,reminder,template}_repo.rs` — 实现新方法 + 查询过滤墓碑
  - `src-tauri/src/application/sync_json_io.rs` — export/import 改造
  - `src-tauri/src/application/{note,reminder,template}_service.rs` — delete 改软删除
  - `src-tauri/src/application/git_sync.rs` — sync 后触发墓碑清理
- 不影响：Tauri 命令签名、前端代码、API 契约（前端不感知墓碑）

## ADDED Requirements

### Requirement: 软删除与墓碑字段
Note/Reminder/Template 实体 SHALL 提供 `deleted_at: Option<String>` 字段和 `delete()` 方法。`delete()` 方法 SHALL 同时设置 `deleted_at = now` 和 `updated_at = now`，确保 `updated_at` 始终代表最后一次写操作时间（无论创建/编辑/删除）。

#### Scenario: 删除便签后变为墓碑
- **WHEN** 调用 `note.delete()`
- **THEN** `note.deleted_at` 为 Some(当前时间)
- **AND** `note.updated_at` 等于 `note.deleted_at`
- **AND** `note.is_deleted()` 返回 true

#### Scenario: 删除时 updated_at 同步更新
- **WHEN** 10:00 创建便签，10:05 编辑，10:10 调用 delete()
- **THEN** `updated_at` = "10:10"（不是 10:05）
- **AND** `deleted_at` = "10:10"

### Requirement: Repository 查询默认过滤墓碑
所有面向业务逻辑的查询方法（find_all/find_archived/find_by_note_id/find_due/search_notes 等）SHALL 默认过滤 `deleted_at IS NOT NULL` 的记录，确保业务逻辑不感知墓碑。

#### Scenario: find_all 不返回墓碑
- **GIVEN** DB 中有 2 条活跃便签和 1 条墓碑
- **WHEN** 调用 `find_all()`
- **THEN** 返回 2 条活跃便签

#### Scenario: search_notes 不返回墓碑
- **GIVEN** DB 中有墓碑便签内容匹配搜索词
- **WHEN** 调用 `search_notes("关键词")`
- **THEN** 不返回墓碑便签

### Requirement: find_all_including_deleted 供 export 使用
Repository SHALL 提供 `find_all_including_deleted()` 方法，返回所有记录（含墓碑），仅供 `sync_json_io::export_to_json` 使用，写出墓碑 JSON 让其他设备 import 时能传播删除。

#### Scenario: export 写出墓碑 JSON
- **GIVEN** DB 中 note-X 是墓碑（deleted_at = Some(...)）
- **WHEN** 调用 `export_to_json`
- **THEN** sync/notes/note-X.json 文件存在
- **AND** JSON 内容包含 `deleted_at` 字段

### Requirement: physical_delete 仅供墓碑清理
Repository SHALL 提供 `physical_delete(id)` 方法执行物理删除（DELETE FROM），仅供墓碑清理逻辑使用。正常业务删除 SHALL 走 domain `delete()` + `save` 软删除路径。

#### Scenario: 墓碑清理调用 physical_delete
- **WHEN** 墓碑清理逻辑判断需清理 note-X
- **THEN** 调用 `repo.physical_delete("note-X")`
- **AND** note-X 从 DB 物理删除（不再存在于任何查询结果）

### Requirement: import 仲裁含墓碑
`sync_json_io::import_from_json` SHALL 使用 `find_by_id_including_deleted` 查询本地记录，让墓碑参与 last-write-wins 仲裁。仲裁规则不变：`item.updated_at > existing.updated_at` 时 save（墓碑覆盖本地 / 非墓碑覆盖墓碑即复活）。

#### Scenario: 远程墓碑传播到本地
- **GIVEN** 本地 note-X 非墓碑（updated_at = 10:05）
- **AND** 远程 note-X.json 是墓碑（updated_at = 10:10, deleted_at = 10:10）
- **WHEN** import 处理 note-X.json
- **THEN** `item.updated_at (10:10) > existing.updated_at (10:05)` 为 true
- **AND** save(item) 后本地 note-X 变为墓碑

#### Scenario: 本地更新晚于远程删除时不传播
- **GIVEN** 本地 note-X 非墓碑（updated_at = 10:15）
- **AND** 远程 note-X.json 是墓碑（updated_at = 10:10）
- **WHEN** import 处理 note-X.json
- **THEN** `item.updated_at (10:10) > existing.updated_at (10:15)` 为 false
- **AND** 不 save，本地 note-X 保留为非墓碑（删除被覆盖，相当于撤销）

#### Scenario: 远程复活覆盖本地墓碑
- **GIVEN** 本地 note-X 是墓碑（updated_at = 10:10, deleted_at = 10:10）
- **AND** 远程 note-X.json 非墓碑（updated_at = 10:15, deleted_at = null）
- **WHEN** import 处理 note-X.json
- **THEN** `item.updated_at (10:15) > existing.updated_at (10:10)` 为 true
- **AND** save(item) 后本地 note-X 复活（deleted_at 被覆盖为 null）

### Requirement: 墓碑清理
sync 流程 SHALL 在 import 完成后执行墓碑清理：跨 note/reminder/template 三类合计，按 `deleted_at` 降序排序，超过 50 条时物理删除最老的墓碑。

#### Scenario: 墓碑未超阈值不清理
- **GIVEN** 三类合计 30 条墓碑
- **WHEN** sync 后执行墓碑清理
- **THEN** 不执行 physical_delete
- **AND** 返回清理数量 0

#### Scenario: 墓碑超阈值清理最老的
- **GIVEN** 三类合计 60 条墓碑，deleted_at 最早的是 note-X
- **WHEN** sync 后执行墓碑清理（阈值 50）
- **THEN** 物理删除最老的 10 条墓碑
- **AND** 返回清理数量 10

### Requirement: DB Migration
database.rs SHALL 在初始化时检查 notes/reminders/templates 三张表是否有 `deleted_at` 列，不存在则 `ALTER TABLE ADD COLUMN deleted_at TEXT`（默认 NULL）。

#### Scenario: 新数据库有 deleted_at 列
- **WHEN** 全新数据库初始化
- **THEN** notes/reminders/templates 表都有 `deleted_at` 列

#### Scenario: 旧数据库自动迁移
- **GIVEN** 旧数据库无 `deleted_at` 列
- **WHEN** 应用启动
- **THEN** 自动添加 `deleted_at TEXT` 列
- **AND** 现有数据 `deleted_at` 为 NULL（视为非墓碑）

## MODIFIED Requirements

### Requirement: last-write-wins 仲裁（INV-011 修订）
冲突解决策略 last-write-wins 按 `updated_at` 取最新。**修订**：仲裁时 SHALL 使用 `find_by_id_including_deleted` 查询本地记录（含墓碑），让墓碑的 `updated_at`（= 删除时间）参与比较。墓碑的 `updated_at` 在 `delete()` 时设为 `deleted_at` 的值，确保删除操作的时间戳正确参与仲裁。

### Requirement: service 层 delete 操作
note_service/reminder_service/template_service 的 delete 方法 SHALL 改用 domain `delete()` 方法 + `save`（软删除），不再调用 `repo.delete`（硬删除）。`delete_note` 的级联删除 reminder 也 SHALL 改为软删除（`reminder.delete()` + `save`）。

### Requirement: INV-025 移除 50% 检查（已在前序任务完成）
**注**：此前的 #BUGFIX-003 已移除 50% 删除检查，本 spec 不再涉及。

## REMOVED Requirements

### Requirement: 硬删除
**Reason**: 硬删除导致跨设备删除传播失效（LES-028），改为软删除 + 墓碑机制。
**Migration**: service 层 delete 改调 domain `delete()` + `save`；DB migration 添加 `deleted_at` 列；现有数据视为非墓碑（`deleted_at = NULL`）。
