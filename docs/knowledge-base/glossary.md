# 术语表

> **TL;DR**: 核心术语：Note（便签聚合根）、Reminder（提醒实体）、AppState（应用全局状态）。⚠️ 能力契约 ≠ 接口契约：前者是核心层定义的业务能力接口，后者是对外暴露的 API。

---

## 添加规则

遇到以下情况必须添加术语：

- 新增业务概念或领域术语
- 存在中英文对照需求
- 团队内对同一概念有不同叫法
- 缩写首次出现

---

## A

### AppState

应用全局状态，在 setup 中创建并通过 Tauri State 管理器注入到各命令。包含 `note_repo`、`note_query`、`reminder_repo`、`reminder_query`、`template_repo`、`git_sync`、`shortcut_manager`、`scheduler`、`event_bus` 九个成员。是组合根的具体实现。

---

## B

### 便签 (Note)

桌面悬浮窗口形式的快捷记录。每张便签有独立窗口，包含标题、内容、颜色、透明度、窗口位置等属性。是系统的聚合根。

### 归档 (Archive)

将便签从桌面移除但保留数据的状态切换。归档后便签不在桌面显示，但可在设置中心查看和恢复。`is_archived` 字段控制。

### 边界 (Boundary)

系统与外部的交互边界。详见 `boundaries.md`。

---

## C

### 能力契约 (Capability Contract)

核心层定义的业务能力接口，表达"能做什么"。示例：`NoteRepository`（便签存储能力）、`ReminderRepository`（提醒存储能力）。

### 接口契约 (API Contract)

对外暴露的 API 接口，表达"如何调用"。示例：`invoke('archive_note', { id })`（归档便签接口）。

> 能力契约是内部的、面向领域的；接口契约是外部的、面向调用方的。

### 组合根 (Composition Root)

`lib.rs` setup 函数中构造具体仓储实现并注入 AppState 的位置。是唯一允许 application 层持有 infrastructure 具体实现的地方。

---

## D

### 调度器 (Scheduler)

`reminder_scheduler` 模块，事件驱动调度。启动后等 5 秒，使用单定时器 + `Arc<Notify>` 机制，提醒数据变更时通知调度器重新计算下次触发时间，触发通知 + 弹出便签窗口。

### 防抖 (Debounce)

`schedule_auto_sync` 使用 30 秒防抖策略，多次触发只执行最后一次。通过 `Mutex<Instant>` 记录最后触发时间。

### 领域事件 (DomainEvent)

后端内部事件总线传递的写操作完成信号（ADR-007 + ADR-008 扩展）。按实体 + 操作类型粒度：`NoteWritten`/`ReminderWritten`/`TemplateWritten` 携带 `WriteAction`（`Created`/`Updated`/`Deleted`）+ 实体 id。由 `note_service`/`reminder_service`/`template_service` 在写操作完成后 emit，**以及 `reminder_scheduler::fire_reminders_with_deps` 在 save 推进状态后 emit `ReminderWritten(Updated)`**（ADR-008 扩展，覆盖 scheduler 系统自动写场景）。监听器（`lib.rs` setup 注册两条）接收事件并触发副作用：所有 `DomainEvent` → `schedule_auto_sync`；`ReminderWritten` → `schedule_recalc`。纯后端事件，不经过 Tauri emit/listen，不广播给前端。

---

## E

### 事件总线 (EventBus)

`EventPublisher` trait 的当前同步实现（`application/event_bus.rs`）。内部用 `Arc<Mutex<Vec<Box<dyn Fn(&DomainEvent) + Send + Sync>>>>` 存储 handler 列表，`emit` 时同步遍历调用所有 handler，`subscribe` 注册新 handler。监听器在 `lib.rs` setup 中通过 `event_bus.subscribe(Box::new(|event| { ... }))` 注册。

### 事件发布者 (EventPublisher)

