# 经验教训库

> 记录开发过程中遇到的问题、解决方案和经验教训。防止同类问题再次发生。

---

## 文件索引

| 编号 | 标题 | 业务分类 | 严重度 | 状态 | 日期 |
|------|------|----------|--------|------|------|
| LES-001 | Tauri 同步命令死锁 | 编码规范 | 高 | 已修复 | 2026-07-08 |
| LES-002 | show_reminder_panel emit_to 死锁 | 编码规范 | 高 | 已修复 | 2026-07-08 |
| LES-003 | SQLite 二进制文件不适合 Git 同步 | 数据同步 | 中 | 已修复 | 2026-07-08 |
| LES-004 | Vite 多页面入口需显式配置 | 前端构建 | 低 | 已修复 | 2026-07-08 |
| LES-005 | 空便签跳过导致启动不恢复 | 业务逻辑 | 低 | 已修复 | 2026-07-08 |
| LES-006 | 全局 listen 导致所有便签窗口收到同一事件 | 编码规范 | 高 | 已修复 | 2026-07-11 |
| LES-007 | Windows 子进程弹出控制台窗口 | 数据同步 | 中 | 已修复 | 2026-07-13 |
| LES-008 | Git fetch 远程分支不存在仍返回成功 | 数据同步 | 高 | 已修复 | 2026-07-13 |
| LES-009 | extract_updated_at 解析逻辑 bug 导致冲突解决失效 | 数据同步 | 高 | 已修复 | 2026-07-13 |
| LES-010 | Git 子进程未设 stdin null 导致测试挂起 | 数据同步 | 中 | 已修复 | 2026-07-14 |
| LES-011 | repeat_config 空置字段违反 YAGNI 原则 | 编码规范 | 低 | 已修复 | 2026-07-14 |
| LES-012 | tags 字段 JSON 存储须加 serde(default) 防止旧数据反序列化失败 | 数据同步 | 中 | 已修复 | 2026-07-15 |
| LES-013 | FTS5 默认 tokenizer 不支持中文 | 数据存储 | 中 | 已修复 | 2026-07-18 |
| LES-014 | FTS5 JOIN 列名歧义 | 数据存储 | 中 | 已修复 | 2026-07-18 |
| LES-015 | Git 同步 unrelated histories 导致远程数据被删除 | 数据同步 | 致命 | 已修复 | 2026-07-18 |
| LES-016 | Tauri 2.x onCloseRequested 改变默认关闭行为 | 编码规范 | 高 | 已修复 | 2026-07-19 |
| LES-017 | 前端模块拆分循环依赖陷阱（共享样式 + 父子回调） | 前端架构 | 中 | 已修复 | 2026-07-21 |
| LES-018 | 后端写操作副作用散布导致 INV-013 漏调（事件总线解耦） | 后端架构 | 高 | 已修复 | 2026-07-21 |
| LES-019 | 事件机制覆盖盲点：scheduler 写后副作用遗漏（ADR-008 扩展） | 后端架构 | 高 | 已修复 | 2026-07-21 |
| LES-020 | 前端共享 module 拆分粒度：按职责拆 colors/datetime/toast/html 而非单一 helpers | 前端架构 | 中 | 已修复 | 2026-07-21 |
| LES-021 | 状态机转换方法应返回 Result 表达合法性（类型系统守护不变量） | 后端架构 | 中 | 已修复 | 2026-07-22 |
| LES-022 | infrastructure 层禁止重新实现 domain 领域规则（SQL 与 Rust 方法漂移） | 后端架构 | 中 | 已修复 | 2026-07-22 |
| LES-023 | mock/sqlite 仓储保真度缺口：delete 语义差异 + 排序差异 + 存在性守卫 | 后端架构 | 高 | 已修复 | 2026-07-22 |
| LES-024 | 浅模块深化三式：常量表替代薄包装函数 + JSON 提取单点归属 + 函数职责按业务概念归位 | 后端架构 | 中 | 已修复 | 2026-07-24 |
| LES-025 | 前端关闭按钮与后端 close_note_if_empty 竞态导致便签丢失 | 前端架构 | 高 | 已修复 | 2026-07-26 |
| LES-026 | 时间字符串格式一致性：ISO 8601 字符串比较的毫秒边界问题 | 后端架构 | 中 | 已修复 | 2026-07-26 |

---

## 检索指引

按业务分类匹配：

- **编码规范**: LES-001, LES-002, LES-006, LES-011, LES-016
- **数据同步**: LES-003, LES-007, LES-008, LES-009, LES-010, LES-012, LES-015
- **数据存储**: LES-013, LES-014
- **前端构建**: LES-004
- **前端架构**: LES-017, LES-020, LES-025
- **后端架构**: LES-018, LES-019, LES-021, LES-022, LES-023, LES-024, LES-026
- **业务逻辑**: LES-005

---

## 教训写作规范

每条教训包含：

```text
**问题**: [遇到了什么问题？现象是什么？]
**原因**: [为什么会出现这个问题？]
**解决方案**: [如何解决的？]
**影响文件**: [修改了哪些文件？]
**预防**: [如何防止再次发生？]
```

---

## LES-001: Tauri 同步命令死锁

**问题**: Hub 页面加载便签列表时调用 `get_reminders`，同时用户点击便签项调用 `open_note`。两个同步命令都在主线程执行，`open_note` 创建窗口阻塞主线程，`get_reminders` 无法完成，导致整个应用冻结（托盘菜单、IPC 全部卡死）。

**原因**: Tauri 2.0 的同步命令在主线程执行。多个同步命令并发调用时，后执行的命令必须等待前一个完成，但前一个可能被窗口创建阻塞 → 死锁。

**解决方案**: 将所有可能并发调用的命令改为 `async`，让 Tauri 在线程池执行，不阻塞主线程。

**影响文件**: `src-tauri/src/application/commands.rs`

**预防**: 新增命令时评估是否可能被并发调用，如是则用 `async`。详见 ADR-004。

---

## LES-002: show_reminder_panel emit_to 死锁

**问题**: 点击提醒按钮后，所有其他按钮失效，托盘菜单无响应。

**原因**: `show_reminder_panel` 是同步命令，在主线程调用 `emit_to` 向正在初始化的便签窗口发送事件。窗口初始化阻塞主线程，`emit_to` 等待窗口就绪 → 死锁。

**解决方案**: 去掉 `show_reminder_panel` 命令，提醒设置弹窗改为在 Hub 页面内直接弹出，不打开便签窗口。

**影响文件**: `src-tauri/src/application/commands.rs`, `src/hub.ts`

**预防**: 禁止在同步命令中向正在初始化的窗口发送事件。详见 ADR-005。

---

## LES-003: SQLite 二进制文件不适合 Git 同步

**问题**: 多设备同步时 SQLite 文件产生二进制冲突，无法文本合并。

**原因**: SQLite 是二进制格式，Git 无法做行级合并。

**解决方案**: 采用双存储架构——SQLite 运行时存储 + JSON 文件同步载体。每实体一个独立 JSON 文件，冲突时 last-write-wins 按 `updated_at` 取最新。

**影响文件**: `src-tauri/src/application/git_sync.rs`

**预防**: 任何需要 Git 同步的数据都不要用二进制格式存储。详见 ADR-003。

---

## LES-004: Vite 多页面入口需显式配置

**问题**: `hub.html` 引用的 `/src/hub.ts` 在 Vite 开发模式下无法正确处理模块依赖，HMR 不生效。

**原因**: Vite 默认只处理 `index.html` 作为入口。`hub.html` 不是入口页面，Vite 没有正确注入 HMR 客户端。

