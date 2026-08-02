# 约束 (Constraints)

> **必读文档**：任何任务都必须阅读本文档。约束不可被绕过。

---

## 设计原则

优先级裁决（冲突时按此顺序）：

```text
业务优先 > 职责优先 > 变更成本 > 简单优先 > 扩展优先
```

核心哲学：

```text
业务优先于技术
职责优先于分层
变更成本优先于开发速度
简单优先于复杂
扩展优先于修改
```

任何设计都应首先解决真实业务问题，而非追求某种架构风格、框架特性或技术概念。

> 上方优先级链为本哲学各项的简称，冲突时按此顺序裁决。

---

### 优先关注能力，而非数据

业务能力描述系统创造的价值，而非数据库中的实体。

---

### 职责清晰

每个模块必须能够回答：

```text
我负责什么？
我不负责什么？
```

如果一个模块存在多个变化原因，则说明职责划分存在问题。

---

### 避免过度设计

```text
Simple First, Evolve Later
```

- 不要为猜测中的需求设计
- 不要为了架构而架构
- 抽象必须来自真实变化，而非未来可能

三次法则：第一次直接实现，第二次允许重复，第三次评估抽象。

---

### 为已知变化设计

如果变化已经明确存在，则应提前设计合理边界。禁止为未知变化设计。

---

## 架构约束

### 三层隔离

逻辑分层，业务内聚、变更隔离、依赖单向。

```text
策略层（易变）
    ↓
核心层（稳定）
    ↑
技术层（可替换）
```

#### 架构状态

- 当前状态：已实施
- 未隔离的模块：无

domain 层（核心层）零技术框架依赖，仅使用 serde/uuid/chrono 值对象工具库。application 层（策略层）通过仓储 trait 访问数据。infrastructure 层（技术层）实现 domain 定义的 trait。

| 架构状态 | AI 代码定位能力 | 文档策略 |
|----------|----------------|----------|
| 已实施 | 精准：核心层内聚，AI 可直接定位 | 只记录负空间 |

架构状态不是固定的。随着代码重构，状态可能变化，文档应逐步精简。

#### 策略层

回答：**选择怎么做？**

负责：Tauri 命令编排、窗口管理策略、提醒调度策略、Git 同步策略。

#### 核心层

回答：**能做什么？必须遵守什么？**

负责：能力契约（NoteRepository/ReminderRepository trait）、领域模型（Note/Reminder）、业务规则、不变量。

不变量优先：流程可以变化，不变量不能被绕过。

#### 技术层

回答：**具体如何完成？**

负责：SQLite 持久化实现（SqliteNoteRepository/SqliteReminderRepository）、数据库迁移。

技术层可替换（换数据库，核心层不受影响）。

---

### 依赖方向

| 源 | 目标 | 允许 | 说明 |
|----|------|------|------|
| application | domain | 是 | application 调用 domain trait + 实体 |
| application | infrastructure | 是 | application 通过 AppState 注入具体实现（仅组合根） |
| infrastructure | domain | 是 | infrastructure 实现 domain 定义的 trait |
| domain | 任何技术层 | 否 | domain 只定义端口，不依赖具体实现 |
| domain 内 | 互相 | 否 | domain 内禁止互相依赖 |

组合根：`lib.rs` setup 函数中构造 `SqliteNoteRepository`/`SqliteReminderRepository` 并通过 `Box::new` 注入 `AppState` 的 trait object 字段。`AppState` 同时持有 `GitSync` 和 `ShortcutManager` 具体结构体。组合根是唯一允许持有具体实现的地方。

---

### 模块边界

- 每个模块对应一个明确的业务能力
- 模块间通过接口通信，禁止直接访问其他模块内部实现
- 所有 `#[tauri::command]` 集中在 `application/commands` 模块（`commands/mod.rs` 门面 + 按业务域拆分的子模块），禁止散落到 domain/infrastructure 层
- 窗口/托盘/快捷键/调度器各自独立模块，互不直接调用
- 业务编排逻辑（仓储调用 + domain 方法）必须下沉到 `*_service` 模块（`note_service`/`reminder_service`/`image_service`），`#[tauri::command]` 仅作为薄壳：调用 service 完成业务 → 执行 Tauri 副作用（emit/schedule_recalc/schedule_auto_sync 等）
- application 层所有模块（`commands/*`、`note_service`、`reminder_scheduler` 等）的窗口操作（destroy/set_always_on_top/set_focus/show/闪烁等）必须委托 `window_manager`，禁止直接调用 `app.get_webview_window()` 操作窗口属性
- 闪烁提示逻辑（临时置顶 300ms + 恢复）统一由 `window_manager::flash_window` 提供，禁止在其他模块重复实现
- 窗口销毁统一用 `window_manager::close_note_window`（封装 `destroy()`），禁止在 commands/note_service 等模块重复内联 `app.get_webview_window().destroy()`
- 窗口置顶统一用 `window_manager::set_note_pinned`（按 bool 设置）或 `window_manager::restore_note_on_top`（按 Note.is_pinned 恢复），禁止在 commands/note_service 等模块重复内联 `app.get_webview_window().set_always_on_top()`
- 窗口聚焦+事件发送统一用 `window_manager::focus_note_window_and_emit`（返回 bool 表示窗口是否存在），禁止在 note_service 等模块重复内联 `app.get_webview_window() + set_focus() + emit()`
- **命令层薄壳化**：`#[tauri::command]` 函数仅负责参数传递 + Tauri 副作用（emit/window_manager/schedule_recalc 等），业务规则（校验/参数解析/第三方库调用）必须下沉到 `*_service`/`*_validation`/封装模块。具体应用：
  - AI 命令的字符/数量校验必须经 `ai_validation::validate_rewrite_text`/`validate_sort_todos`，禁止在 `ai_commands` 内联校验
  - 报告周期参数解析必须经 `report_generator::parse_period`，禁止在 `ai_commands::generate_report` 内联 `period_type` match
  - 农历计算必须经 `lunar_calendar::lunar_date_text`/`TymeCalendarAdapter`，禁止在 `commands/*` 或 `reminder_scheduler` 直接 `use tyme4rs`（消除 shallow module，第三方库统一入口）