service 层依赖的事件发布抽象 trait（依赖倒置原则）。service 接收 `&dyn EventPublisher` 参数（trait object，可 mock 测试，可替换实现）。当前唯一实现是同步的 `EventBus`；未来可新增 `ChannelPublisher`（基于 `tokio::sync::broadcast`）实现异步事件，service 签名零改动。是 ADR-007 的核心 seam。

---

## F

### 闪烁 (Flash Window)

窗口已存在时被聚焦的视觉提示。后端临时 `set_always_on_top(true)` 300ms，同时 emit `flash-window` 事件触发前端蓝色边框动画。

---

## G

### 贪睡 (Snooze)

提醒触发后延后再次提醒的功能。设置 `snoozed_until` 字段，调度器在贪睡截止时间后再次触发。状态保持 Pending。

---

## H

### Hub 窗口

设置中心窗口，原生标题栏，640x520。包含便签管理、同步设置、关于页面。通过托盘菜单或 `hub.html` 加载。

---

## J

### 聚合根 (Aggregate Root)

领域驱动设计概念。Note 是聚合根，Reminder 是其关联实体。外部只能通过 Note 访问 Reminder，删除 Note 时级联删除关联 Reminder。

---

## P

### 标签 (Tag)

便签的自定义分类标记。每张便签最多 10 个标签，单标签最长 20 字符。存储为 JSON 数组（`Vec<String>`），在 SQLite 中以 TEXT 列 `tags` 存储，在 JSON 同步文件中随 Note 序列化。标签支持去重和自动 trim。

---

### 搜索 (Search)

跨活跃+归档便签的全文搜索能力。使用 SQLite FTS5 虚拟表（外部内容模式 `content=notes`）+ trigram tokenizer 支持 CJK 子串匹配，查询字符数 < 3 时自动回退到 LIKE 模糊匹配（trigram 要求至少 3 字符）。搜索结果通过 FTS5 `snippet()` 函数生成带 `<mark>` 标签的高亮片段，存储在 `Note.highlight` 字段。结果按置顶优先 + FTS5 rank 排序。

---

### 置顶 (Pin)

将便签窗口设为始终置顶（`always_on_top`）。`is_pinned` 字段控制，通过 `set_always_on_top` 同步到窗口。

---

## R

### 提醒 (Reminder)

关联到便签的时间触发器。支持一次性（Once）、每日（Daily）、每周（Weekly）、每月（Monthly）、农历每月（LunarMonthly）五种重复类型。状态机：Pending → Triggered → Done/Cancelled；4 个转换方法（mark_triggered/snooze/mark_done/cancel）返回 `Result<(), String>` 表达转换合法性（INV-031）——终态 Done/Cancelled 拒绝所有转换，Triggered 拒绝重复 mark_triggered；Triggered → Pending via snooze 合法（用户主动延后）。Monthly 按精确日历月计算（月末溢出取目标月最后一天）；LunarMonthly 按农历月计算（domain 层返回 None，由 application 层调用 tyme4rs 库计算）。

### 仓储 trait (Repository Trait)

domain 层定义的数据访问能力契约（`NoteRepository`/`ReminderRepository`），infrastructure 层提供 SQLite 实现。依赖倒置原则的体现。

### CQRS 风味拆分 (CQRS-flavored Repository Split)

Repository trait 按聚合 CRUD 与读投影分离的拆分模式（ADR-010）。`NoteRepository`（5 方法：save/find_by_id/find_all/delete/find_archived）与 `NoteQuery`（2 方法：search_notes/find_activity_by_month）独立 trait；`ReminderRepository`（6 方法）与 `ReminderQuery`（3 方法：find_due/find_next_due_time/find_by_date_range）独立 trait。实现层 `SqliteXxxRepository` 同时 impl 两个 trait（双 impl 块）。不引入完整 CQRS 框架（无单独 Command/Query 模型），仅 trait 接口分离。目的：缩小 mock surface，让 scheduler 签名表达"只读投影"语义，为未来读模型优化（缓存/只读副本）留路径。

### NoteQuery