**解决方案**: 在 `vite.config.ts` 的 `rollupOptions.input` 中显式声明多页面入口。

**影响文件**: `vite.config.ts`

**预防**: 新增 HTML 页面时同步更新 Vite 多页面入口配置。

---

## LES-005: 空便签跳过导致启动不恢复

**问题**: 应用启动后活跃便签没有在桌面显示，日志显示"恢复了 0 张便签"。

**原因**: `restore_all_windows` 中有逻辑跳过 title 和 content 均为空的便签，但用户创建的便签可能确实没有内容。

**解决方案**: 去掉空便签跳过逻辑，恢复所有未归档便签窗口。

**影响文件**: `src-tauri/src/application/window_manager.rs`

**预防**: 启动恢复逻辑不应有业务过滤条件，应恢复所有未归档便签。

---

## LES-006: 全局 listen 导致所有便签窗口收到同一事件

**问题**: 提醒到期时，所有便签窗口都显示了提醒横幅，而不仅是触发提醒的那张便签。

**原因**: 前端使用 `@tauri-apps/api/event` 的全局 `listen()` 监听 `reminder-triggered` 和 `flash-window` 事件。即使后端用 `emit_to` 定向发送到特定窗口 label，全局 `listen` 仍会在所有窗口中触发。

**解决方案**: 改用 `getCurrentWindow().listen()` 窗口级监听，只接收 `emit_to` 发送给当前窗口的事件。

**影响文件**: `src/main.ts`

**预防**: 多窗口应用中，前端事件监听必须使用 `getCurrentWindow().listen`，禁止使用全局 `listen`。

---

## LES-007: Windows 子进程弹出控制台窗口

**问题**: 在 Windows 上执行 Git 同步时，每次调用 `git` 命令都会弹出一个黑色的控制台窗口（cmd.exe），闪烁后消失，严重影响用户体验。

**原因**: Rust 的 `std::process::Command::new("git")` 在 Windows 上默认会创建新的控制台窗口。Tauri 应用是 GUI 程序，没有附加控制台，因此每个子进程调用都会创建一个新窗口。

**解决方案**: 使用 `std::os::windows::process::CommandExt` 的 `creation_flags(CREATE_NO_WINDOW)` 标志（0x08000000）隐藏控制台窗口。通过 `#[cfg(target_os = "windows")]` 条件编译确保跨平台兼容。

**影响文件**: `src-tauri/src/application/git_ops.rs`

**预防**: 在 Windows 上执行任何子进程调用时，必须设置 `CREATE_NO_WINDOW` 标志。详见 INV-014。

---

## LES-008: Git fetch 远程分支不存在仍返回成功

**问题**: 用户配置同步分支为 `main`，但远程仓库实际分支为 `master`。同步操作没有报错，而是静默在远程创建了一个新的 `main` 分支，导致数据分散在两个分支中。

**原因**: `git fetch origin main` 在远程不存在 `main` 分支时，exit code 仍为 0（成功），只是没有获取到任何数据。代码用 `fetch_result.is_ok()` 判断 `has_remote`，导致误认为远程有数据，最终 push 创建了新分支。

**解决方案**: fetch 后用 `git rev-parse origin/<branch>` 验证 ref 是否真实存在。如果不存在，用 `list_remote_branches` 检查远程仓库是否有任何分支：有则报错提示分支名不匹配，无则视为首次推送。

**影响文件**: `src-tauri/src/application/git_sync.rs`, `src-tauri/src/application/git_ops.rs`

**预防**: `git fetch` 的 exit code 不能用于判断远程分支是否存在，必须用 `git rev-parse` 验证 ref。详见 INV-015。

---

## LES-009: extract_updated_at 解析逻辑 bug 导致冲突解决失效

**问题**: Git 同步冲突解决（last-write-wins）始终选择 ours 版本，忽略了 theirs 的 updated_at 时间戳。在多设备同步场景下，较新的数据可能被较旧的数据覆盖。

**原因**: `extract_updated_at` 函数的引号匹配逻辑有误：在找到 `"updated_at"` 后，第一次 `find('"')` 匹配到的是键名的开始引号，而非值的开始引号。函数最终返回 `:` 而非时间戳值，导致所有比较都是 `:` == `:`，永远取 ours。

**解决方案**: 改为先找冒号 `:` 分隔键值，再在冒号后找值的开始引号和结束引号。添加单元测试 `test_extract_updated_at` 覆盖正常和异常场景。

**影响文件**: `src-tauri/src/application/sync_json_io.rs`

**预防**: 纯字符串解析函数必须有单元测试覆盖。冲突解决逻辑的测试应验证"theirs 更新时取 theirs"场景，而非仅验证无冲突场景。

---

## LES-010: Git 子进程未设 stdin null 导致测试挂起

**问题**: git_sync 集成测试运行超过 60 秒仍未完成，进程卡死无响应。

**原因**: `git_ops::run_git` 使用 `Command::new("git").output()` 执行 git 命令，未设置 `stdin(Stdio::null())`。当 git 遇到需要用户输入的场景（如凭证请求、merge 冲突编辑器调用）时，会等待 stdin 输入，导致进程永久挂起。

**解决方案**: 所有 `Command::new("git")` 调用添加 `.stdin(Stdio::null())`，包括 `run_git`、`check_git_installed`、`list_remote_branches`。

**影响文件**: `src-tauri/src/application/git_ops.rs`

**预防**: 所有子进程调用必须设置 `stdin(Stdio::null())`，即使预期不需要输入。详见 INV-016。

---

## LES-011: repeat_config 空置字段违反 YAGNI 原则

**问题**: `Reminder` 结构体的 `repeat_config` 字段在构造函数中始终为 `String::new()`，无任何业务逻辑读写此字段。数据库表也保留了对应的列。

**原因**: 该字段可能为"未来精确日历月重复"（如每月 15 号）预留，但当前 Monthly 重复简化为 +30 天。根据 YAGNI 原则，不应为猜测中的需求提前实现。

**解决方案**: 从 `Reminder` 结构体删除 `repeat_config` 字段，同步修改 SQLite 仓储的 `SELECT_COLS`/`INSERT` SQL 和建表语句。旧数据库通过 `ALTER TABLE reminders DROP COLUMN repeat_config` 自动迁移。

**影响文件**: `src-tauri/src/domain/reminder.rs`, `src-tauri/src/infrastructure/sqlite_reminder_repo.rs`, `src-tauri/src/infrastructure/database.rs`

**预防**: domain 层结构体不应包含无业务逻辑使用的字段。抽象来源于真实变化，而非未来可能的需求。三次法则：第一次直接实现，不预留扩展字段。

---

## LES-012: tags 字段 JSON 存储须加 serde(default) 防止旧数据反序列化失败

**问题**: Note 新增 `tags: Vec<String>` 字段后，旧版本的 JSON 同步文件（不含 tags 字段）在反序列化时会因字段缺失而失败。

**原因**: serde 默认要求所有字段都存在，缺失字段会导致 `Error: missing field`。这在多设备同步场景中尤其常见——旧版本设备导出的 JSON 文件在新版本设备导入时失败。

**解决方案**: 给 tags 字段添加 `#[serde(default)]` 属性，缺失时自动填充 `Vec::new()`（空数组）。同时 SQLite 建表语句设置 `DEFAULT '[]'`。

**影响文件**: `src-tauri/src/domain/note.rs`（Note 结构体 tags 字段），`src-tauri/src/infrastructure/database.rs`（建表/迁移 DEFAULT '[]'）

**预防**: 所有新增的领域模型字段，如果通过 JSON 序列化同步，必须加 `#[serde(default)]` 以保证向后兼容。SQLite 列须设置合理的 DEFAULT 值。