- **locality 下沉（副作用集中）**：相关副作用必须与触发它的写操作在同一个 service 模块内，禁止在命令层跨模块调用副作用。具体应用：`delete_note` 的图片清理必须在 `note_service::delete_note` 内部（先 find_by_id 取 content 再 cleanup），`batch_delete` 内部循环调用 `delete_note` 自动触发图片清理，消除单/批量路径不对称；禁止在 `note_commands::delete_note` 命令层调用 `image_service::cleanup_removed_images`
- **跨模块常量集中**：跨模块共享的技术常量必须定义为 `pub const` 并集中在所有者模块，禁止 magic number 散布。具体应用：`CREATE_NO_WINDOW: u32 = 0x08000000` 定义在 `git_ops.rs`，`git_sync.rs`/`sync_commands.rs` 必须引用 `git_ops::CREATE_NO_WINDOW`，禁止内联 `0x08000000`
- **窗口管理模块单一职责**（ADR-009）：`window_manager` 仅负责 Note 窗口生命周期（开窗/关窗/置顶/聚焦/闪烁/启动恢复），Hub 窗口逻辑在 `hub_window_manager`（`toggle_hub_window`/`open_or_focus_hub`/`create_hub_window`），重叠物理计算在 `window_overlap_resolver`（`compute_overlaps` 纯函数 + `resolve_overlaps` 副作用）。`tray_manager`/`shortcut_manager` 的 Hub 窗口操作必须委托 `hub_window_manager`，禁止依赖 `window_manager`
- **commands 子模块按业务域拆分**（ADR-009 同批次）：`application/commands` 下子模块按单一业务能力划分——`note_commands`/`reminder_commands`/`sync_commands`/`shortcut_commands`/`locale_commands`/`system_commands`/`ai_commands`/`template_commands`/`image_commands`，禁止把不相关业务塞同一子模块（原 `sync_commands` 承担同步+快捷键+语言+系统四类已拆分）
- **浅模块深化（常量表替代薄包装函数）**：当模块表现为多个 `pub fn xxx() -> &'static str { t!(...) }` 形式的薄包装函数（每个函数仅一行查表返回）时，必须重构为常量表 + `.get()` 方法模式。具体应用：`locale_manager` 用 `LocaleText { zh, en }` 结构体 + 9 个 `pub const` 常量（`MENU_NEW_NOTE`/`NOTIFY_SYNC_OK` 等）+ `LocaleText::get()` 按 `LOCALE` 全局开关返回文本，替代原 9 个 `pub fn menu_xxx()/notify_xxx()` 薄函数；调用方 `MENU_NEW_NOTE.get()` vs `menu_new_note()` 语义更清晰，新增文本只需加一行 `pub const`，无需新增函数
- **JSON 片段提取单点归属**：从 AI 返回文本中提取 JSON 片段的逻辑必须集中在 `application::json_extract` 模块，禁止在 `ai_commands`/`reminder_parser` 等调用方独立实现 `find('{') + rfind('}')` 手写切片。具体应用：`json_extract::extract_object`（提取 `{` 开头对象）+ `extract_array`（提取 `[` 开头数组）用 `serde_json::Deserializer::into_iter::<IgnoredAny>` 流式解析 + `byte_offset()` 精确取边界，正确处理 JSON 字符串值内含 `{`/`}`/`[`/`]` 字符的场景（旧 `rfind` 实现会误切边界）；`ai_commands::ai_sort_todos` 和 `reminder_parser::parse_reminder_json`/`sniff_suggestions` 必须委托本模块
- **函数职责按业务概念归属**：函数必须放在与其业务概念对应的模块中，禁止技术命名模块承载不相关业务函数。具体应用：`formatNoteTime`（时间格式化）归属 `datetime.ts`（日期时间工具），禁止放在 `colors.ts`（颜色映射）；`applyNoteStyle`（颜色应用）归属 `colors.ts`，禁止放在 `datetime.ts`。判断标准：函数处理的业务概念（颜色/时间/toast/HTML 转义）应与模块名（colors/datetime/toast/html）一一对应

### 写操作事件化（ADR-007 + ADR-008 扩展）

- **service 层 emit 事件**：所有写操作（create/update/delete）完成后必须 emit `DomainEvent`（`NoteWritten`/`ReminderWritten`/`TemplateWritten`），携带 `WriteAction`（`Created`/`Updated`/`Deleted`）+ 实体 id
- **scheduler 层 emit 事件**（ADR-008）：`reminder_scheduler::fire_reminders_with_deps` 在 `reminder_repo.save(&updated)` 成功后必须 emit `DomainEvent::ReminderWritten { action: WriteAction::Updated, id }`，与 service 层 emit 语义一致；scheduler 接收 `&dyn EventPublisher` + `&dyn ReminderQuery` 参数
- **service 层依赖 `&dyn EventPublisher`**（trait object，可 mock，可替换实现），禁止依赖具体类型 `EventBus`
- **scheduler 层依赖 `&dyn EventPublisher` + `&dyn ReminderQuery`**（trait object），禁止依赖具体类型 `EventBus` 或 `ReminderRepository`
- **监听器集中注册**：`schedule_auto_sync`（所有 `DomainEvent`）+ `schedule_recalc`（仅 `ReminderWritten`）等副作用监听器在 `lib.rs` setup 中通过 `event_bus.subscribe(...)` 注册（两条监听器），禁止在 commands/tray/shortcut/scheduler 等调用方手动触发 `schedule_auto_sync` 或 `schedule_recalc`
- **新增写操作只需在 service/scheduler 内 emit 事件**，副作用自动触发，无需调用方感知
- **EventPublisher trait 是切换 seam**：当前同步实现（`EventBus`），未来可替换为 `ChannelPublisher`（基于 `tokio::sync::broadcast`），service/scheduler 签名零改动
- **template 写操作必须经 `template_service`**，禁止 `template_commands` 直接访问 `template_repo`（与 note/reminder 模式对齐）

### Repository trait CQRS 拆分（ADR-010）

- **聚合 CRUD 与读投影分离**：`NoteRepository`（save/find_by_id/find_all/delete/find_archived）与 `NoteQuery`（search_notes/find_activity_by_month）独立 trait；`ReminderRepository`（save/find_by_id/find_all/find_by_note_id/delete/delete_by_note_id）与 `ReminderQuery`（find_due/find_next_due_time/find_by_date_range）独立 trait
- **TemplateRepository 不拆分**（YAGNI，方法少且无读投影需求）
- **实现层双 impl**：`SqliteNoteRepository` 同时 impl `NoteRepository` + `NoteQuery`（两个 impl 块）；`SqliteReminderRepository` 同时 impl `ReminderRepository` + `ReminderQuery`；`InMemoryXxxRepository` 同样双 impl
- **AppState 持有 4 个仓储字段**：`note_repo`/`note_query`/`reminder_repo`/`reminder_query`，组合根中 `SqliteXxxRepository::new(db.clone())` 构造两次
- **scheduler 依赖 `&dyn ReminderQuery`**：`reminder_scheduler::start` 循环用 `state.reminder_query.find_next_due_time`，`fire_reminders_with_deps` 接收 `reminder_query: &dyn ReminderQuery` 替代 `reminder_repo.find_due`，表达"scheduler 只依赖读投影"语义
- **查询方法必须通过 query trait**：`note_commands::search_notes` 用 `state.note_query.search_notes`，`reminder_commands::find_by_date_range` 用 `state.reminder_query`，`reminder_commands::find_activity_by_month` 用 `state.note_query`，禁止用 `*_repo` 调用查询方法

### mock/sqlite 仓储保真度（LES-023）

- **mock 以 sqlite 为基准对齐行为契约**：`InMemoryXxxRepository`（mock）必须模拟 `SqliteXxxRepository`（生产）的行为契约，禁止 mock 自定义语义。具体应用：
  - **delete 幂等**：mock `delete` 对不存在的 id 返回 `Ok(())`（与 sqlite `DELETE WHERE id=?` 0 行受影响对齐），禁止返回 `Err("not found")`
  - **find_all/find_by_note_id 排序对齐**：mock 用 HashMap 存储时，返回集合的方法必须显式 `sort_by`（与 sqlite `ORDER BY` 对齐）。`InMemoryReminderRepository::find_all`/`find_by_note_id` 按 `remind_at` 排序；`InMemoryTemplateRepository::find_all` 按 `sort_order` then `created_at` 排序
  - **find_by_id 返回 None 语义对齐**：mock 和 sqlite 对不存在的 id 都返回 `Ok(None)`，禁止 mock 返回 `Err`
- **mock/sqlite 一致性测试**：`infrastructure/mod.rs` `repo_consistency_tests` 模块必须覆盖两套实现的等价行为，包括：delete 不存在 id 幂等、find_all/find_by_note_id 排序一致、find_by_id 返回 None 一致。新增 mock 方法时必须同步新增对应的一致性测试
- **service 层 delete 存在性守卫**：`note_service::delete_note`/`reminder_service::delete_reminder`/`template_service::delete_template` 在 delete 前必须 `find_by_id` 检查存在性，返回 None 时直接 `Ok(())` 不 emit 事件（INV-013 语义保真）。`note_service::batch_delete` 预检查存在性后再调用 `delete_note`，避免幂等 delete 导致 succeeded 列表错误包含 nonexistent id
- **一致性测试辅助函数处理 setup 漂移**：`sqlite_reminder_repo_with_note`（先插入父 note 满足外键约束）、`sqlite_template_repo_empty`（清空默认模板），避免外键约束和默认种子数据导致测试失败

