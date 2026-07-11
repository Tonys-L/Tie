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

应用全局状态，在 setup 中创建并通过 Tauri State 管理器注入到各命令。包含 `note_repo`、`reminder_repo`、`git_sync` 三个成员。是组合根的具体实现。

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

`reminder_scheduler` 模块，启动后等 5 秒，每 30 秒轮询 `find_due` 查询到期提醒，触发通知 + 创建便签窗口。

### 防抖 (Debounce)

`schedule_auto_sync` 使用 30 秒防抖策略，多次触发只执行最后一次。通过 `Mutex<Instant>` 记录最后触发时间。

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

### 置顶 (Pin)

将便签窗口设为始终置顶（`always_on_top`）。`is_pinned` 字段控制，通过 `set_always_on_top` 同步到窗口。

---

## R

### 提醒 (Reminder)

关联到便签的时间触发器。支持一次性（Once）、每日（Daily）、每周（Weekly）、每月（Monthly）四种重复类型。状态机：Pending → Triggered → Done/Cancelled。

### 仓储 trait (Repository Trait)

domain 层定义的数据访问能力契约（`NoteRepository`/`ReminderRepository`），infrastructure 层提供 SQLite 实现。依赖倒置原则的体现。

---

## S

### 双存储架构 (Dual Storage)

SQLite 作为本地运行时存储（事务/并发安全），JSON 文件作为 Git 同步传输载体（文本可合并）。`data/sync/` 为 Git 仓库根，每实体一个独立 JSON 文件。

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
