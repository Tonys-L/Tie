# 架构决策记录 (ADR)

> 记录重要的架构决策及其背景、后果。每个决策包含：背景、决策、后果。

---

## ADR 索引

| 编号 | 标题 | 业务分类 | 状态 | 影响模块 | 日期 |
|------|------|----------|------|----------|------|
| ADR-001 | 选择 Tauri 2.0 而非 Electron | 技术选型 | Accepted | 全局 | 2026-07-08 |
| ADR-002 | 三层领域分层架构 | 架构设计 | Accepted | domain/application/infrastructure | 2026-07-08 |
| ADR-003 | 双存储架构：SQLite + JSON | 数据同步 | Accepted | git_sync | 2026-07-08 |
| ADR-004 | Tauri 命令使用 async 避免死锁 | 编码规范 | Accepted | commands | 2026-07-08 |
| ADR-005 | 提醒由后端直接控窗 | 架构设计 | Accepted | reminder_scheduler/window_manager | 2026-07-08 |
| ADR-006 | 前端模块化与 AI 可读性原则 | 前端架构 | Accepted | src/main.ts + src/hub.ts + 27 个前端模块 | 2026-07-21 |
| ADR-007 | 后端内部事件总线（写操作副作用解耦） | 架构设计 | Accepted | application/event_bus + *_service + lib.rs | 2026-07-21 |

---

## ADR 生命周期

```text
Draft → Proposed → Accepted → Deprecated → Superseded
```

- **Draft**: 正在考虑中，尚未决定
- **Proposed**: 已提出，等待讨论
- **Accepted**: 已采纳，正在执行
- **Deprecated**: 已废弃，不再适用
- **Superseded**: 已被新 ADR 替代（需注明替代 ADR 编号）

---

## ADR 文档模板

新建 ADR 时复制以下结构：

```markdown
## ADR-XXX: [标题]

**状态**: [Draft|Proposed|Accepted|Deprecated|Superseded]

**背景**: [为什么需要做这个决策？遇到了什么问题？]

**决策**: [选择了什么方案？理由是什么？]

**后果**:
- 正面: [带来了什么好处]
- 负面: [带来了什么代价或风险]
```

---

## ADR-001: 选择 Tauri 2.0 而非 Electron

**状态**: Accepted

**背景**: 需要开发桌面便签应用，要求体积小、内存低、支持系统托盘常驻、透明窗口。

**决策**: 选择 Tauri 2.0。理由：Rust 后端，release 优化（lto/strip/opt-level=s）产出极小体积；原生支持系统托盘（`features=["tray-icon"]`）；`transparent(true)` + `shadow(false)` 实现真窗口透明；2.0 新权限模型提供细粒度窗口操作控制。

**后果**:
- 正面：体积/内存显著优于 Electron，原生托盘支持完善
- 负面：Tauri 2.0 生态不如 Electron 成熟，文档较少；IPC 模型需要注意死锁问题（见 ADR-004）

---

## ADR-002: 三层领域分层架构

**状态**: Accepted

**背景**: 需要在 Rust 后端组织便签/提醒/同步等业务逻辑，避免技术实现与业务规则耦合。

**决策**: 采用 domain/application/infrastructure 三层架构。domain 定义业务核心与能力契约（纯 Rust），application 编排用例并桥接 Tauri，infrastructure 提供 SQLite 实现。

**后果**:
- 正面：业务规则可测试，技术实现可替换，职责清晰
- 负面：文件较多，初期开发有一定样板代码成本

---

## ADR-003: 双存储架构：SQLite + JSON

**状态**: Accepted

**背景**: 需要本地存储（事务/并发安全）和多设备同步（可文本合并）。SQLite 二进制文件无法 Git 合并。

**决策**: SQLite 作为运行时存储，JSON 文件作为同步传输载体。`data/sync/` 为 Git 仓库根，每实体一个独立 JSON 文件，`notes.db` 不入 Git。

**后果**:
- 正面：JSON 文本可合并，避免了 SQLite 二进制锁冲突
- 负面：需要维护双存储一致性，同步时有导出/导入开销

---

## ADR-004: Tauri 命令使用 async 避免死锁

**状态**: Accepted