### 前端模块边界（AI 可读性约束）

- **入口文件仅编排**：`src/main.ts`（便签窗口入口）和 `src/hub.ts`（Hub 入口）仅负责编排（init/load/页面切换/全局事件监听），具体业务实现必须拆分到独立模块；入口文件不包含业务逻辑
- **文件命名=业务名**：前端模块文件必须使用业务/UI 部件名（如 `tag-bar.ts`、`note-renderer.ts`、`reminder-dialog.ts`），禁止使用通用技术名（如 `utils.ts`、`helpers.ts`）。**已删除的 `utils.ts`/`note-style.ts` 不允许重建**——共享逻辑按职责拆到 `colors.ts`/`datetime.ts`/`toast.ts`/`html.ts` 等独立 module
- **JSDoc 三段头**：每个前端模块文件必须包含 JSDoc 头，三段：职责边界、被调用方、依赖
- **单向依赖无环**：前端模块间禁止循环依赖。当前依赖方向：`main.ts → note-renderer → colors`；`context-menu → colors`；`hub.ts → notes-list → reminder-dialog`；`calendar-view → notes-list`
- **callback 模式破环**：跨模块双向协作必须用 callback 参数避免循环依赖。`renderNote(note, setupEventsCallback)`、`showReminderDialog(noteId, noteTitle, onNotesChanged)`、`showTemplateDialog(title, app, onSelect)`
- **state 就近原则**：模块级私有 state + getter/setter 导出。`note-context.ts`（getNote/setNote）、`notes-list.ts`（getActiveNotes/getArchivedNotes）、`sync-settings.ts`（syncConfigLoaded 防重复绑定）
- **side-effect 模块**：纯绑定型模块（如 `template-manager.ts`、`update-check.ts`）用 `import './xxx'` 引入，顶层执行按钮绑定，无 named export
- **函数 < 100 行**：前端函数体必须 < 100 行，降低 AI 上下文负担；超长函数必须按 UI 部件/事件类型拆分（如 `setupNoteEvents` 拆分为 5 个 bind 子函数）
- **共享样式逻辑提取**：被 2 个以上模块复用的样式/格式化逻辑必须提取到独立 module。具体应用：`colors.ts`（`COLOR_MAP`/`COLORS`/`applyNoteStyle`，被 note-renderer 和 context-menu 共享）；`datetime.ts`（`localISO`/`formatDate`/`formatNoteTime`/`quickDate`/`repeatLabel`，被 reminder-dialog/calendar-view/notes-list/note-renderer 等共享）；`toast.ts`（`showToast`，被所有页面共享）；`html.ts`（`escapeHtml`，被 markdown-renderer 等共享）
- **AI 调用统一入口**（ADR-009 同批次）：AI 配置查询和 AI 调用包装必须经 `ai-client.ts`，禁止在 `ai-todo-sort`/`ai-rewrite`/`ai-sniff`/`context-menu` 等模块内联 `api.getAiConfig()` 或重复 try/catch + toast 模式。具体应用：`isAiConfigured()`（带 5 秒缓存 + ai-config-changed 事件清缓存）+ `getAiConfigCached()`（带缓存的完整配置读取）+ `runAi<T>(op, opts)`（统一 loading/success/error toast 包装）。例外：`ai-settings.ts` 配置页本身每次刷新表单需读最新值，仍直接调用 `api.getAiConfig`

---

### Tauri 命令约束

- **可能并发调用的命令必须标记为 `async`**，让 Tauri 在线程池执行，避免阻塞主线程导致死锁
- 同步命令（`pub fn`）仅在主线程执行，与窗口创建/IPC 事件并发时会产生死锁
- 耗时操作命令（如 `sync_notes` 执行 Git 子进程+网络）已改为 async，不阻塞主线程
- 多窗口并发调用的命令（`get_note`、`open_note_with_flag`、`update_note_content`、`update_note_title`、`update_note_style`、`update_note_window_state`、`check_git`）也已改为 async，避免多窗口初始化或子进程调用阻塞主线程
- 命令参数中 `State` 的生命周期标注必须用 `State<'_, AppState>`（async 命令）或 `State<AppState>`（同步命令）
- **写操作命令必须调用 `schedule_auto_sync`**：所有修改便签或提醒数据的命令，在业务逻辑完成后必须调用 `state.git_sync.schedule_auto_sync(app)`，确保自动同步防抖机制生效

### Tauri 2.0 权限约束

- `capabilities/default.json` 的 `windows` 列表必须包含所有窗口 label 前缀：`["main","note","note-*","settings","archive-list","hub"]`
- 未列出的窗口无命令调用权限，`invoke` 会静默挂起

### 窗口管理约束

- 每张便签一个独立窗口，label 格式 `note-{uuid}`
- 便签窗口必须 `decorations(false)` + `transparent(true)` + `shadow(false)`
- 窗口已存在时禁止重复创建，应聚焦 + 闪烁提示
- 提醒触发的窗口创建由后端直接执行（`open_note_window_with_url`），不依赖前端事件监听
- 提醒触发时若窗口已存在，通过 `emit_to` 发送 `reminder-triggered` 事件让前端显示横幅
- 前端事件监听必须使用 `getCurrentWindow().listen`（窗口级），禁止使用全局 `listen`（会导致所有窗口收到同一事件）

---

### 错误处理

- 核心层定义业务异常类型，携带业务语义
- 技术层将技术异常转换为业务异常（`map_err(|e| e.to_string())`）
- 禁止吞掉异常（空 catch）
- 禁止在核心层使用技术异常类型（如 rusqlite::Error）

---

## 业务不变量