---

## LES-013: FTS5 默认 tokenizer 不支持中文

**问题**: SQLite FTS5 默认 tokenizer 无法对中文进行子串匹配搜索。

**原因**: FTS5 默认使用 unicode61 tokenizer，按空白分词，中文无空格分隔导致整段文本被当作一个 token。

**解决方案**: 改用 trigram tokenizer（`tokenize="trigram"`），按 3 字符滑动窗口生成索引，支持任意语言子串匹配。短查询（< 3 字符）回退到 LIKE 模糊匹配。

**影响文件**: `src-tauri/src/infrastructure/database.rs`, `src-tauri/src/infrastructure/sqlite_note_repo.rs`

**预防**: 需要中文搜索支持时，必须使用 trigram tokenizer 或自定义 tokenizer，不能依赖默认的 unicode61。

---

## LES-014: FTS5 JOIN 列名歧义

**问题**: FTS5 虚拟表与原表 JOIN 时，同名列（如 `id`）产生歧义错误。

**原因**: FTS5 外部内容模式（`content=notes`）的虚拟表包含与原表相同的列名，JOIN 时未指定表名前缀。

**解决方案**: JOIN 查询中所有列名必须指定表别名前缀（如 `notes.id`），避免歧义。

**影响文件**: `src-tauri/src/infrastructure/sqlite_note_repo.rs`

**预防**: 涉及 FTS5 虚拟表与原表 JOIN 时，所有列引用必须带表别名。

---

## LES-015: Git 同步 unrelated histories 导致远程数据被删除

**问题**: 新设备首次同步或换源后同步，远程仓库的全部数据被覆盖为本地数据。用户本意是拉取远程数据，结果远程数据丢失。

**原因**: 新设备的本地仓库由 `git init` 创建，与远程仓库无共同祖先（unrelated histories）。`git merge` 默认拒绝合并不相关历史（Git 2.9+），merge 失败后代码仍继续执行 `git push --force-with-lease`，导致本地少量数据强制覆盖远程大量数据。

此外，旧流程"先导出后拉取"（export→commit→fetch→merge→import→push）在首次同步时，本地 JSON 只包含本地数据，merge 失败后 push 的是不含远程数据的提交。

**解决方案**:
1. 重构同步流程为"先拉后推"：fetch→merge→import→export→commit→push，确保远程数据先进入本地数据库再导出推送
2. merge 命令添加 `--allow-unrelated-histories` 参数，允许合并不相关历史的仓库
3. merge 失败后检查是否仍有未解决的冲突，若有则拒绝 push（不再盲目继续）
4. push 前添加安全检查：当删除文件占比超过 50% 时拒绝推送，防止覆盖远程数据

**影响文件**: `src-tauri/src/application/git_sync.rs`

**预防**: 任何涉及 force push 的同步逻辑，必须遵循"先拉后推"原则，且 merge 失败时禁止继续 push。详见 INV-024/INV-025。

---

## LES-016: Tauri 2.x onCloseRequested 改变默认关闭行为

**问题**: 便签右上角关闭按钮点击后窗口不关闭，`win.close()` 调用无效。

**原因**: Tauri 2.x 中，注册 `win.onCloseRequested(() => { ... })` 会改变窗口的默认关闭行为。注册后，`win.close()` 不再直接关闭窗口，而是触发 `closeRequested` 事件，等待回调处理。如果回调中未手动调用 `event.close()` 或有其他逻辑阻止，窗口就不会关闭。

在便签场景中，`onCloseRequested` 被用来设置 `isClosing = true` 标记（防止保存极小尺寸窗口状态），但这导致后续的 `win.close()` 调用无法直接关闭窗口。

**解决方案**: 移除 `onCloseRequested` 注册，改为在关闭按钮点击时手动设置 `isClosing = true`，然后直接调用 `win.close()` 或 `win.destroy()`。对于 delete_note 命令，后端使用 `win.destroy()` 强制销毁窗口作为主路径（INV-026）。

**影响文件**: `src/main.ts`

**预防**: Tauri 2.x 中 `onCloseRequested` 注册会改变默认关闭行为，需要谨慎使用。如果仅需在关闭前执行逻辑，应考虑其他方式（如手动标记 + 事件拦截），而非全局注册 `onCloseRequested`。

---

## LES-017: 前端模块拆分循环依赖陷阱（共享样式 + 父子回调）

**问题**: 将 `main.ts`（1903 行）和 `hub.ts`（1441 行）按 UI 部件拆分为独立模块时，出现两类循环依赖：

1. **共享样式循环**：`note-renderer.ts`（渲染便签）和 `context-menu.ts`（右键菜单）都需要 `applyNoteStyle` 和 `formatNoteTime`。若把这两个函数放在 `note-renderer.ts`，`context-menu.ts` 导入 `note-renderer.ts`；同时 `note-renderer.ts` 又需要 `context-menu.ts` 的 `showCustomColorPanel` → 循环依赖。

2. **父子回调循环**：`hub.ts` 调用 `showReminderDialog`（提取到 `reminder-dialog.ts`），`showReminderDialog` 创建/删除提醒后需要刷新便签列表调用 `loadNotes`；但 `loadNotes` 也在 `hub.ts`（后提取到 `notes-list.ts`）→ `reminder-dialog.ts` 导入 `notes-list.ts`，`notes-list.ts` 又导入 `reminder-dialog.ts` → 循环依赖。

**原因**: 大文件拆分时，模块间天然存在双向协作：A 调用 B 的渲染能力，B 反过来调用 A 的刷新能力；多个模块共享同一套样式/格式化逻辑。

**解决方案**:

1. **共享样式提取第三模块**：新建 `note-style.ts`，导出 `applyNoteStyle` 和 `formatNoteTime`。`note-renderer.ts` 和 `context-menu.ts` 都只依赖 `note-style.ts`，互不依赖。依赖方向：`note-renderer → note-style`，`context-menu → note-style`。

2. **callback 参数破父子环**：`showReminderDialog(noteId, noteTitle, onNotesChanged)` 接收回调函数而非直接 import 调用方。`notes-list.ts` 调用 `showReminderDialog(..., loadNotes)` 把 `loadNotes` 作为回调传入。依赖方向：`notes-list → reminder-dialog`（单向），`reminder-dialog` 不感知 `notes-list`。

3. **同类应用**：`renderNote(note, setupEventsCallback)` 用 callback 破 `note-renderer ↔ main.ts` 环；`showTemplateDialog(title, app, onSelect)` 用 callback 破 `template-ui ↔` 调用方环。

**影响文件**: `src/note-style.ts`（新建）、`src/note-renderer.ts`、`src/context-menu.ts`、`src/reminder-dialog.ts`、`src/notes-list.ts`、`src/template-ui.ts`

**预防**:
- 大文件拆分前先画依赖图，识别潜在循环
- 共享逻辑（被 2+ 模块复用）必须提取到独立第三模块，禁止放在某个消费方模块
- 父子双向协作必须用 callback 参数：父调用子时把自己的回调传入，子执行完后回调通知父，子不反向 import 父
- 模块间依赖必须是单向的，禁止 `A imports B && B imports A`

---

## LES-018: 后端写操作副作用散布导致 INV-013 漏调（事件总线解耦）

**问题**: `schedule_auto_sync`（自动同步防抖）调用散布在 `commands/note_commands.rs`（15 处）、`commands/reminder_commands.rs`（4 处）、`commands/template_commands.rs`（1 处）、`tray_manager.rs`（1 处）共 21 处。每个写操作命令必须手动调用 `state.git_sync.schedule_auto_sync(app)`，否则违反 INV-013（写操作必须触发自动同步防抖）。