**背景**: 同步 Tauri 命令在主线程执行。当 `loadNotes` 中的 `get_reminders` 在主线程排队时，用户点击便签项调用 `open_note`（也在主线程），窗口创建阻塞主线程 → 死锁。整个应用冻结。

**决策**: 所有可能并发调用的命令标记为 `async`，让 Tauri 在线程池执行。简单/不会被并发调用的命令保持同步。

**后果**:
- 正面：彻底解决死锁问题
- 负面：async 命令的 `State` 参数需要用 `State<'_, AppState>` 生命周期标注

---

## ADR-005: 提醒由后端直接控窗

**状态**: Accepted

**背景**: 提醒触发时需要弹窗显示便签。最初方案是后端 `emit_to` 发事件给前端 `listen`，但便签窗口可能已关闭，前端无法接收事件。且 `show_reminder_panel` 同步命令在主线程调 `emit_to` 与正在初始化的窗口产生死锁。

**决策**: 提醒触发时由后端直接 `open_note_window_with_url`（URL 带 `?reminder=1` 参数），前端初始化时从 URL 读取参数显示红色横幅。

**后果**:
- 正面：不依赖前端事件，窗口关闭也能触发；避免 emit_to 死锁
- 负面：URL 参数传递不够优雅，但可靠

---

## ADR-006: 前端模块化与 AI 可读性原则

**状态**: Accepted

**背景**: `src/main.ts`（1903 行）和 `src/hub.ts`（1441 行）单文件过大，AI 在理解和修改时需要频繁跨行跳转，上下文负担重。文件名 `main.ts`/`hub.ts` 是技术名而非业务名，AI 难以从文件名推断职责。模块间存在隐式循环依赖（note-renderer ↔ context-menu 共享样式，hub ↔ reminder-dialog 父子回调），重构时易引入破坏性改动。

**决策**: 将两个入口文件按 UI 部件/业务域拆分为 27 个独立模块，并确立 6 项 AI 可读性原则作为前端架构约束：

1. **入口文件仅编排**：`main.ts`（390 行）和 `hub.ts`（131 行）只负责 init/load/页面切换/全局事件监听，业务实现拆分到独立模块
2. **文件命名=业务名**：使用 `tag-bar.ts`/`note-renderer.ts`/`reminder-dialog.ts` 等业务名，禁止 `utils.ts`/`helpers.ts` 等通用技术名。AI 相关模块统一加 `ai-` 前缀（`ai-sniff.ts`/`ai-rewrite.ts`/`ai-todo-sort.ts`/`ai-settings.ts`），AI 一看前缀即知 AI 模块
3. **JSDoc 三段头**：每个模块文件头必须含三段——职责边界、被调用方、依赖
4. **单向依赖无环**：模块间依赖必须单向，禁止循环。共享逻辑提取第三模块（`note-style.ts`），父子协作用 callback 参数（`renderNote(note, setupEventsCallback)`、`showReminderDialog(noteId, noteTitle, onNotesChanged)`）
5. **state 就近原则**：模块级私有 state + getter/setter 导出（`note-context.ts` 的 getNote/setNote、`notes-list.ts` 的 getActiveNotes/getArchivedNotes）
6. **函数 < 100 行**：超长函数按 UI 部件/事件类型拆分（`setupNoteEvents` 拆为 5 个 bind 子函数）

**后果**:
- 正面：
  - AI 可读性显著提升——单文件单职责，文件名即业务名，JSDoc 头提供上下文
  - 修改某 UI 部件只需读 1 个模块文件，不再需要通读 1900 行入口文件
  - 循环依赖显式化——共享逻辑集中 in `note-style.ts`，父子协作通过 callback 解耦
  - 复用性提升——`notes-list.ts` 的 `getActiveNotes`/`getArchivedNotes` 被 `calendar-view.ts` 复用
- 负面：
  - 文件数从 2 个增加到 29 个，初次理解整体结构需要查看 imports
  - 新增模块需遵守 JSDoc 三段头规范，有一定文档维护成本
  - callback 模式增加函数签名复杂度（如 `showReminderDialog` 3 个参数）