| 编号 | 不变量描述 | 检查位置 |
|------|-----------|----------|
| INV-001 | 透明度范围 0.3~1.0，超出自动 clamp | `Note::set_opacity`（所有写入路径经 domain 方法，仓储无 partial update） |
| INV-003 | 空便签窗口关闭时从 DB 删除（title+content 均空） | `Note::is_empty()`（domain 层聚合根方法）→ `note_service::close_note_if_empty` + `window_manager::restore_all_windows` 调用；前端关闭按钮必须先 await `api.updateNoteContent` 保存完成再 `win.close()`，避免与后端 `close_note_if_empty` 竞态导致已输入内容被误判为空而删除（LES-025） |
| INV-004 | ID 唯一性：UUID v4 + DB PRIMARY KEY + INSERT OR REPLACE | `Note::new` / `Reminder::new` |
| INV-005 | 外键级联：删除 Note 时级联删除 Reminder（DB ON DELETE CASCADE + 代码双保险） | `delete_note` 命令 |
| INV-006 | DB 查询强制用列名（`row.get("id")`）而非索引 | `sqlite_note_repo.rs`、`sqlite_reminder_repo.rs` |
| INV-007 | Note 归档互斥：find_all 返回 is_archived=0，find_archived 返回 is_archived=1 | `sqlite_note_repo.rs` |
| INV-008 | Reminder 仅 Pending 状态可触发；snoozed_until 存在时比较贪睡截止时间 | `Reminder::effective_time()`（domain 单一真相源：snoozed_until 存在时返回 snoozed_until，否则返回 remind_at）→ `Reminder::is_due` 调用 `effective_time()`；`SqliteReminderRepository::find_due` / `InMemoryReminderRepository::find_due` 均委托 `is_due` 过滤，禁止 SQL 重新实现；`find_next_due_time`/`find_by_date_range`（mock + sqlite）均委托 `effective_time()`（sqlite 用 `EFFECTIVE_TIME_EXPR` SQL 常量保持一致），禁止 SQL 重新实现 snoozed_until 逻辑；`is_due` 用 ISO 8601 字符串比较，`now` 与 `remind_at` 格式必须一致（均不带毫秒，`%Y-%m-%dT%H:%M:%SZ`），前端构造 `remind_at` 时显式 `setSeconds(0, 0)` 与界面分钟级精度对齐（LES-026） |
| INV-009 | 周期提醒触发后由调度器计算 next_trigger 并重置为 Pending；一次性标记为 Triggered | `reminder_scheduler.rs` |
| INV-010 | 双存储：SQLite 为运行时存储，JSON 文件为同步载体，notes.db 不入 Git | `git_sync.rs` |
| INV-011 | 冲突解决：last-write-wins，按 updated_at 取最新。**修订（2026-08-03）**：仲裁时 SHALL 使用 `find_by_id_including_deleted` 查询本地记录（含墓碑），让墓碑的 `updated_at`（= 删除时间，由 `delete()` 同时设 `deleted_at == updated_at` 保证）参与比较。墓碑覆盖本地非墓碑→传播删除；非墓碑覆盖本地墓碑→复活 | `git_sync.rs` resolve_json_conflict + `sync_json_io.rs` import_from_json（用 `find_by_id_including_deleted`） |
| INV-012 | push 策略：--force-with-lease | `git_sync.rs` |
| INV-013 | 自动同步防抖：30 秒延迟 | `git_sync.rs` schedule_auto_sync（监听器侧，所有 DomainEvent 触发） + `note_service`/`reminder_service`/`template_service` 写方法 emit `DomainEvent`（事件侧，ADR-007）+ `reminder_scheduler::fire_reminders_with_deps` save 后 emit `ReminderWritten(Updated)`（事件侧，ADR-008 扩展）；监听器在 `lib.rs` setup 中注册（两条监听器：auto_sync 监听所有事件，recalc 仅监听 ReminderWritten）；service 层 delete 方法（delete_note/delete_reminder/delete_template）加存在性守卫：find_by_id 返回 None 时直接 Ok(()) 不 emit 事件，避免对不存在的实体触发 schedule_auto_sync（语义保真，LES-023） |
| INV-014 | Windows 子进程必须设置 CREATE_NO_WINDOW 标志，禁止弹出控制台窗口 | `git_ops.rs` run_git / check_git_installed / list_remote_branches |
| INV-015 | Git 同步前必须验证远程分支存在（`git rev-parse origin/<branch>`），分支不存在时返回 `BRANCH_NOT_FOUND:<已有分支>` 由前端提示用户选择是否创建 | `git_sync.rs` sync 方法 |
| INV-016 | 所有 git 子进程必须设置 `stdin(Stdio::null())`，防止交互式提示导致进程挂起 | `git_ops.rs` run_git / check_git_installed / list_remote_branches |
| INV-017 | Git 同步初始化后必须验证本地分支名与配置一致，不一致时自动重命名 | `git_sync.rs` sync 方法 |
| INV-018 | Note.tags 字段使用 `#[serde(default)]` 确保旧版 JSON 同步文件反序列化为空数组而非报错 | `domain/note.rs` Note 结构体 |
| INV-019 | 标签数量上限 10 个（MAX_TAGS），单个标签长度上限 20 字符（MAX_TAG_LEN）；set_tags 自动 trim/去重/截断 | `domain/note.rs` set_tags/add_tag |
| INV-020 | LunarMonthly 重复类型的 next_trigger 通过 trait 注入计算：domain 层定义 `CalendarAdapter` trait（接口）并返回 `NextTrigger::External`，application 层提供 `TymeCalendarAdapter` 实现（依赖 tyme4rs）。domain 层不依赖 tyme4rs，但 seam 完整（调用方无需 `repeat_type` 二次判别） | `domain/reminder.rs` CalendarAdapter trait + next_trigger + advance_state + `application/lunar_calendar.rs` TymeCalendarAdapter + `application/reminder_scheduler.rs` fire_reminders |
| INV-028 | `fire_reminders` 接收 trait object（`&dyn ReminderNotifier` + `&dyn CalendarAdapter`）而非 `AppHandle`，使核心逻辑可注入 mock 测试。Tauri AppHandle 仅在 `TauriReminderNotifier` 实现中使用。`fire_reminders_with_deps` 是可测试入口，`fire_reminders` 是 Tauri 入口包装 | `application/reminder_scheduler.rs` ReminderNotifier trait + TauriReminderNotifier + fire_reminders_with_deps |
| INV-021 | 搜索使用 FTS5 trigram tokenizer（支持 CJK 子串匹配）；查询字符数 < 3 时自动回退到 LIKE 模糊匹配（trigram 要求至少 3 字符）；FTS5 虚拟表采用外部内容模式（content=notes）+ 触发器同步，避免数据复制 | `infrastructure/database.rs` FTS5 迁移 + `sqlite_note_repo.rs` search_notes |
| INV-022 | 模板表首次启动检测为空时自动种子 3 个默认模板（空白/会议记录/待办清单）；模板 id 格式 `tpl-{uuid}`；模板 category 固定为 `custom`；模板只支持用户自定义，不预设系统模板 | `infrastructure/database.rs` 默认模板种子 + `domain/template.rs` Template::new |
| INV-023 | 模板必须随 Git 同步：导出到 `sync/templates/{id}.json`，导入时按 `updated_at` 仲裁（last-write-wins），与便签/提醒一致；sync_json_io 的 export_to_json/import_from_json 必须接收 template_repo 参数 | `application/sync_json_io.rs` export_to_json/import_from_json + `application/git_sync.rs` sync/auto_pull_on_startup |
| INV-024 | Git 同步流程必须遵循"先拉后推"：fetch→merge→import→export→commit→push，禁止先 export 再 fetch/merge，确保远程数据先进入本地数据库后再推送 | `application/git_sync.rs` sync 方法 |
| INV-025 | Git merge 必须使用 `--allow-unrelated-histories`；merge 失败后必须检查是否仍有未解决冲突，若有则拒绝 push（返回错误）。注：原有的"删除文件占比 >50% 拒绝推送"检查已移除（LES-027）——在"先拉后推 + merge --allow-unrelated-histories"修复后，diff 显示的删除均为 DB 决定的预期删除（用户主动删除便签后 export 不再写出对应 JSON）。push 仍保留 `--force-with-lease` 作为防他人并发推送的最低安全网。跨设备删除传播问题（墓碑缺失）另行处理 | `application/git_sync.rs` sync/commit_and_push 方法 |
| INV-026 | delete_note 必须关闭窗口：delete_note 命令在删除数据后 SHALL 关闭对应便签窗口（使用 `destroy()`），确保用户看不到已删除的便签。前端保留 `win.destroy()` 作为兜底，但后端关闭是主要路径 | `delete_note` 命令 |
| INV-027 | 便签窗口最小尺寸 200×150：domain 层 `update_window_state` clamp（`width.max(MIN_WIDTH)`, `height.max(MIN_HEIGHT)`）；window_manager 创建窗口时 `.min_inner_size(200, 150)`；前端保存窗口状态时拦截宽<200 或高<150 的异常值 | `domain/note.rs` update_window_state + `application/window_manager.rs` open_note_window + `src/window-state.ts` saveWindowState |
| INV-029 | scheduler 写后副作用事件化（ADR-008）：`reminder_scheduler::fire_reminders_with_deps` 在 `reminder_repo.save(&updated)` 成功后必须 emit `ReminderWritten(Updated)` 事件，由 lib.rs 监听器统一触发 `schedule_recalc` + `schedule_auto_sync`。禁止在 scheduler 内部手动调用 `schedule_recalc`/`schedule_auto_sync`，禁止调用方（commands/tray/shortcut）手动触发 `schedule_recalc` | `application/reminder_scheduler.rs` fire_reminders_with_deps emit + `lib.rs` setup 监听器 |
| INV-030 | Repository trait CQRS 分离（ADR-010）：`NoteRepository` 仅含聚合 CRUD 5 方法，`NoteQuery` 仅含读投影 2 方法；`ReminderRepository` 仅含聚合 CRUD 6 方法，`ReminderQuery` 仅含读投影 3 方法。scheduler 签名要求 `&dyn ReminderQuery`（非 `&dyn ReminderRepository`）表达只读投影语义。查询方法（search_notes/find_activity_by_month/find_due/find_next_due_time/find_by_date_range）禁止挂在 `*Repository` trait 上 | `domain/repositories.rs` 4 个 trait 定义 + `application/reminder_scheduler.rs` 签名 + `application/commands/*` 调用方 |
| INV-031 | Reminder 状态机转换前置校验：`mark_triggered`/`snooze`/`mark_done`/`cancel` 必须校验当前状态，返回 `Result<(), String>` 拒绝禁止转换。允许的源状态：`mark_triggered` 仅 Pending；`snooze` 仅 Pending/Triggered；`mark_done` 仅 Pending/Triggered；`cancel` 仅 Pending/Triggered。终态（Done/Cancelled）拒绝所有转换。`advance_state` 内部调用 `mark_triggered` 用 `expect` 表达契约（仅对 Pending 调用，由 `find_due` WHERE status='pending' 保证）。service 层（snooze_reminder/dismiss_reminder）用 `?` 传播错误，失败时不 save 不 emit | `domain/reminder.rs` 4 个转换方法 + `application/reminder_service.rs` snooze_reminder/dismiss_reminder + `application/reminder_scheduler.rs` advance_state |
| INV-032 | 墓碑机制（软删除）确保跨设备删除传播（LES-028 修复）：① Note/Reminder/Template 领域模型有 `deleted_at: Option<String>` 字段（`#[serde(default, skip_serializing_if = "Option::is_none")]`），`delete()` 同时设 `deleted_at` 和 `updated_at` 为 now（确保 `updated_at == deleted_at`，last-write-wins 仲裁只需比较 `updated_at`）。② Repository trait 提供 `find_by_id_including_deleted`/`find_all_including_deleted`（含墓碑查询，供 sync 使用）+ `physical_delete`（硬删除，仅供墓碑清理）。③ 业务查询（`find_by_id`/`find_all`/`find_archived`/`search_notes`/`find_due` 等）默认过滤墓碑。④ `sync_json_io::import_from_json` 用 `find_by_id_including_deleted` 让本地墓碑的 `updated_at` 参与 last-write-wins 仲裁（远程墓碑更新→传播删除；远程非墓碑更新→复活本地墓碑）。⑤ `sync_json_io::export_to_json` 用 `find_all_including_deleted` 写出墓碑 JSON（含 `deleted_at` 字段），让其他设备感知删除。仲裁规则不变（`item.updated_at > existing.updated_at` 时 save，INV-011）。墓碑清理（`sync_tombstone_cleanup::cleanup_old_tombstones`，跨三类合计超 50 条时物理删除最老的，在 `git_sync::sync` 阶段 4.5 调用）已实施 | `domain/note.rs` + `domain/reminder.rs` + `domain/template.rs` delete()/deleted_at + `domain/repositories.rs` `*_including_deleted`/`physical_delete` + `domain/mock_repo.rs` + `infrastructure/sqlite_*_repo.rs` + `application/note_service.rs`/`reminder_service.rs`/`template_service.rs` delete 走 domain delete() + save 软删除 + `application/sync_json_io.rs` import/export + `infrastructure/database.rs` deleted_at 列迁移 |