架构评估发现 3 处 INV-013 违规（漏调）：
1. `shortcut_manager::setup_shortcuts` 和 `save_and_reregister` 中的 new_note 回调（快捷键新建便签）未触发 `schedule_auto_sync`
2. `template_commands::save_template` 直接访问 repo 未触发 `schedule_auto_sync`
3. `template_commands::delete_template` 直接访问 repo 未触发 `schedule_auto_sync`

**原因**: 命令层（`#[tauri::command]`）同时承担业务编排和副作用触发两个职责。`service` 层只做纯业务编排（CRUD + 仓储交互），不感知 Tauri 副作用（`schedule_auto_sync`/`emit`/`schedule_recalc`）。每个新写操作入口（命令/快捷键/托盘菜单）都必须记得手动触发 `schedule_auto_sync`，散布在 N 处调用方的副作用极易漏调。

此外 `template_commands` 直接访问 `state.template_repo` 而非通过 service，绕过了 service 层的统一副作用入口，是漏调的典型场景。

**解决方案**: 引入后端内部事件总线（ADR-007），将"写操作完成"语义事件化，副作用下沉到 lib.rs 统一监听：

1. **事件抽象**：`event_bus.rs` 定义 `DomainEvent` 枚举（`NoteWritten`/`ReminderWritten`/`TemplateWritten`）+ `WriteAction` 枚举（`Created`/`Updated`/`Deleted`）+ `EventPublisher` trait + `EventBus` 同步实现 + `MockEventPublisher` 测试工具

2. **service emit 事件**：`note_service`/`reminder_service`/`template_service` 全部写方法增加 `publisher: &dyn EventPublisher` 参数，写操作完成后 emit 对应 `DomainEvent`（携带 `WriteAction` 区分新增/更新/删除）

3. **lib.rs 统一监听**：`AppState` 增加 `event_bus: Arc<EventBus>` 字段；`setup` 中注册监听器，接收任意 `DomainEvent` → 调用 `state.git_sync.schedule_auto_sync(app)`

4. **命令层薄壳化**：`commands/*` 删除全部 20 处 `schedule_auto_sync` 手动调用；`template_commands` 改为调用 `template_service`（消除直接 repo 访问）；`shortcut_manager`/`tray_manager` 写操作入口传 `event_bus` 给 service

5. **trait 抽象保留切换路径**：用 `EventPublisher` trait 而非具体 `EventBus`，未来切 channel 异步只需新增 `ChannelPublisher` 实现，service 签名不变

**影响文件**:
- 新增：`src-tauri/src/application/event_bus.rs`、`src-tauri/src/application/template_service.rs`
- 重写：`src-tauri/src/application/note_service.rs`、`src-tauri/src/application/reminder_service.rs`
- 修改：`src-tauri/src/application/commands/note_commands.rs`、`reminder_commands.rs`、`template_commands.rs`、`src-tauri/src/application/shortcut_manager.rs`、`src-tauri/src/application/tray_manager.rs`、`src-tauri/src/lib.rs`

**预防**:
- service 层写操作的副作用（如同步/通知/调度刷新）应通过事件解耦，禁止在命令层手动触发
- 新增写操作入口（命令/快捷键/托盘菜单/任何调用 service 的地方）只需调用 service 传 `event_bus`，自动触发副作用，无需记忆散布的副作用调用
- service 必须接收 `&dyn EventPublisher` trait object 而非具体 `EventBus`，保留切换实现的低成本路径（依赖倒置）
- 命令层禁止直接访问 repo，必须通过 service（否则绕过事件 emit 导致 INV-013 漏调）
- `MockEventPublisher` 作为测试工具放在 `event_bus.rs` 模块内（`#[cfg(test)]` 标注但模块外可见），供所有 service 测试引用断言 emit 行为

---

## LES-019: 事件机制覆盖盲点：scheduler 写后副作用遗漏（ADR-008 扩展）

**问题**: ADR-007 引入事件总线后，`schedule_auto_sync` 副作用已统一由 lib.rs 监听器处理，但架构评估发现 `reminder_scheduler::fire_reminders_with_deps` 在 `reminder_repo.save(&updated)` 后未 emit 事件——即 scheduler 推进提醒状态后既未触发 `schedule_recalc`（下次到期时间可能变化），也未触发 `schedule_auto_sync`（save 是写操作）。同时 `reminder_commands` 的 create/snooze/dismiss/delete 4 处和 `note_commands::delete_note` 1 处仍手动调用 `state.scheduler.schedule_recalc()`，未走事件总线。

**原因**: ADR-007 设计时聚焦"用户主动写操作"（CRUD 命令），把 scheduler 的"系统自动写"（advance_state 后 save）视为内部逻辑，未纳入事件化范围。事件覆盖范围按"service 层写方法"枚举，scheduler 不在 service 层（在 application 层但属调度器子域），导致遗漏。评估盲点的根本原因：把"写操作"等同于"用户主动写操作"，忽略了系统自动写（定时器触发、状态推进）也是写操作，同样需要触发副作用。

**解决方案**: 将事件总线扩展到 reminder-scheduler 生命周期（ADR-008）：

1. **scheduler 写后 emit 事件**：`fire_reminders_with_deps` 接收 `publisher: &dyn EventPublisher` 参数；每次 `reminder_repo.save(&updated)` 成功后 emit `DomainEvent::ReminderWritten { action: WriteAction::Updated, id: updated.id }`
2. **lib.rs 监听器扩展**：第二条监听器接收 `DomainEvent::ReminderWritten` → 调用 `state.scheduler.schedule_recalc()`（第一条监听器所有 `DomainEvent` → `schedule_auto_sync` 已存在）
3. **调用方删除手动 schedule_recalc**：`reminder_commands` 的 create/snooze/dismiss/delete 4 处和 `note_commands::delete_note` 1 处删除 `state.scheduler.schedule_recalc()` 调用
4. **fire_reminders 签名扩展**：增加 `publisher: &dyn EventPublisher` 参数，`check_and_fire` 从 `state.event_bus.as_ref()` 取出传入

事件语义统一：所有 `ReminderWritten` 事件（无论来自 service 层用户 CRUD 还是 scheduler 内部 advance_state）都触发 `schedule_recalc` + `schedule_auto_sync` 两个副作用。

**影响文件**:
- 修改：`src-tauri/src/application/reminder_scheduler.rs`（fire_reminders_with_deps 签名 +2 参数，save 后 emit；fire_reminders 签名 +1 参数；check_and_fire 传 event_bus；7 处测试调用同步）
- 修改：`src-tauri/src/application/commands/reminder_commands.rs`（4 处删除 schedule_recalc 手动调用）
- 修改：`src-tauri/src/application/commands/note_commands.rs`（delete_note 删除 schedule_recalc 手动调用）
- 修改：`src-tauri/src/lib.rs`（setup 新增第二条监听器：ReminderWritten → schedule_recalc）

**预防**:
- 事件机制评估范围必须覆盖所有写操作，包括系统自动写（定时器触发、状态推进、级联删除等），不能只看用户主动写
- 评估事件覆盖时按"哪些代码会调用 `repo.save`/`repo.delete`"枚举，而非按"service 层方法"枚举（scheduler 等非 service 模块也会调用 repo）
- scheduler 等系统自动写模块的 save 后副作用（重新调度/同步）应通过事件解耦，禁止在 scheduler 内部手动触发，禁止在调用方手动触发
- 事件机制扩展后，新增写操作模块（如新调度器、新后台任务）只需在 save 后 emit 事件，副作用自动触发
- ADR-008 是 ADR-007 的扩展，两者共同覆盖所有写操作的事件化（service 层 + scheduler 层）

