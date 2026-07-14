# 能力边界

> **TL;DR**: 核心能力：便签管理、提醒调度、数据同步。能力边界：单用户桌面工具，不提供云服务/多用户协作。⚠️ 便签管理不包含富文本编辑，提醒调度不包含日历视图。

---

## 核心能力

### 便签管理

**能力定义**: 创建、编辑、归档/恢复、删除桌面悬浮便签，支持颜色/透明度/置顶调整。

**业务规则**:
- 每张便签一个独立窗口，label 格式 `note-{uuid}`
- 窗口关闭时若 title+content 均空则自动删除
- 归档后不在桌面显示但保留数据
- 透明度范围 0.3~1.0

**变化点**:
- 前端渲染方式（当前 Markdown，未来可能富文本）
- 颜色选项扩展

**对应代码**:
- `src-tauri/src/domain/note.rs`（领域模型）
- `src-tauri/src/application/commands.rs`（命令入口，`#[tauri::command]` 集中地）
- `src-tauri/src/application/note_service.rs`（便签编排：create_note 创建+开窗口、close_note_if_empty 空便签自动删除 INV-003、sync_notes 同步机制）
- `src-tauri/src/application/window_manager.rs`（窗口管理）
- `src/main.ts`（前端入口）

---

### 提醒调度

**能力定义**: 为便签设置一次性或周期性提醒，到期触发系统通知 + 弹窗。

**业务规则**:
- 仅 Pending 状态可触发
- 周期提醒触发后重置为 Pending 并计算下次时间
- 一次性提醒触发后标记为 Triggered
- 贪睡功能延后再次触发，状态保持 Pending
- 提醒触发由后端直接创建便签窗口，不依赖前端事件
- 调度方式：事件驱动（单定时器 + Arc<Notify>），创建/更新/删除提醒时通知调度器重新计算定时器

**变化点**:
- Monthly 重复当前简化为 +30 天，未来需精确日历月
- 通知方式（当前系统通知 + 弹窗）

**对应代码**:
- `src-tauri/src/domain/reminder.rs`（领域模型 + 状态机）
- `src-tauri/src/application/reminder_scheduler.rs`（事件驱动调度：单定时器 + Notify）
- `src-tauri/src/application/reminder_service.rs`（提醒触发编排：通知+弹窗+状态更新）
- `src-tauri/src/application/commands.rs`（提醒命令）

---

### 数据同步

**能力定义**: 基于 Git 仓库的多设备数据同步，JSON 文件为传输载体。

**业务规则**:
- SQLite 为运行时存储，JSON 文件为同步载体
- 冲突解决：last-write-wins，按 updated_at 取最新
- push 策略：--force-with-lease
- 自动同步防抖：30 秒延迟

**变化点**:
- 同步协议（当前 Git HTTPS，未来可能其他）
- 冲突解决策略（当前 last-write-wins，未来可能语义级合并）

**对应代码**:
- `src-tauri/src/application/git_sync.rs`（GitSync struct + sync() 编排 + 调度）
- `src-tauri/src/application/sync_config.rs`（SyncConfig + 配置读写 + 认证 URL）
- `src-tauri/src/application/sync_json_io.rs`（DB ↔ JSON 文件转换）
- `src-tauri/src/application/git_ops.rs`（Git 子进程执行 + 冲突解决）
- `src-tauri/src/application/note_service.rs`（sync_notes 编排，被 commands/tray_manager 复用）

---

## 支撑能力

### 桌面常驻

- 系统托盘常驻（`tray_manager.rs`）
- 全局快捷键唤起（`shortcut_manager.rs`：快捷键可配置，存储在 `data/shortcut_config.json`，默认 Ctrl+Shift+N 新建、Ctrl+Shift+S 显示全部）
- 启动时恢复所有未归档便签窗口（`window_manager.rs` restore_all_windows）
- 关闭窗口不退出应用，托盘菜单"退出"才真正退出

### IPC 通信

- 前端通过 `@tauri-apps/api/core` 的 `invoke` 调用后端命令
- 后端通过 `window.emit` / `emit_to` 向前端发送事件（如 `flash-window`、`reminder-triggered`）
- 27 个命令集中在 `application/commands.rs`
- 可能并发的命令必须 `async` 避免死锁

### 前端多页面边界

- `index.html` → 便签窗口入口（`src/main.ts`）
- `hub.html` → 设置中心入口（`src/hub.ts`）
- 共享模块：`src/types.ts`（类型定义）、`src/api.ts`（IPC 封装）、`src/utils.ts`（工具函数）
- 两个页面独立加载，共享 CSS 变量（`--surface`、`--text-title` 等）
- Vite 多页面入口需在 `vite.config.ts` 的 `rollupOptions.input` 中显式配置（当前：index.html + hub.html）