### 已知策略缺口

无（提醒导入已遵循 last-write-wins，与便签导入逻辑一致）

---

## 禁止事项

### 架构禁止

- 核心层（domain）禁止出现 tauri/rusqlite/tokio 等技术框架代码；serde/uuid/chrono 作为值对象工具库允许
- 核心层内禁止互相依赖
- 禁止循环依赖
- 禁止在 `application/commands` 模块之外定义 `#[tauri::command]`
- 禁止用 `emit_to` 向正在初始化的窗口同步发送事件（死锁）

### 设计禁止

- 禁止为未知变化提前抽象（YAGNI）
- 禁止因为"未来可能"而创建 Factory/Strategy/Registry
- 禁止一步到位抽象（只有一种实现时直接实现）
- 禁止在前端通过 `listen` 事件触发窗口创建（前端关闭后无法接收事件）
- 禁止仓储层提供 partial update 方法（所有写入经 domain 方法 + save，NoteRepository 和 ReminderRepository 均适用）
- 禁止 `reminder_scheduler`/`note_service`/`commands/*` 等 application 层模块直接操作窗口属性（`app.get_webview_window().destroy()`/`set_always_on_top()`/`set_focus()`/`emit()` 等），必须委托 `window_manager`（`close_note_window`/`set_note_pinned`/`restore_note_on_top`/`focus_note_window_and_emit` 等）
- 禁止在 Windows 上执行子进程时不设 CREATE_NO_WINDOW 标志（会导致控制台窗口弹出）
- 禁止子进程调用不设 `stdin(Stdio::null())`（可能导致交互式提示挂起进程）
- 禁止在 domain 层结构体中保留无业务逻辑使用的字段（YAGNI 原则）
- 禁止在 `*Repository` trait 上挂读投影方法（`search_notes`/`find_activity_by_month`/`find_due`/`find_next_due_time`/`find_by_date_range`），必须挂在对应的 `*Query` trait 上（ADR-010）
- 禁止在 `commands/*`/`tray_manager`/`shortcut_manager`/`reminder_scheduler` 等调用方手动调用 `schedule_auto_sync` 或 `schedule_recalc`，必须由 service/scheduler emit 事件，lib.rs 监听器统一触发（ADR-007 + ADR-008）
- 禁止 `tray_manager`/`shortcut_manager` 通过 `window_manager` 操作 Hub 窗口，必须委托 `hub_window_manager`（ADR-009）
- 禁止在 `ai-todo-sort`/`ai-rewrite`/`ai-sniff`/`context-menu` 等模块内联 `api.getAiConfig()` 或重复 try/catch + toast 模式，必须经 `ai-client.ts` 的 `isAiConfigured`/`getAiConfigCached`/`runAi`（前端 AI 调用统一入口）
- 禁止重建已删除的 `src/utils.ts`/`src/note-style.ts`，共享逻辑按职责拆到 `colors.ts`/`datetime.ts`/`toast.ts`/`html.ts` 等独立 module
- 禁止用多个 `pub fn xxx() -> &'static str` 薄包装函数实现 i18n 文本表（浅模块反模式），必须用 `LocaleText` 常量表 + `.get()` 方法（`locale_manager` 模式）
- 禁止在 `ai_commands`/`reminder_parser` 等模块内联手写 JSON 片段提取（`find('{') + rfind('}')`/`find('[') + rfind(']')`），必须委托 `application::json_extract::extract_object`/`extract_array`（流式解析精确边界，正确处理字符串值内含定界符场景）
- 禁止函数职责错位（函数放在与其业务概念不对应的模块），如 `formatNoteTime`（时间格式化）放在 `colors.ts`（颜色模块）；函数必须按业务概念归属到对应模块

### 编码禁止

- 禁止吞掉异常（空 catch）
- 禁止在核心层使用技术异常类型
- 禁止用 `SELECT *` 查询（用显式列名 `SELECT_COLS` 常量）
- 禁止将 SQLite 二进制文件作为 Git 同步对象

---

## 项目约束

### 人员约束

单人开发。

### 技术约束

- 后端：Rust 2021 + Tauri 2.0
- 前端：TypeScript + Vite 5 + 原生 HTML（无框架）
- 数据库：SQLite (rusqlite 0.31, bundled, WAL 模式)
- 异步运行时：tokio (full features)
- 构建工具：tauri-cli 2.0 + vite + tsc
- **图片宽度语法约束**：`extract_image_filenames` 函数 SHALL 将 `{` 作为文件名终止符，以兼容 `img:filename{width=N}` 语法。文件名提取正则为 `img:([^{}\s)]+)`，`{` 后的 `width=N}` 为宽度参数，不属于文件名