---

## LES-020: 前端共享 module 拆分粒度：按职责拆 colors/datetime/toast/html 而非单一 helpers

**问题**: 前端原 `src/utils.ts` 承载 7 个不相关函数（escapeHtml/showToast/localISO/formatDate/quickDate/repeatLabel/COLOR_MAP）+ `src/note-style.ts` 承载 2 个函数（applyNoteStyle/formatNoteTime）。`utils.ts` 是通用技术名违反"文件命名=业务名"约束，且单文件混合 HTML 转义/toast 提示/日期时间/颜色映射 4 类不相关职责，AI 修改 showToast 时需要跳过 escapeHtml/localISO 等无关代码，上下文负担重。

**原因**: 初期为减少文件数把不相关的工具函数塞同一 `utils.ts`，违反单一职责。`note-style.ts` 命名虽是业务名但只含 2 个函数且仅服务便签样式场景，与 colors 概念重合。

**解决方案**: 按职责拆分为 4 个独立 module（ADR-009 同批次）：

1. **`src/colors.ts`**：COLOR_MAP + COLORS 颜色映射 + applyNoteStyle + formatNoteTime（吸收原 note-style.ts 2 个函数，因 colors 与 note-style 概念重合）
2. **`src/datetime.ts`**：localISO + formatDate + quickDate + repeatLabel（日期时间工具，依赖 i18n）
3. **`src/toast.ts`**：showToast（toast 提示，被所有页面共享）
4. **`src/html.ts`**：escapeHtml（HTML 转义，被 markdown-renderer 等共享）

删除 `src/utils.ts` 和 `src/note-style.ts`，14 处调用方 import 同步更新。

**影响文件**:
- 新建：`src/colors.ts`、`src/datetime.ts`、`src/toast.ts`、`src/html.ts`
- 删除：`src/utils.ts`、`src/note-style.ts`
- 修改：14 个调用方 import 更新（ai-todo-sort/ai-settings/ai-sniff/ai-rewrite/context-menu/note-renderer/notes-list/reminder-dialog/reminder-panel/tag-bar/template-manager/template-ui/calendar-view/main.ts）

**预防**:
- 前端共享 module 必须按职责拆分，禁止用 `utils.ts`/`helpers.ts` 等通用技术名塞不相关函数
- 拆分粒度判断：一个 module 内的函数应服务同一业务概念（颜色/日期时间/toast/HTML 转义是 4 个独立概念，不应混在一起）
- 概念重合的小 module（如 note-style 与 colors）应合并而非保留两个，减少文件数
- 删除通用技术名 module 后，禁止重建（constraints.md 已列入设计禁止）
- JSDoc 三段头（职责/被调用方/依赖）让 AI 一眼看出 module 用途，无需通读代码

---

## LES-021: 状态机转换方法应返回 Result 表达合法性（类型系统守护不变量）

**问题**: `Reminder` 状态机有 4 个状态（Pending/Triggered/Done/Cancelled）和 4 个转换方法（mark_triggered/snooze/mark_done/cancel），但转换方法返回 `()`，不校验当前状态是否允许转换。终态 Done/Cancelled 可被任意 mark_done/cancel/snooze/mark_triggered 调用，Triggered 可被重复 mark_triggered。flows.md 的禁止转换表只是文档约束，代码层无防护。

**原因**: 初期实现把状态机合法性检查放在调用方（service/scheduler），依赖调用方"知道"哪些状态可转换。这违反了"不变量优先"原则——不变量应由 domain 层守护，不能依赖调用方记忆。advance_state 内部调用 mark_triggered 时用 `let _ = self.mark_triggered();` 忽略了返回值（原返回 `()`），进一步掩盖了问题。

**解决方案**: 4 个转换方法（mark_triggered/snooze/mark_done/cancel）改为返回 `Result<(), String>`，match 当前状态：
- 合法转换：执行 + `Ok(())`
- 非法转换：返回 `Err(错误描述)`

终态 Done/Cancelled 拒绝所有转换；Triggered 拒绝 mark_triggered（不可重复触发）；Triggered 允许 snooze（用户主动延后，回到 Pending）。

advance_state 内部调用 mark_triggered 时用 `.expect("advance_state 契约: 仅对 Pending 提醒调用")` 表达契约——调用方（fire_reminders_with_deps）通过 find_due 的 `WHERE status='pending'` 保证，若违反则 panic 暴露调用方 bug 而非静默失败。

service 层（snooze_reminder/dismiss_reminder）用 `?` 传播 Result 错误。

**影响文件**:
- 修改：`src-tauri/src/domain/reminder.rs`（4 方法签名改返回 Result + advance_state expect + 13 个新测试）
- 修改：`src-tauri/src/application/reminder_service.rs`（snooze/dismiss 加 `?` 传播 + 2 个新测试）
- 同步：`docs/knowledge-base/flows.md`（状态图/转换规则/禁止转换表 + Triggered→Pending via snooze 合法性说明）
- 同步：`docs/knowledge-base/constraints.md`（新增 INV-031）

**预防**:
- 状态机转换方法必须返回 `Result<(), Error>` 表达转换合法性，禁止返回 `()` 隐式成功
- 终态必须拒绝所有转换（match 兜底 `_ => Err`），不能依赖调用方"不调用"
- 内部调用有契约保证的转换方法可用 `expect` 表达契约，panic 优于静默失败
- 状态机合法性属于 domain 不变量，必须由 domain 层守护，不能下沉到 service/scheduler 层
- flows.md 的禁止转换表必须有对应的代码层校验（每个禁止转换至少 1 个测试覆盖）

---

## LES-022: infrastructure 层禁止重新实现 domain 领域规则（SQL 与 Rust 方法漂移）

**问题**: `SqliteReminderRepository::find_due` 用 SQL `WHERE status='pending' AND (snoozed_until IS NULL AND remind_at <= ?1 OR snoozed_until IS NOT NULL AND snoozed_until <= ?1)` 重新实现了 `Reminder::is_due` 的领域规则。而 `InMemoryReminderRepository::find_due` 直接调用 `r.is_due(now)`。两处逻辑当前一致，但 `is_due` 规则变化时 SQL 需同步修改，易遗漏导致行为漂移。

**原因**: infrastructure 层为"性能"（SQL WHERE 直接过滤，避免查全部再过滤）重新实现了 domain 层的领域规则。这违反"领域规则单点归属"原则——领域规则应由 domain 层守护，infrastructure 层只负责持久化，不应承载业务判断逻辑。SQL 和 Rust 方法是两种不同语言，无法共享代码，规则变化时必须手动同步两处，是典型 DRY 违规。

**解决方案**: `find_due` 改为"SQL 只筛 status='pending'，Rust 侧调 `is_due` 过滤"：
```rust
let reminders: Vec<Reminder> = stmt.query_map([], row_to_reminder)...;
Ok(reminders.into_iter().filter(|r| r.is_due(now)).collect())
```
与 `InMemoryReminderRepository::find_due` 实现方式对齐（都委托 is_due）。`is_due` 成为单一真相源（INV-008）。

**影响文件**:
- 修改：`src-tauri/src/infrastructure/sqlite_reminder_repo.rs`（find_due 简化 + 2 个 snoozed 测试）
- 同步：`docs/knowledge-base/constraints.md`（INV-008 检查位置更新）

**预防**:
- infrastructure 层禁止重新实现 domain 层的领域规则（判断/校验/计算逻辑），应委托 domain 方法
- SQL WHERE 用于结构性筛选（status/外键/范围），业务判断逻辑（如"是否到期"）委托 domain 方法在 Rust 侧过滤
- 当 InMemory mock 实现调用 domain 方法而 SQLite 实现用 SQL 重新实现时，是规则重复的信号
- 领域规则变化时，如果需要修改多个文件/多种语言（SQL + Rust），说明规则归属错位
- 性能优化不应以规则重复为代价：Pending 提醒数量通常极小，内存过滤开销可忽略

