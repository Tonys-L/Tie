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

---

## 检索指引

按业务分类匹配：

- **编码规范**: LES-001, LES-002, LES-006, LES-011
- **数据同步**: LES-003, LES-007, LES-008, LES-009, LES-010, LES-012, LES-015
- **数据存储**: LES-013, LES-014
- **前端构建**: LES-004
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