Note 读投影查询 trait（CQRS 拆分，ADR-010）。承载 `search_notes`（FTS5 搜索）+ `find_activity_by_month`（日历视图活动查询）2 个方法。`SqliteNoteRepository` 同时 impl `NoteRepository` + `NoteQuery`。`AppState.note_query: Box<dyn NoteQuery>` 持有，`note_commands::search_notes` / `reminder_commands::find_activity_by_month` 通过此字段调用。

### ReminderQuery

Reminder 读投影查询 trait（CQRS 拆分，ADR-010）。承载 `find_due`（scheduler 查到期）+ `find_next_due_time`（scheduler 计算下次）+ `find_by_date_range`（日历视图时间范围）3 个方法。`SqliteReminderRepository` 同时 impl `ReminderRepository` + `ReminderQuery`。`AppState.reminder_query: Box<dyn ReminderQuery>` 持有，`reminder_scheduler` 签名要求 `&dyn ReminderQuery`（非 `&dyn ReminderRepository`）表达"scheduler 只依赖读投影"语义。

### ai-client (前端 AI 调用统一入口)

前端 AI 调用的统一封装模块（`src/ai-client.ts`，ADR-009 同批次）。提供三个公开 API：`isAiConfigured()`（带 5 秒缓存的"AI 是否已配置"查询，避免右键菜单每次打开都发起 IPC）+ `getAiConfigCached()`（带缓存的完整配置读取，用于 ai-sniff 等需要 sniff_enabled 等字段的场景）+ `runAi<T>(op, opts)`（统一包装 AI 调用，处理 loading/success/error toast）。配置缓存通过 `ai-config-changed` 事件自动清空（Hub 保存配置后立即生效）。例外：`ai-settings.ts` 配置页本身每次刷新表单需读最新值，仍直接调用 `api.getAiConfig`。

### runAi

`ai-client.ts` 提供的 AI 调用包装函数。签名 `runAi<T>(op: () => Promise<T>, opts: RunAiOptions): Promise<T | undefined>`。流程：可选 loading toast → 执行 op → 成功显示 successMsg 返回结果 / 失败 console.error + 显示 errorPrefix toast 返回 undefined。`RunAiOptions` 含 `loadingMsg`/`successMsg`/`errorPrefix`/`silentError` 4 个可选字段。被 `ai-todo-sort`/`ai-rewrite` 复用，消除 4 个 AI module 的 try/catch + toast 重复模式。

---

## S

### 双存储架构 (Dual Storage)

SQLite 作为本地运行时存储（事务/并发安全），JSON 文件作为 Git 同步传输载体（文本可合并）。`data/sync/` 为 Git 仓库根，每实体一个独立 JSON 文件。

---

## T

### Tombstone（墓碑）

软删除标记的实体记录。当用户删除便签/提醒/模板时，不执行物理删除（DELETE FROM），而是通过 `delete()` 方法设置 `deleted_at` 字段（同时更新 `updated_at`）。墓碑记录参与 Git 同步的 last-write-wins 仲裁，确保删除操作能跨设备传播。业务查询默认过滤墓碑（`WHERE deleted_at IS NULL`），只有同步专用的 `find_*_including_deleted` 方法返回含墓碑记录。墓碑清理在超过 50 条阈值时物理删除最老的（INV-032）。

---

### 便签模板 (Template)

用户自定义的便签内容模板，支持从模板一键创建便签。存储在 SQLite `templates` 表，首次启动为空时自动种子 3 个默认模板（空白/会议记录/待办清单）。模板 id 格式 `tpl-{uuid}`，category 固定为 `custom`。模板随 Git 同步（导出到 `sync/templates/*.json`，按 `updated_at` 仲裁 last-write-wins）。UI 入口三处：设置中心模板管理弹窗（CRUD）、便签右键菜单"从模板新建"（创建新便签）、空便签编辑区顶部模板快捷条（一键填充当前便签）。

---

### 图片宽度语法 (Image Width Syntax)

`img:filename{width=N}` — 便签内容中指定图片显示宽度的 Markdown 扩展语法。`filename` 为图片文件名，`N` 为像素宽度值。渲染时图片以指定宽度显示，保持原始宽高比。若省略 `{width=N}` 则按容器宽度自适应。