---

## LES-023: mock/sqlite 仓储保真度缺口：delete 语义差异 + 排序差异 + 存在性守卫

**问题**: 架构评估第三轮发现 `InMemoryXxxRepository`（mock）与 `SqliteXxxRepository`（生产）存在 3 类行为差异，且无测试锁定等价性：

1. **delete 语义差异**：mock `NoteRepository::delete`/`ReminderRepository::delete`/`TemplateRepository::delete` 对不存在的 id 返回 `Err("not found")`；sqlite delete（`DELETE FROM xxx WHERE id=?`）对不存在的 id 静默返回 `Ok(())`（0 行受影响）。生产环境 `note_service::delete_note("nonexistent")` 走到 sqlite 路径时，因 mock 测试未覆盖此场景，service 层无条件 emit `NoteWritten(Deleted)` 事件，触发不必要的 `schedule_auto_sync`，违反 INV-013 的"写操作完成"语义（实际未删除任何数据）。

2. **排序差异**：sqlite `find_all`/`find_by_note_id`/`find_all_templates` 由 SQL `ORDER BY` 保证返回顺序（按 created_at/updated_at/sort_order）；mock 用 `HashMap` 存储，`iter()` 顺序不确定。service 层在 `batch_delete` 中依赖 `find_by_id` 预检查存在性，但其他批量操作（`batch_archive`/`batch_unarchive`/`batch_update_color`）直接遍历 `find_all` 结果，顺序不确定导致 UI 列表渲染顺序在测试与生产间不一致。

3. **存在性守卫缺失**：service 层 `delete_note`/`delete_reminder`/`delete_template` 在 delete 前未检查实体是否存在，导致对不存在的 id emit 事件，触发无意义的 git sync。

**原因**: mock 仓储实现独立编写，以"测试便利性"为目标（delete 不存在时报错便于发现测试 bug），未以 sqlite 生产实现为基准对齐语义。这是"mock 应模拟生产行为"原则的违反——mock 的职责是模拟生产仓储的行为契约，不是定义自己的行为契约。两套实现独立维护，无等价性测试锁定，语义漂移在所难免。

根本原因：缺乏"mock/sqlite 一致性测试"模式。现有测试只测 mock 行为或只测 sqlite 行为，没有同时跑两套实现断言等价的测试。当 mock 行为变化时，无人检查 sqlite 是否一致；反之亦然。

**解决方案**:

1. **mock delete 幂等化**：`InMemoryNoteRepository::delete`/`InMemoryReminderRepository::delete`/`InMemoryTemplateRepository::delete` 改为 `self.xxx.lock().unwrap().remove(id); Ok(())`（HashMap::remove 对不存在的 key 返回 None 但不报错），与 sqlite `DELETE WHERE id=?` 语义对齐。

2. **mock 排序对齐**：
   - `InMemoryReminderRepository::find_all` 加 `result.sort_by(|a, b| a.remind_at.cmp(&b.remind_at))`
   - `InMemoryReminderRepository::find_by_note_id` 加同上排序
   - `InMemoryTemplateRepository::find_all` 加 `result.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.created_at.cmp(&b.created_at)))`

3. **service 层存在性守卫**：
   - `note_service::delete_note`：`find_by_id` 返回 `None` 时直接 `Ok(())`，不 emit 事件
   - `reminder_service::delete_reminder`：同上
   - `template_service::delete_template`：同上
   - `note_service::batch_delete`：预检查 `if note_repo.find_by_id(id)?.is_none() { continue; }`（修复 mock 幂等化后 succeeded 语义问题——原 batch_delete 用 `is_ok()` 判断成功，mock delete 幂等化后 nonexistent 也返回 Ok 会被错误计入 succeeded）

4. **mock/sqlite 一致性测试**：`infrastructure/mod.rs` 新增 `repo_consistency_tests` 模块，7 个测试覆盖：
   - `note_delete_nonexistent_idempotent`：两套实现都返回 `Ok(())`
   - `reminder_delete_nonexistent_idempotent`：同上
   - `template_delete_nonexistent_idempotent`：同上
   - `reminder_find_all_sort_order_consistent`：两套实现返回顺序一致（按 remind_at）
   - `reminder_find_by_note_id_sort_order_consistent`：同上
   - `template_find_all_sort_order_consistent`：两套实现返回顺序一致（按 sort_order then created_at）
   - 辅助函数 `sqlite_reminder_repo_with_note`（先插入父 note 满足外键约束）、`sqlite_template_repo_empty`（清空默认模板）

**影响文件**:
- 修改：`src-tauri/src/domain/mock_repo.rs`（3 处 delete 幂等化 + 3 处 find_all/find_by_note_id 加排序）
- 修改：`src-tauri/src/application/note_service.rs`（delete_note 存在性守卫 + batch_delete 预检查 + 抽取 create_note_with_deps）
- 修改：`src-tauri/src/application/reminder_service.rs`（delete_reminder 存在性守卫）
- 修改：`src-tauri/src/application/template_service.rs`（delete_template 存在性守卫）
- 新增：`src-tauri/src/infrastructure/mod.rs` `repo_consistency_tests` 模块（7 个一致性测试）
- 同步：`docs/knowledge-base/constraints.md`（INV-013 检查位置补充存在性守卫 + 新增 mock/sqlite 保真度约束）

**预防**:
- mock 仓储必须以 sqlite 生产实现为基准对齐行为契约，禁止 mock 自定义语义（如 delete 不存在时报错）
- 必须有 mock/sqlite 一致性测试锁定等价行为，覆盖 delete 语义、排序、find_by_id 返回 None 等边界
- service 层 delete 方法必须加存在性守卫：`find_by_id` 返回 None 时直接 `Ok(())` 不 emit 事件，避免对不存在的实体触发副作用（INV-013 语义保真）
- mock 用 HashMap 存储时，`find_all`/`find_by_note_id` 等返回集合的方法必须显式排序（与 sqlite ORDER BY 对齐），禁止依赖 HashMap iter 顺序
- 新增 mock 方法时，必须同步新增对应的一致性测试（同输入跑 mock + sqlite，断言返回值/顺序等价）
- 一致性测试辅助函数处理外键约束（如 reminders 表 FOREIGN KEY notes(id)）和默认种子数据（如 templates 表 3 个默认模板），避免测试 setup 漂移

---

## LES-024: 浅模块深化三式——常量表替代薄包装函数 + JSON 提取单点归属 + 函数职责按业务概念归位

**问题**: 架构评估第五轮发现 3 处模块设计问题：

1. **`locale_manager` 浅模块**：模块由 9 个 `pub fn menu_new_note() -> &'static str { t!("新建便签", "New Note") }` 形式的薄包装函数组成，每个函数仅一行查表返回。新增文本需新增函数，调用方 `menu_new_note()` vs 常量 `MENU_NEW_NOTE` 语义不够清晰，且函数无法在 const 上下文使用。

2. **JSON 提取逻辑两处独立实现**：`ai_commands::extract_json_array`（提取 `[` 开头数组）和 `reminder_parser::extract_json`（提取 `{` 开头对象）各自手写 `find(start_char) + rfind(end_char)` 切片逻辑，逻辑重复且 `rfind` 会误切 JSON 字符串值内含定界符的场景（如待办条目 `[重要]` 被误当作数组结束）。

