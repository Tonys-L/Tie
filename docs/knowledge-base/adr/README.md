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

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-08 | 初始版本，5 条 ADR | — | — |
| 2026-07-09 | 按模板重构，补充业务分类和影响模块 | — | — |