### 环境约束

- 操作系统：Windows
- 依赖系统安装的 git 可执行文件（PATH 可访问）
- 依赖 WebView2 运行时
- 数据目录：exe 同级 `data/` 文件夹

### 已知限制

- **tao 0.35.3 Windows 偶发崩溃**：`flush_paint_messages` 断言失败（`event_loop.rs:2344`），点击设置中心等窗口操作时偶现。这是 tao 上游库的已知 bug，当前 tao 0.35.3 已是最新版（Tauri 2.11.5 依赖），上游尚未修复。无法通过业务代码或升级解决，等待 tao 发布修复版本。
- **CI/CD**：GitHub Actions workflow（`.github/workflows/release.yml`）在 push tag `v*` 时触发，自动构建 NSIS 安装包并发布到 GitHub Release。仅支持 Windows 平台。

### 发布流程

发版步骤（CI/CD 自动构建发布）：

1. 修改版本号（三处同步）：
   - `src-tauri/tauri.conf.json` → `version`
   - `src-tauri/Cargo.toml` → `version`
   - `package.json` → `version`
2. 提交并打 tag：`git commit && git tag v0.x.0 && git push origin main --tags`
3. CI 自动构建 NSIS 安装包并发布到 GitHub Release

### 编码规范

- Rust：遵循 `cargo fmt` + `cargo clippy`
- TypeScript：遵循项目现有风格
- 命名：Rust 使用 snake_case，TypeScript 使用 camelCase
- DB 列名读取：强制用列名而非索引

---

## 测试约束

| 模块类型 | 覆盖率要求 | 重点测试 |
|----------|-----------|----------|
| 核心层 (domain) | >= 80% | 业务规则、不变量、状态流转 |
| 技术层 (infrastructure) | >= 70% | 端口实现、错误处理、边界条件 |
| 策略层 (application) | >= 60% | 命令编排、调度策略 |

测试原则：

- Arrange-Act-Assert 模式组织测试代码
- 每个测试只验证一个行为
- 测试之间互不依赖，可独立运行
- 优先测试核心层业务规则
- infrastructure 测试用 `:memory:` 内存数据库

状态机测试要求：

- 每个状态转换必须有独立测试用例
- 每个禁止转换必须有测试验证其被拒绝
- 修改状态机后必须全量运行状态机测试