3. **`formatNoteTime` 职责错位**：时间格式化函数 `formatNoteTime` 放在 `colors.ts`（颜色映射模块），违反"函数职责按业务概念归属"原则——时间格式化属于 `datetime.ts`（日期时间工具）的职责范围。

**原因**:

1. **浅模块反模式**：初期为"减少 const 定义"用函数封装 i18n 文本，随着文本数量增长到 9 个，函数体的样板代码（函数签名 + `t!` 宏调用）比常量定义更长，且每个函数独立暴露增加模块表面面积。这是"用函数封装一切"的过度抽象——当函数体仅是一行查表返回时，常量表 + `.get()` 方法是更直接的表达。

2. **JSON 提取重复**：`ai_commands` 和 `reminder_parser` 各自实现 JSON 提取，因为两处看似"提取不同符号（`[` vs `{`）"，但本质都是"从文本中提取第一个完整 JSON 值片段"。手写 `rfind` 还隐藏了边界 bug——JSON 字符串值内含定界符时误切。这是 DRY 违规 + 手写解析器 bug 的双重问题。

3. **职责错位历史遗留**：`formatNoteTime` 初期与 `applyNoteStyle` 同属 `note-style.ts`（便签样式），ADR-009 拆分时 `note-style.ts` 合并到 `colors.ts`（概念重合），但 `formatNoteTime` 是时间格式化，与颜色无关，应归 `datetime.ts`。拆分时未细察函数业务概念，导致错位延续。

**解决方案**:

1. **`locale_manager` 常量表化**：定义 `LocaleText { zh: &'static str, en: &'static str }` 结构体 + `impl LocaleText { pub fn get(&self) -> &'static str }`（按 `LOCALE` 全局 `AtomicU8` 返回）；9 个 `pub const MENU_NEW_NOTE: LocaleText = LocaleText { zh: "...", en: "..." }`；调用方 `locale_manager::MENU_NEW_NOTE.get()` 替代 `locale_manager::menu_new_note()`。新增文本只需加一行 `pub const`，无需新增函数。`get_locale_code`/`set_locale_code` 保留 pub 供 `locale_commands::set_locale` 调用。

2. **新建 `application/json_extract.rs`**：`extract_object(text: &str) -> Option<&str>`（提取 `{` 开头对象）+ `extract_array(text: &str) -> Option<&str>`（提取 `[` 开头数组），共享私有 `extract_first(text, start_char)` 实现。用 `serde_json::Deserializer::from_str(slice).into_iter::<IgnoredAny>()` 流式解析第一个完整 JSON 值，通过 `stream.byte_offset()` 精确取得边界。`ai_commands::ai_sort_todos` 改用 `json_extract::extract_array`，`reminder_parser::parse_reminder_json`/`sniff_suggestions` 改用 `json_extract::extract_object`；7 个测试迁移到 `json_extract::tests`（4 个 array + 4 个 object 边界场景）。

3. **`formatNoteTime` 迁移**：从 `src/colors.ts` 移到 `src/datetime.ts`，`src/note-renderer.ts` import 从 `import { formatNoteTime } from './colors'` 改为 `from './datetime'`。`colors.ts` JSDoc 更新说明"不负责 formatNoteTime（在 datetime.ts）"。

**影响文件**:
- 重写：`src-tauri/src/application/locale_manager.rs`（`LocaleText` 结构体 + 9 常量 + `.get()` + 3 测试）
- 新建：`src-tauri/src/application/json_extract.rs`（`extract_object`/`extract_array` + 7 测试）
- 修改：`src-tauri/src/application/commands/ai_commands.rs`（删除 `extract_json_array`，改用 `json_extract::extract_array`）
- 修改：`src-tauri/src/application/reminder_parser.rs`（删除 `extract_json`，改用 `json_extract::extract_object`）
- 修改：`src-tauri/src/application/tray_manager.rs`（7 处 `menu_*()` → `MENU_*.get()` + `MENU_TOOLTIP.get()`）
- 修改：`src-tauri/src/application/hub_window_manager.rs`（`menu_hub_title()` → `MENU_HUB_TITLE.get()`）
- 修改：`src-tauri/src/application/git_sync.rs`（`notify_sync_ok()`/`notify_sync_fail()` → `NOTIFY_SYNC_OK.get()`/`NOTIFY_SYNC_FAIL.get()`）
- 修改：`src/colors.ts`（删除 `formatNoteTime` + JSDoc 更新）
- 修改：`src/datetime.ts`（新增 `formatNoteTime`）
- 修改：`src/note-renderer.ts`（import 更新）
- 同步：`docs/knowledge-base/constraints.md`（新增模块边界 3 条 + 设计禁止 3 条）
- 同步：`docs/knowledge-base/boundaries.md`（json_extract.rs 文档化 + formatNoteTime 迁移注释）

**预防**:
- **浅模块识别信号**：模块内多个函数体仅一行查表/包装返回时，是浅模块反模式。优先用常量表 + 方法替代，减少模块表面面积（N 个函数 → 1 个方法 + N 个常量）
- **常量表模式适用判断**：当函数返回值是编译期已知的固定集合（如 i18n 文本、错误码映射），用 `struct + const + method` 模式比 `fn` 更直接；当返回值依赖运行时计算/参数时仍用函数
- **JSON 解析禁止手写切片**：任何"从文本提取 JSON 片段"的需求必须用 `serde_json::Deserializer` 流式解析，禁止 `find + rfind` 手写切片——手写无法正确处理字符串值内含定界符场景，且重复实现违反 DRY
- **函数迁移判断标准**：函数处理的业务概念（颜色/时间/toast/HTML 转义）应与模块名一一对应。当函数与所在模块名概念不符时（如 `formatNoteTime` 在 `colors.ts`），即使历史遗留也应及时迁移，避免"技术命名模块承载不相关业务函数"的累积
- **模块拆分/合并时复查函数归属**：ADR-009 类型的批量拆分（如 `note-style.ts` 合并到 `colors.ts`）必须逐函数复查业务概念归属，不能简单"整体搬迁"

---

## LES-025: 前端关闭按钮与后端 close_note_if_empty 竞态导致便签丢失

**问题**: 用户创建新便签后输入内容，点击关闭按钮，便签内容丢失（未存库）。重启应用后便签消失。

**原因**: 前端关闭按钮点击时直接调用 `win.close()`，未等待 `api.updateNoteContent` 保存完成。后端 `CloseRequested` 事件触发 `close_note_if_empty`（INV-003：空便签关闭时从 DB 删除），此时数据库中 `note.content` 仍为空（前端 save 尚未完成或尚未调用），`is_empty()` 返回 true → 便签被误删除。

竞态时序：
1. 用户点击关闭 → 前端 `win.close()` 立即执行
2. 后端 `CloseRequested` → `close_note_if_empty` 查询 DB → content 为空 → 删除便签
3. 前端 `api.updateNoteContent` 的 IPC 调用可能尚未到达后端，或已到达但 save 未完成

**解决方案**: 前端关闭按钮改为 async，若处于编辑模式（textarea display !== 'none'），先 await `api.updateNoteContent` 完成保存，再切换回查看模式，最后 `win.close()`：

```typescript
app.querySelector('[data-close]')!.addEventListener('click', async () => {
  const textareaEl = app.querySelector('[data-content]') as HTMLTextAreaElement;
  const contentViewEl = app.querySelector('[data-content-view]') as HTMLElement;
  if (textareaEl.style.display !== 'none') {
    const content = textareaEl.value;
    if (content !== note.content) {
      note.content = content;
      try { await api.updateNoteContent(note.id, content); } catch (e) { console.error('保存便签失败:', e); }
    }
    textareaEl.style.display = 'none';
    contentViewEl.style.display = 'block';
    contentViewEl.innerHTML = renderMarkdown(content);
  }
  setClosing(true);
  win.close();
});
```