---

## 外部依赖能力

| 依赖 | 用途 | 替换成本 |
|------|------|----------|
| Git（系统安装） | 数据同步的版本控制和传输 | 高（同步逻辑全部重写） |
| WebView2 运行时 | 前端渲染引擎 | 高（无替代方案） |
| Tauri 2.0 框架 | 窗口管理、IPC、托盘、通知、快捷键 | 高（整个后端重写） |
| SQLite (rusqlite) | 本地数据持久化 | 中（仓储 trait 隔离，换 DB 只改 infrastructure） |

---

## 系统边界

### 系统内（我们负责）

- 便签的本地 CRUD 和窗口管理
- 提醒的创建、调度、触发
- 基于 Git 的数据同步逻辑
- 系统托盘和全局快捷键

### 系统外（外部负责）

- Git 平台安全性（Gitee/GitHub 负责）
- 系统通知展示（操作系统负责）
- WebView2 渲染引擎（Microsoft 负责）
- 文件系统权限（操作系统负责）

---

## 扩展点分析

| 扩展点 | 当前实现 | 未来可能 | 扩展方式 |
|--------|----------|----------|----------|
| 前端框架 | 原生 TS | 可能引入 React/Vue | Vite 配置不变，替换前端代码 |
| 数据库 | SQLite | 可能换 PostgreSQL | 仓储 trait 隔离，新增 infrastructure 实现 |
| 同步协议 | Git HTTPS | 可能换 WebSocket/云服务 | 重写 git_sync 模块 |
| 重复类型 | Daily/Weekly/Monthly(+30天) | 可能精确日历月 | 修改 `next_trigger()` 计算 |
| 通知方式 | 系统通知 + 弹窗 | 可能加邮件/推送 | 新增通知通道模块 |
| 快捷键 | 可配置（2 个动作） | 可能新增动作 | `shortcut_manager.rs` + `shortcut_config.json` |

---

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-09 | 初始版本，按模板结构填充 | — | — |
| 2026-07-09 | 清理遗留 HTML 文件，更新前端页面描述 | — | #REFACTOR-001 |
| 2026-07-09 | 前端分层重构：新增 types.ts/api.ts/utils.ts | — | #REFACTOR-003 |
| 2026-07-09 | 提取 create_note 编排到 note_service.rs，三处调用方复用 | — | #REFACTOR-004 |
| 2026-07-09 | 提取 sync_notes 编排到 note_service.rs，commands/tray_manager 复用 | — | #REFACTOR-005 |
| 2026-07-09 | 提取 close_note_if_empty 到 note_service.rs，lib.rs 关窗事件委托 | — | #REFACTOR-006 |
| 2026-07-09 | 删除 NoteRepository 4 个 partial update 方法，所有写入经 domain + save | — | #REFACTOR-007 |
| 2026-07-09 | 拆分 git_sync.rs（445 行）为 sync_config/sync_json_io/git_ops 三模块，git_sync 保留编排+调度 | — | #REFACTOR-008 |
| 2026-07-10 | 提取 reminder_scheduler 编排到 reminder_service.rs，调度器仅保留定时入口 | — | #REFACTOR-009 |
| 2026-07-10 | 提取 commands.rs 4 个编排命令到 note_service.rs（open_note/open_note_with_flag/update_note_style/delete_note） | — | #REFACTOR-010 |
| 2026-07-10 | AppState 仓储字段改为 Box<dyn trait>，遵循依赖倒置原则 | — | #REFACTOR-011 |
| 2026-07-10 | 补全 schedule_auto_sync 调用链，12 个写操作命令触发自动同步防抖 | — | #REFACTOR-012 |
| 2026-07-11 | 快捷键可配置（ShortcutManager + shortcut_config.json）；提醒到期已开窗口通过 emit_to 显示横幅；Hub 加 Loading/提醒 tab | — | #FEAT-001 |
| 2026-07-11 | 提醒调度器从 30 秒轮询改为事件驱动（单定时器 + Arc<Notify>）；前端事件监听改为窗口级 getCurrentWindow().listen | — | #FEAT-002 |
| 2026-07-13 | IPC 命令数修正为 25；删除 ReminderRepository partial update 方法；reminder_service 窗口操作委托 window_manager；移除 tauri-plugin-store | — | #REFACTOR-013 |
| 2026-07-13 | 新增 get_data_dir/open_data_dir 命令；通用设置页新增数据存储卡片；sync_notes 新增 create_branch 参数 | — | #FEAT-003 |
| 2026-07-14 | 删除 Reminder.repeat_config 字段；新增 git_sync 集成测试和 reminder_scheduler 单元测试；新增 INV-016/017 | — | #REFACTOR-014 |
