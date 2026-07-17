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
- 所有 `#[tauri::command]` 集中在 `application/commands.rs`
- 窗口/托盘/快捷键/调度器各自独立模块，互不直接调用
- `reminder_service` 的窗口操作（显示/聚焦/闪烁）必须委托 `window_manager`，禁止直接操作窗口属性
- 闪烁提示逻辑（临时置顶 300ms + 恢复）统一由 `window_manager::flash_window` 提供，禁止在其他模块重复实现

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
| INV-002 | 颜色未知值降级为 Amber | `NoteColor::from_str` |
| INV-003 | 空便签窗口关闭时从 DB 删除（title+content 均空） | `note_service::close_note_if_empty` |
| INV-004 | ID 唯一性：UUID v4 + DB PRIMARY KEY + INSERT OR REPLACE | `Note::new` / `Reminder::new` |
| INV-005 | 外键级联：删除 Note 时级联删除 Reminder（DB ON DELETE CASCADE + 代码双保险） | `delete_note` 命令 |
| INV-006 | DB 查询强制用列名（`row.get("id")`）而非索引 | `sqlite_note_repo.rs`、`sqlite_reminder_repo.rs` |
| INV-007 | Note 归档互斥：find_all 返回 is_archived=0，find_archived 返回 is_archived=1 | `sqlite_note_repo.rs` |
| INV-008 | Reminder 仅 Pending 状态可触发；snoozed_until 存在时比较贪睡截止时间 | `Reminder::is_due` |
| INV-009 | 周期提醒触发后由调度器计算 next_trigger 并重置为 Pending；一次性标记为 Triggered | `reminder_scheduler.rs` |
| INV-010 | 双存储：SQLite 为运行时存储，JSON 文件为同步载体，notes.db 不入 Git | `git_sync.rs` |
| INV-011 | 冲突解决：last-write-wins，按 updated_at 取最新 | `git_sync.rs` resolve_json_conflict |
| INV-012 | push 策略：--force-with-lease | `git_sync.rs` |
| INV-013 | 自动同步防抖：30 秒延迟 | `git_sync.rs` schedule_auto_sync + `commands.rs` 写操作命令调用 |
| INV-014 | Windows 子进程必须设置 CREATE_NO_WINDOW 标志，禁止弹出控制台窗口 | `git_ops.rs` run_git / check_git_installed / list_remote_branches |
| INV-015 | Git 同步前必须验证远程分支存在（`git rev-parse origin/<branch>`），分支不存在时返回 `BRANCH_NOT_FOUND:<已有分支>` 由前端提示用户选择是否创建 | `git_sync.rs` sync 方法 |
| INV-016 | 所有 git 子进程必须设置 `stdin(Stdio::null())`，防止交互式提示导致进程挂起 | `git_ops.rs` run_git / check_git_installed / list_remote_branches |
| INV-017 | Git 同步初始化后必须验证本地分支名与配置一致，不一致时自动重命名 | `git_sync.rs` sync 方法 |
| INV-018 | Note.tags 字段使用 `#[serde(default)]` 确保旧版 JSON 同步文件反序列化为空数组而非报错 | `domain/note.rs` Note 结构体 |
| INV-019 | 标签数量上限 10 个（MAX_TAGS），单个标签长度上限 20 字符（MAX_TAG_LEN）；set_tags 自动 trim/去重/截断 | `domain/note.rs` set_tags/add_tag |
| INV-020 | LunarMonthly 重复类型的 next_trigger 在 application 层计算（domain 层返回 None），因为农历转换依赖外部库 tyme4rs，不能放入 domain 层 | `domain/reminder.rs` next_trigger + `application/lunar_calendar.rs` lunar_next_month + `application/reminder_service.rs` fire_reminders |

### 已知策略缺口

无（提醒导入已遵循 last-write-wins，与便签导入逻辑一致）

---

## 禁止事项

### 架构禁止

- 核心层（domain）禁止出现 tauri/rusqlite/tokio 等技术框架代码；serde/uuid/chrono 作为值对象工具库允许
- 核心层内禁止互相依赖
- 禁止循环依赖
- 禁止在 commands.rs 之外定义 `#[tauri::command]`
- 禁止用 `emit_to` 向正在初始化的窗口同步发送事件（死锁）

### 设计禁止

- 禁止为未知变化提前抽象（YAGNI）
- 禁止因为"未来可能"而创建 Factory/Strategy/Registry
- 禁止一步到位抽象（只有一种实现时直接实现）
- 禁止在前端通过 `listen` 事件触发窗口创建（前端关闭后无法接收事件）
- 禁止仓储层提供 partial update 方法（所有写入经 domain 方法 + save，NoteRepository 和 ReminderRepository 均适用）
- 禁止 `reminder_service` 直接操作窗口属性，必须委托 `window_manager`
- 禁止在 Windows 上执行子进程时不设 CREATE_NO_WINDOW 标志（会导致控制台窗口弹出）
- 禁止子进程调用不设 `stdin(Stdio::null())`（可能导致交互式提示挂起进程）
- 禁止在 domain 层结构体中保留无业务逻辑使用的字段（YAGNI 原则）

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