**影响文件**: `src/main.ts`（关闭按钮逻辑改 async + 先保存再关闭）

**预防**:
- 前端关闭按钮/窗口关闭流程若涉及未保存数据，必须先 await 保存完成再调用 `win.close()`，禁止直接 `win.close()` 后依赖后端事件
- 涉及 `close_note_if_empty`（INV-003）等"检查后删除"语义的场景，前端必须保证数据已持久化后再触发关闭
- 关闭按钮 async 化的延迟通常 < 100ms，用户无感知，但避免了数据丢失风险

---

## LES-026: 时间字符串格式一致性：ISO 8601 字符串比较的毫秒边界问题

**问题**: 用户反馈提醒触发时间不准（延迟/提前）。`Reminder::is_due` 用字符串比较 `effective_time() <= now`，当 `remind_at` 和 `now` 的毫秒格式不一致时，可能出现 1 秒以内的边界判断错误。

**原因**: ISO 8601 字符串比较在数字部分（年月日时分秒）按字典序与数值比较一致，但在数字部分结束后的字符（`'Z'` vs `'.'`）会出现非数值比较：

- `remind_at = "2026-07-25T14:18:00Z"`（不带毫秒）
- `now = "2026-07-25T14:18:00.123Z"`（带毫秒，比 remind_at 晚 123ms）
- 字符串比较第 20 位：`'Z'`（90）> `'.'`（46）→ `remind_at > now` → `is_due` 返回 false → 延迟触发

后端 `fire_reminders_with_deps` 的 `now` 原本带毫秒（`%Y-%m-%dT%H:%M:%S%.3fZ`），而前端 `datetime-local` 输入只能选到分钟（秒和毫秒本应为 0），但 `dt.toISOString()` 会输出带 `.000Z` 的格式，导致格式不一致。

**解决方案**:

1. **后端 now 不带毫秒**：`reminder_scheduler::fire_reminders_with_deps` 的 `now` 改用 `%Y-%m-%dT%H:%M:%SZ` 格式，与界面分钟级 `remind_at` 格式对齐
2. **前端显式设置秒和毫秒为 0**：`reminder-dialog.ts` 和 `reminder-panel.ts` 在 `dt.toISOString()` 前调用 `dt.setSeconds(0, 0)`，与界面精度对齐
3. **parse_instant 添加解析失败日志**：`reminder_scheduler::parse_instant` 在 `parse_from_rfc3339` 失败时记录警告日志，便于排查异常时间格式导致的提前触发

边界场景文档化测试 `test_is_due_same_second_millis_boundary`：验证同一秒内毫秒格式不一致时的字符串比较行为，提醒未来不要让 `now` 带毫秒。

**影响文件**:
- 修改：`src-tauri/src/application/reminder_scheduler.rs`（now 格式去掉毫秒 + parse_instant 日志）
- 修改：`src/reminder-dialog.ts`（dt.setSeconds(0, 0)）
- 修改：`src/reminder-panel.ts`（dt.setSeconds(0, 0)）
- 修改：`src-tauri/src/domain/reminder.rs`（3 个 is_due 回归测试）

**预防**:
- ISO 8601 时间字符串比较要求格式完全一致，禁止一个带毫秒一个不带毫秒
- 后端 `now` 时间字符串的精度应与界面输入精度对齐（界面分钟级 → 后端秒级，不带毫秒）
- 前端构造 `remind_at` 时应显式设置秒和毫秒为 0，与界面精度对齐，避免 `toISOString()` 输出带 `.000Z` 的格式
- 时间解析失败时必须记录日志，便于排查异常格式导致的提前/延迟触发
- 字符串比较 ISO 8601 时间在数字部分是正确的（ASCII 数字字符按字典序与数值比较一致），仅在数字部分结束后的字符（`'Z'` vs `'.'`）可能出现边界问题，最多导致 1 秒误差

---

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-08 | 初始版本，5 条教训 | — | — |
| 2026-07-09 | 按模板重构，补充业务分类和检索指引 | — | — |
| 2026-07-11 | 新增 LES-006（全局 listen 事件广播问题） | — | #FEAT-002 |
| 2026-07-13 | 新增 LES-007/008/009（Windows 控制台窗口、git fetch 分支验证、extract_updated_at bug） | — | #REFACTOR-013 |
| 2026-07-14 | 新增 LES-010/011（git stdin null 挂起、repeat_config YAGNI 清理） | — | #REFACTOR-014 |
| 2026-07-15 | 新增 LES-012（tags serde(default) 兼容性） | — | #FEAT-002 |
| 2026-07-18 | 新增 LES-013（FTS5 默认 tokenizer 不支持中文）/LES-014（FTS5 JOIN 列名歧义） | — | #FEAT-011 |
| 2026-07-18 | 新增 LES-015（Git 同步 unrelated histories 导致远程数据被删除） | — | #BUGFIX-001 同步更新 constraints.md |
| 2026-07-19 | 新增 LES-016（Tauri 2.x onCloseRequested 改变默认关闭行为） | AI | v0.8.5 同步更新 constraints.md/flows.md |
| 2026-07-21 | 新增 LES-017（前端模块拆分循环依赖陷阱：共享样式提取第三模块 + 父子 callback 破环）；新增"前端架构"业务分类 | AI | #REFACTOR-034 同步更新 constraints.md/boundaries.md |
| 2026-07-21 | 新增 LES-018（后端写操作副作用散布导致 INV-013 漏调：事件总线解耦）；新增"后端架构"业务分类 | AI | #REFACTOR-036 同步更新 constraints.md/glossary.md/adr/README.md/boundaries.md |
| 2026-07-21 | 新增 LES-019（事件机制覆盖盲点：scheduler 写后副作用遗漏，ADR-008 扩展，覆盖系统自动写场景）+ LES-020（前端共享 module 拆分粒度：按职责拆 colors/datetime/toast/html 而非单一 helpers）；更新检索指引（前端架构 +LES-020，后端架构 +LES-019） | AI | #REFACTOR-038 同步更新 ADR-008/009/010/constraints.md/glossary.md/boundaries.md |
| 2026-07-22 | 新增 LES-021（状态机转换方法应返回 Result 表达合法性：类型系统守护不变量）；更新检索指引（后端架构 +LES-021） | AI | #REFACTOR-039 同步更新 constraints.md/flows.md/boundaries.md/glossary.md |
| 2026-07-22 | 新增 LES-022（infrastructure 层禁止重新实现 domain 领域规则：SQL 与 Rust 方法漂移）；更新检索指引（后端架构 +LES-022） | AI | #REFACTOR-040 同步更新 constraints.md |
| 2026-07-22 | 新增 LES-023（mock/sqlite 仓储保真度缺口：delete 语义差异 + 排序差异 + 存在性守卫）；更新检索指引（后端架构 +LES-023） | AI | #REFACTOR-044 同步更新 constraints.md |
| 2026-07-24 | 新增 LES-024（浅模块深化三式：常量表替代薄包装函数 + JSON 提取单点归属 + 函数职责按业务概念归位）；更新检索指引（后端架构 +LES-024） | AI | #REFACTOR-045 同步更新 constraints.md/boundaries.md |
| 2026-07-26 | 新增 LES-025（前端关闭按钮与后端 close_note_if_empty 竞态导致便签丢失）+ LES-026（时间字符串格式一致性：ISO 8601 字符串比较的毫秒边界问题）；更新检索指引（前端架构 +LES-025，后端架构 +LES-026） | AI | #BUGFIX-002 |