---

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-08 | 初始版本，5 条 ADR | — | — |
| 2026-07-09 | 按模板重构，补充业务分类和影响模块 | — | — |
| 2026-07-21 | 新增 ADR-006（前端模块化与 AI 可读性原则：6 项原则 + main.ts/hub.ts 拆分为 27 个模块） | AI | #REFACTOR-034 同步更新 constraints.md/boundaries.md/lessons/README.md |
| 2026-07-21 | ADR-006 决策 2 补充：AI 相关模块统一加 `ai-` 前缀（ai-sniff/ai-rewrite/ai-todo-sort/ai-settings） | AI | #REFACTOR-035 同步更新 boundaries.md |
| 2026-07-21 | 新增 ADR-007（后端内部事件总线：写操作副作用解耦，EventPublisher trait + EventBus 同步实现） | AI | #REFACTOR-036 同步更新 constraints.md/glossary.md/boundaries.md/lessons/README.md |

---

## ADR-007: 后端内部事件总线（写操作副作用解耦）

**状态**: Accepted

**背景**:
当前 `schedule_auto_sync` 调用散布在 17 处（note_commands 14 处 + reminder_commands 4 处 + template_commands 1 处 + tray_manager 1 处 + shortcut_manager 0 处但应调用）。这种"调用方手动触发副作用"模式导致 3 处 INV-013 违规：

1. `shortcut_manager` 创建便签漏调 `schedule_auto_sync`（setup_shortcuts + save_and_reregister 两处回调）
2. `template_commands::save_template` 直接访问 repo 且漏调
3. `template_commands::delete_template` 直接访问 repo 且漏调

根本原因：service 层完成写操作后，副作用（`schedule_auto_sync`）由调用方负责触发。新增调用方（如 shortcut_manager）或新写操作（如 template save/delete）容易漏调。INV-013 约束靠人工保证，无机制强制。

附带问题：
- `template_commands` 的 save/delete 直接访问 repo 不经过 service，违反"业务编排下沉到 *_service"原则
- `shortcut_manager` 的 `setup_shortcuts` 与 `save_and_reregister` 重复 ~50 行 `gs.on_shortcut` 注册逻辑

**决策**:
引入后端内部事件总线，service 层依赖 `&dyn EventPublisher` trait（依赖倒置），当前用 `EventBus` 同步实现。

- **事件机制**：自定义 `EventPublisher` trait + `EventBus` 同步实现（`Arc<Mutex<Vec<Box<dyn Fn>>>>`）
- **service 层接收 `&dyn EventPublisher` 参数**（trait object，可 mock，可替换实现）
- **事件粒度**：按实体 + 操作类型（`DomainEvent::NoteWritten/ReminderWritten/TemplateWritten` + `WriteAction::Created/Updated/Deleted`）
- **监听器在 lib.rs setup 中注册**（`subscribe(callback)` → 调 `schedule_auto_sync`）
- **所有调用方（命令层/tray/shortcut）删除手动 `schedule_auto_sync` 调用**
- **新建 `template_service`**，承载 template save/delete/create_note_from_template 编排（消除直接 repo 访问）
- **shortcut_manager 提取 `register_handlers` 内部函数**（消除 setup/save_and_reregister 重复）

未来若需异步：新建 `ChannelPublisher` impl `EventPublisher`（内部用 `tokio::sync::broadcast`），service 签名零改动，预计 ~30 行迁移成本集中在 event_bus.rs + lib.rs。

不选 Tauri emit/listen 的原因：会广播给前端；不选直接用 tokio broadcast 的原因：当前同步足够，无需 spawn 监听任务；但通过 trait 保留切换路径。

**后果**:
- **正面**：
  - service 层保持纯净（依赖 `&dyn EventPublisher` 可 mock，不依赖 GitSync）
  - 彻底消除 INV-013 违规风险（新增写操作只需在 service 内 emit）
  - 17 处 `schedule_auto_sync` 散布集中到 1 处监听器
  - `template_commands` 走 service 层消除直接 repo 访问
  - shortcut_manager 不再需要手动调副作用
  - trait 抽象保留切换 channel 的低成本路径
- **负面**：
  - 新增 EventBus 抽象（理解成本略增）
  - service 签名变化（增加 publisher 参数）
  - 事件是同步处理（当前足够，若未来需异步可改为 channel）