---

### 图片拖拽调整 (Image Drag Resize)

便签查看模式下，hover 图片时右下角出现拖拽手柄，用户可水平拖拽调整图片宽度。松开后宽度值以 `img:filename{width=N}` 格式写回 Markdown 内容并持久化。

---

### trigram tokenizer

SQLite FTS5 的一种分词器，将文本按 3 字符滑动窗口生成 trigram 索引。支持任意语言（包括 CJK 中文）的子串匹配，适合便签搜索场景。要求查询至少 3 个字符才能生成 trigram，因此短查询（< 3 字符）需回退到 LIKE 模糊匹配。

---

## W

### 便签窗口 (Note Window)

每张便签的独立窗口，label 格式 `note-{uuid}`。无装饰透明窗口，由前端自绘标题栏。通过 `index.html` 加载。

---

## 缩写表

| 缩写 | 全称 | 说明 |
|------|------|------|
| ADR | Architecture Decision Record | 架构决策记录 |
| IPC | Inter-Process Communication | 进程间通信（Tauri 前后端通信） |
| WAL | Write-Ahead Logging | SQLite 日志模式 |
| YAGNI | You Aren't Gonna Need It | 避免过度设计原则 |

---

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-08 | 初始版本 | — | — |
| 2026-07-09 | 按模板重构，改为字母排序，补充缩写表 | — | — |
| 2026-07-13 | 更新 AppState（5 成员）和调度器（事件驱动）词条 | — | #REFACTOR-008 |
| 2026-07-15 | 新增标签（Tag）和搜索（Search）术语 | — | #FEAT-002 |
| 2026-07-18 | 更新搜索术语（LIKE → FTS5 trigram + snippet 高亮）；新增便签模板（Template）和 trigram tokenizer 术语 | — | #FEAT-011 同步更新 constraints.md/boundaries.md |
| 2026-07-18 | 更新便签模板术语：模板随 Git 同步（templates 目录 + updated_at 仲裁）；新增三处 UI 入口（设置中心/右键菜单/空便签快捷条） | — | #FEAT-012 同步更新 constraints.md/boundaries.md |
| 2026-07-18 | 右键菜单改为两项并存：「从模板新建便签」+「应用模板到当前便签」（追加到末尾，非破坏性）；模板快捷条多模板时横向单行滚动 | — | #FEAT-013 同步更新 constraints.md/boundaries.md |
| 2026-07-19 | 新增图片宽度语法、图片拖拽调整术语 | AI | v0.8.5 |
| 2026-07-21 | 新增领域事件（DomainEvent）、事件总线（EventBus）、事件发布者（EventPublisher）术语；更新 AppState 词条（新增 template_repo/event_bus 成员，5→7 个） | AI | #REFACTOR-036 同步更新 ADR-007/constraints.md/boundaries.md |
| 2026-07-21 | 新增 CQRS 风味拆分、NoteQuery、ReminderQuery、ai-client、runAi 术语；更新 AppState 词条（新增 note_query/reminder_query 成员，7→9 个）；更新 DomainEvent 词条（ADR-008 扩展：scheduler 也 emit ReminderWritten，监听器两条） | AI | #REFACTOR-038 同步更新 ADR-008/010/constraints.md/boundaries.md/lessons/README.md |
| 2026-07-22 | 更新提醒（Reminder）词条：4 个转换方法（mark_triggered/snooze/mark_done/cancel）返回 `Result<(), String>` 表达转换合法性（INV-031）——终态 Done/Cancelled 拒绝所有转换，Triggered 拒绝重复 mark_triggered；Triggered → Pending via snooze 合法（用户主动延后） | AI | #REFACTOR-039 同步更新 constraints.md/flows.md/boundaries.md/lessons/README.md |
| 2026-08-03 | 新增 Tombstone（墓碑）术语：软删除机制文档化（INV-032） | AI | #FEAT-TOMBSTONE 同步更新 constraints.md + lessons/README.md + boundaries.md |