---

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-08 | 初始版本 | — | — |
| 2026-07-09 | 按模板重构，补充设计原则、架构状态、测试约束 | — | — |
| 2026-07-09 | 修复 INV-006 违规、提醒导入策略缺口、sync_notes 改 async | — | #REFACTOR-001 |
| 2026-07-09 | 业务逻辑下沉：reset_for_next_trigger、snooze/dismiss 通过 domain 方法 | — | #REFACTOR-002 |
| 2026-07-09 | 删除 NoteRepository 4 个 partial update 方法，所有写入经 domain 方法 + save；修复 INV-001 仓储绕过漏洞；更新 INV-001/INV-003 检查位置 | — | #REFACTOR-007 |
| 2026-07-10 | 补全 schedule_auto_sync 调用链：12 个写操作命令追加调用；新增写操作命令必须触发自动同步规则；更新 INV-013 检查位置 | — | #REFACTOR-012 |
| 2026-07-11 | AppState 新增 ShortcutManager 字段；提醒触发已存在窗口通过 emit_to 发送 reminder-triggered 事件 | — | #FEAT-001 |
| 2026-07-13 | 7 个同步命令改 async；删除 ReminderRepository update_status/snooze 方法；reminder_service 窗口操作委托 window_manager；新增 INV-014/INV-015；新增模块边界和禁止事项 | — | #REFACTOR-013 |
| 2026-07-14 | 删除 Reminder.repeat_config 字段（YAGNI）；新增 INV-016（stdin null）/INV-017（分支名验证）；git 子进程全部加 stdin(Stdio::null()) | — | #REFACTOR-014 |
| 2026-07-15 | 迭代一 v0.2.0：Note 新增 tags 字段 + 标签管理能力；NoteRepository 新增 search_notes；新增 search_notes/update_note_tags 命令；新增 INV-018（tags serde default）/INV-019（标签数量/长度限制） | — | #FEAT-002 同步更新 boundaries.md |
| 2026-07-15 | 迭代三 v0.4.0：Monthly 改精确日历月；新增 LunarMonthly 重复类型 + tyme4rs 农历库；新增日历视图；ReminderRepository 新增 find_pending_by_date_range；新增 get_reminders_by_month 命令；新增 INV-020（LunarMonthly 农历计算在 application 层） | — | #FEAT-003 同步更新 boundaries.md/glossary.md |
| 2026-07-15 | 迭代三 v0.4.1：日历视图 7 项增强；find_pending_by_date_range 改为 find_by_date_range（含所有状态）；新增 get_lunar_dates/get_notes_activity_by_month 命令；NoteRepository 新增 find_activity_by_month | — | #FEAT-005 同步更新 boundaries.md |
| 2026-07-17 | 新增已知限制：tao 0.35.3 Windows `flush_paint_messages` 断言失败偶发崩溃（上游未修复）；新增 GitHub Actions CI/CD（tag v* 触发自动构建 NSIS + 发布 Release） | — | #FEAT-009 |
| 2026-07-17 | v0.8.0：删除 NoteColor 枚举（color 改为纯 String）；新增批量操作命令 batch_archive_notes/batch_delete_notes/batch_update_color；flash-window 改为 emit_to 定向发送；启动防重叠 resolve_overlaps；归档便签不触发提醒 | — | #FEAT-010 同步更新 boundaries.md |
| 2026-07-18 | v0.8.1：搜索改用 FTS5 trigram tokenizer + LIKE 短查询回退；新增便签模板能力（Template 领域模型 + TemplateRepository + 4 个命令）；新增 toggle_hub 全局快捷键（Ctrl+Shift+H）；Note 新增 highlight 字段（搜索高亮）；新增 INV-021（FTS5 trigram）/INV-022（模板种子） | — | #FEAT-011 同步更新 boundaries.md/glossary.md |
| 2026-07-18 | 模板 Git 同步：sync_json_io export/import 增加 template_repo 参数 + templates 目录处理；git_sync/note_service/commands/tray_manager/lib.rs 全链路传 template_repo；新增 INV-023（模板必须 Git 同步）；搜索高亮修复（snippet 三列选择 + 选第一个含 `<mark>` 的）；新增 UI 入口：空便签模板快捷条 + 右键菜单"从模板新建" | — | #FEAT-012 同步更新 boundaries.md/glossary.md |
| 2026-07-18 | UI 修复：i18n 命名空间错误修复（tpl 键在 note 命名空间下新增，hub 保留）；右键菜单改为两项并存——「从模板新建便签」+「应用模板到当前便签」（追加到末尾，非破坏性）；模板快捷条 CSS 改为横向单行滚动；应用图标替换为 TIE 字母图标 | — | #FEAT-013 同步更新 boundaries.md/glossary.md |
| 2026-07-18 | Bugfix：Git 同步 unrelated histories 导致远程数据被删除；重构 sync 流程为"先拉后推"（fetch→merge→import→export→commit→push）；merge 添加 --allow-unrelated-histories；push 前安全检查（删除>50%拒绝推送）；新增 INV-024（先拉后推）/INV-025（merge 安全保护） | — | #BUGFIX-001 同步更新 lessons/README.md |
| 2026-07-19 | 新增 INV-026 + 图片宽度语法约束 | AI | v0.8.5 |
| 2026-07-19 | 新增 INV-027（窗口最小尺寸 200×150 三处校验） | AI | v0.8.5 同步更新 flows.md |
| 2026-07-19 | 合并 reminder_service 到 reminder_scheduler（删除 reminder_service.rs）；fire_reminders 成为 reminder_scheduler 的 pub fn；更新模块边界/禁止事项/INV-020 检查位置 | AI | #REFACTOR-015 同步更新 boundaries.md |
| 2026-07-19 | 拆分 commands.rs 为 commands 模块（commands/ 目录 + 7 个子模块）；模块边界第 139 行"application/commands.rs"→"application/commands 模块"；禁止事项第 225 行同步；新增 application/image_service.rs 承载图片处理业务逻辑 | AI | #REFACTOR-017 同步更新 boundaries.md |
| 2026-07-19 | 深化 fire_reminders：domain/reminder.rs 新增 `NextTrigger` enum（None/DateTime/External）+ `CalendarAdapter` trait + `Reminder::advance_state` 方法；删除 `reset_for_next_trigger`（被 advance_state 替代）；application/reminder_scheduler.rs 新增 `ReminderNotifier` trait + `TauriReminderNotifier` 实现 + `fire_reminders_with_deps` 可测试入口；application/lunar_calendar.rs 新增 `TymeCalendarAdapter` impl CalendarAdapter；改写 INV-020（保留"domain 不依赖 tyme4rs"意图，seam 改为 trait 注入）；新增 INV-028（fire_reminders 接收 trait object） | AI | #REFACTOR-019 同步更新 boundaries.md/flows.md |
| 2026-07-19 | Candidate 02 窗口操作统一：window_manager.rs 新增 `close_note_window`/`set_note_pinned`/`restore_note_on_top`/`focus_note_window_and_emit` 4 个 pub fn（封装 destroy/set_always_on_top/set_focus+emit）；note_commands.rs 的 `delete_note`/`batch_delete_notes`/`restore_window_on_top` 和 note_service.rs 的 `update_note_style`/`open_note_with_flag` 5 处直接窗口操作替换为 window_manager 调用；note_commands.rs 移除 `Manager` 导入，note_service.rs 移除 `Manager`/`Emitter` 导入；扩展模块边界规则（"reminder_scheduler"→"application 层所有模块"）和设计禁止事项覆盖范围 | AI | #REFACTOR-020 同步更新 boundaries.md |
| 2026-07-19 | Candidate 03 sync_json_io 泛化：新增私有泛型函数 `export_entity_to_json`（序列化+写文件）和 `import_entity_from_json`（反序列化+last-write-wins 仲裁）；用泛型 + 闭包参数化差异点（id/updated_at/find_by_id/save），消除 Note/Reminder/Template 三种实体的导出/导入三重重复；`export_to_json` 从 33 行降至 21 行，`import_from_json` 从 82 行降至 35 行；`R: ?Sized` 约束支持 `&dyn Trait` 传入；调用处用 turbofish `::<Note, dyn NoteRepository>` 明确泛型参数；未新增 INV（无新业务不变量，纯重构） | AI | #REFACTOR-021 同步更新 boundaries.md |
| 2026-07-19 | Candidate 04 Note 聚合根完善：domain/note.rs 新增 `is_empty()`（title+content 均空，INV-003）和 `is_reminder_eligible()`（!is_archived，归档不触发提醒）2 个业务查询方法；window_manager::restore_all_windows 和 note_service::close_note_if_empty 2 处 `note.title.is_empty() && note.content.is_empty()` 散布判断替换为 `note.is_empty()`；reminder_scheduler::fire_reminders_with_deps 1 处 `note.is_archived` 判断替换为 `!note.is_reminder_eligible()`；新增 6 个 domain 单元测试覆盖 4 种 is_empty 边界 + 2 种 is_reminder_eligible 状态；更新 INV-003 检查位置描述（明确 Note::is_empty 为入口） | AI | #REFACTOR-022 同步更新 boundaries.md |
| 2026-07-19 | Candidate 06 命令层可测试性提升：新建 `application/reminder_service.rs`（CRUD 编排：create/snooze/dismiss/delete，接收 `&dyn ReminderRepository` 无 Tauri 依赖可单测）；reminder_commands.rs 4 处命令改为薄壳（调用 service → 统一副作用 scheduler.schedule_recalc + app.emit + git_sync.schedule_auto_sync）；snooze/dismiss 副作用模式统一（原直接 return save 结果，现改为先 service 完成业务再执行副作用）；application/mod.rs 新增 `pub mod reminder_service`；模块边界规则新增"业务编排逻辑必须下沉到 *_service 模块，#[tauri::command] 仅作为薄壳"；新增 9 个 service 单元测试；198 个测试全部通过 | AI | #REFACTOR-024 同步更新 boundaries.md |
| 2026-07-20 | Candidate 10 知识库不一致修正：删除 INV-002（NoteColor 枚举已于 v0.8.0 #FEAT-010 删除，引用位置 `NoteColor::from_str` 失效；后端不校验颜色值，由前端 UI 限制；DB schema `color TEXT NOT NULL DEFAULT 'amber'` 仅设默认值非业务约束）；INV-002 编号废弃不复用 | AI | #REFACTOR-027 |
| 2026-07-20 | Candidate 11+12 note_commands 9 个命令下沉 note_service：与 Candidate 06（reminder_service）同范式，note_service.rs 新增 6 个单字段更新函数 + 4 个批量函数（batch_* 返回 `Vec<String>` 成功 id 列表供命令层 emit）；note_commands.rs 9 个命令改为薄壳；image_service::cleanup_removed_images 从 update_note_content/batch_delete 命令下沉到 service；新增 24 个 service 单测；消除"reminder 走 service、note 不走"的范式不对称 | AI | #REFACTOR-028 同步更新 boundaries.md |
| 2026-07-20 | Candidate 13 tray_manager 三处重复 + 约束违规：13a 抽取 `build_tray_menu`；13b git_sync 新增 `sync_with_notification` 统一 sync+通知；13c window_manager 新增 `open_or_focus_hub`+`create_hub_window`，tray 委托消除内联 WebviewWindowBuilder 违规；附带修复 INV-013 违规（handle_new_note 漏 schedule_auto_sync） | AI | #REFACTOR-029 同步更新 boundaries.md |
| 2026-07-21 | 前端模块化拆分（AI 可读性）：新增"前端模块边界（AI 可读性约束）"小节 9 条（入口仅编排/文件名=业务名/JSDoc 三段头/单向依赖无环/callback 模式破环/state 就近/side-effect 模块/函数<100 行/共享样式提取）；INV-027 引用位置 `src/main.ts` saveWindowState 更正为 `src/window-state.ts` saveWindowState（函数已随 window-state 模块拆分迁移） | AI | #REFACTOR-034 同步更新 boundaries.md |
| 2026-07-21 | 新增"写操作事件化（ADR-007）"约束（service 层 emit 事件/依赖 EventPublisher trait/监听器集中注册/template 经 template_service）；更新 INV-013 检查位置（从 commands 调用方迁移到 service emit + lib.rs 监听器） | AI | #REFACTOR-036 同步更新 ADR-007/glossary.md/boundaries.md/lessons/README.md |
| 2026-07-21 | 架构深化第二轮：新增 3 条模块边界约束——"命令层薄壳化"（业务规则/参数解析/第三方库调用下沉到 service/validator/封装模块，含 AI 校验/period 解析/农历计算 3 项具体应用）、"locality 下沉"（相关副作用与触发它的写操作在同一 service 模块，含 delete_note 图片清理 1 项具体应用）、"跨模块常量集中"（CREATE_NO_WINDOW 等技术常量 pub const 集中所有者模块，禁止 magic number 散布）；未新增 INV（纯架构深化，无新业务不变量）；261 个测试全部通过 | AI | #REFACTOR-037 同步更新 boundaries.md |
| 2026-07-21 | 架构深化第三轮（6 候选合并提交）：候选1 hub.ts IPC seam 修复；候选2 window_manager 拆分为 `hub_window_manager`+`window_overlap_resolver`+`window_manager`（ADR-009）；候选3 前端 `utils.ts`+`note-style.ts` 拆为 `colors.ts`/`datetime.ts`/`toast.ts`/`html.ts` 4 个独立 module（14 处调用方同步）；候选4 新建 `src/ai-client.ts` 统一 AI 配置缓存+调用包装（`isAiConfigured`/`getAiConfigCached`/`runAi`，4 个 AI module 改为薄调用）；候选5 `sync_commands` 拆为 `sync_commands`+`shortcut_commands`+`locale_commands`+`system_commands` 4 个子模块（commands/mod.rs glob 重导出 9 个子模块）；候选6 Repository trait CQRS 拆分（ADR-010，4 个 trait + sqlite/mock 双 impl + AppState 4 仓储字段 + scheduler 依赖 ReminderQuery）；新增 INV-029（scheduler 写后副作用事件化，ADR-008）+ INV-030（Repository trait CQRS 分离）；更新 INV-013 检查位置（增加 scheduler emit）；前端模块边界更新（note-style→colors，utils 拆分，ai-client 统一入口）；新增设计禁止 5 条（CQRS trait 分离/scheduler 副作用事件化/Hub 窗口委托/AI 调用统一入口/禁止重建 utils.ts）；264 个测试全部通过 | AI | #REFACTOR-038 同步更新 ADR-008/009/010/glossary.md/boundaries.md/lessons/README.md |
| 2026-07-22 | Reminder 状态机补齐转换合法性校验：新增 INV-031（mark_triggered/snooze/mark_done/cancel 返回 Result 校验源状态，终态拒绝所有转换，advance_state 内部 expect 表达契约，service 层用 `?` 传播错误）；满足 constraints.md 第 363-367 行"每个禁止转换必须有测试验证其被拒绝"硬性要求；新增 13 个状态转换测试（合法 3 + 禁止 10）；278 个测试全部通过 | AI | #REFACTOR-039 同步更新 flows.md |
| 2026-07-22 | find_due 消除 SQL 重新实现 is_due 领域规则：SqliteReminderRepository::find_due 从"SQL WHERE 实现完整 is_due 逻辑"改为"SQL 只筛 status='pending'，Rust 侧调 is_due 过滤"，与 InMemoryReminderRepository 实现对齐；is_due 成为 INV-008 单一真相源；新增 2 个 snoozed 测试（到期/未到期）；280 个测试全部通过 | AI | #REFACTOR-040 同步更新 lessons/README.md |
| 2026-07-22 | 架构深化第四轮（7 候选合并）：候选1 mock/sqlite 保真度缺口修复（mock delete 幂等化 + find_all/find_by_note_id 加排序 + service 层 delete 加存在性守卫 + 7 个一致性测试，LES-023）；候选2 image_service 反向依赖修复（image_dir 委托 application::paths::data_dir_path 正向依赖）；候选3 restore_all_windows 委托 note_service::delete_note（消除漏 emit NoteWritten(Deleted) 事件）；候选4 Reminder::effective_time() 单一真相源（is_due/find_next_due_time/find_by_date_range 统一委托，EFFECTIVE_TIME_EXPR SQL 常量）；候选5 手写 JSON 切片改用 serde_json 流式解析（sync_json_io::extract_updated_at + reminder_parser::extract_json + ai_commands::extract_json_array 3 处，消除字符串值内含 `{`/`}`/`[`/`]` 误切边界 bug）；候选6 create_note 抽取 create_note_with_deps 脱离 AppHandle 可单测（仿 INV-028 模式）；候选7a 前端颜色表 purple 缺失 bug 修复 + BATCH_COLORS 统一引用；更新 INV-013 检查位置（补充 service 层 delete 存在性守卫）；新增"mock/sqlite 仓储保真度（LES-023）"约束小节（mock 以 sqlite 为基准对齐/delete 幂等/排序对齐/一致性测试/存在性守卫/setup 漂移处理）；308 个测试全部通过 | AI | #REFACTOR-044 同步更新 lessons/README.md |
| 2026-07-24 | 架构深化第五轮（3 候选合并）：候选6 JSON 提取逻辑两处独立实现合并——新建 `application/json_extract.rs`（`extract_object`/`extract_array` 用 `Deserializer::into_iter::<IgnoredAny>` 流式解析 + `byte_offset` 精确边界），`ai_commands::extract_json_array` 和 `reminder_parser::extract_json` 删除并委托本模块，7 个测试迁移；候选7 `formatNoteTime` 职责错位修正——从 `colors.ts`（颜色模块）迁到 `datetime.ts`（日期时间工具），`note-renderer.ts` import 更新；候选8 `locale_manager` 浅模块深化——9 个 `pub fn menu_xxx()/notify_xxx()` 薄包装函数改为 `LocaleText { zh, en }` 常量表 + `.get()` 方法，10 处调用方（tray_manager/hub_window_manager/git_sync）同步；新增模块边界 3 条（浅模块深化常量表/JSON 提取单点归属/函数职责按业务概念归属）+ 设计禁止 3 条；321 个测试全部通过；未新增 INV（纯架构深化，无新业务不变量） | AI | #REFACTOR-045 同步更新 lessons/README.md/boundaries.md |
| 2026-07-26 | 知识库健康度检查修正 + bug 修复同步：INV-003 检查位置补充前端关闭按钮先 await 保存再 close 说明（LES-025 竞态修复）；INV-008 检查位置补充 ISO 8601 字符串比较格式一致性约束（LES-026 毫秒边界修复，`now` 与 `remind_at` 均不带毫秒，前端 `setSeconds(0, 0)`）；未新增 INV（仅补充已有 INV 的检查位置描述） | AI | #BUGFIX-002 同步更新 lessons/README.md/boundaries.md |
| 2026-08-03 | 移除 INV-025 的"删除文件占比 >50% 拒绝推送"检查（LES-027）：在 BUGFIX-001 引入"先拉后推 + merge --allow-unrelated-histories"修复后，push 前 diff 显示的删除均为 DB 决定的预期删除（用户主动删除便签 → export 不再写出对应 JSON），50% 检查从"防误覆盖"退化为"拦批量删除"，误报率远高于真实防护价值；保留 merge --allow-unrelated-histories + 未解决冲突拒绝 push + --force-with-lease 三重防护；跨设备删除传播问题（墓碑缺失）另行处理 | AI | #BUGFIX-003 同步更新 lessons/README.md |
| 2026-08-03 | 新增 INV-032（墓碑机制软删除确保跨设备删除传播，LES-028 修复）：文档化代码已引用但未在 constraints.md 登记的 INV-032 不变量（历史不一致修正）。涵盖 5 项要点：① 领域模型 deleted_at 字段 + delete() 同时设 updated_at ② Repository `*_including_deleted` + `physical_delete` ③ 业务查询默认过滤墓碑 ④ sync import 用 find_by_id_including_deleted 让墓碑参与 last-write-wins 仲裁 ⑤ sync export 用 find_all_including_deleted 写出墓碑 JSON。sync_json_io import/export 是本次完成的部分（Task 10+11），其余部分（领域模型/Repository/sqlite/mock/DB migration）此前已完成，墓碑清理另建任务 | AI | #TOMBSTONE-001 同步更新 lessons/README.md |
| 2026-08-03 | INV-032 检查位置补充 service 层：reminder_service::delete_reminder 改软删除（domain delete() + save），与 note_service/template_service 对齐 | AI | #TOMBSTONE-001 |
| 2026-08-03 | 墓碑机制全量落地：INV-011 修订（仲裁含墓碑）+ INV-032 墓碑清理已实施（sync_tombstone_cleanup 阈值 50） | AI | #FEAT-TOMBSTONE 同步更新 lessons/README.md + boundaries.md + glossary.md |
