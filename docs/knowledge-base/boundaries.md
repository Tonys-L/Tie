# 能力边界

> **TL;DR**: 核心能力：便签管理、提醒调度、数据同步、日历视图。能力边界：单用户桌面工具，不提供云服务/多用户协作。⚠️ 便签管理不包含富文本编辑，日历视图展示提醒+便签活动+农历，支持点击日期创建提醒。

---

## 核心能力

### 便签管理

**能力定义**: 创建、编辑、归档/恢复、删除桌面悬浮便签，支持颜色/透明度/置顶调整、标签分类、全局搜索。

**业务规则**:
- 每张便签一个独立窗口，label 格式 `note-{uuid}`
- 窗口关闭时若 title+content 均空则自动删除
- 归档后不在桌面显示但保留数据
- 透明度范围 0.3~1.0
- 标签数量上限 10 个，单标签长度上限 20 字符（INV-019）
- 搜索范围跨活跃+归档，匹配标题+内容+标签
- delete_note 命令职责：删除便签及关联提醒；删除成功后通过 `app.get_webview_window(&label)` 获取窗口并 `destroy()` 强制销毁（用 destroy 而非 close：close 可能因 Tauri 2.x 事件时序问题失败，destroy 是强制销毁，确保窗口立即消失）
- 搜索使用 FTS5 trigram tokenizer（CJK 子串匹配），查询 < 3 字符回退 LIKE
- 搜索结果支持高亮片段（FTS5 snippet 生成 `<mark>` 标签）

**变化点**:
- 前端渲染方式（当前 Markdown + 待办清单交互，未来可能富文本）
- 颜色选项扩展
- 搜索 tokenizer（当前 trigram，未来可能引入分词器优化中文匹配）

**对应代码**:
- `src-tauri/src/domain/note.rs`（领域模型，含 tags 字段 + set_tags/add_tag/remove_tag + highlight 搜索高亮片段）
- `src-tauri/src/domain/repositories.rs`（NoteRepository trait，含 search_notes）
- `src-tauri/src/application/commands.rs`（命令入口，含 search_notes/update_note_tags）
- `src-tauri/src/application/note_service.rs`（便签编排：create_note 创建+开窗口、close_note_if_empty 空便签自动删除 INV-003、sync_notes 同步机制）
- `src-tauri/src/application/window_manager.rs`（窗口管理）
- `src-tauri/src/infrastructure/database.rs`（FTS5 虚拟表 + 触发器迁移）
- `src-tauri/src/infrastructure/sqlite_note_repo.rs`（search_notes 实现：FTS5 MATCH + snippet + LIKE 短查询回退）
- `src/main.ts`（前端入口，含标签栏）
- `src/hub.ts`（Hub 前端，含后端搜索+标签侧边栏+排序+高亮渲染）

---

### 便签模板

**能力定义**: 用户自定义便签模板，支持从模板一键创建便签，并随 Git 跨设备同步。

**业务规则**:
- 模板表首次启动为空时自动种子 3 个默认模板（空白/会议记录/待办清单）（INV-022）
- 模板 id 格式 `tpl-{uuid}`，category 固定为 `custom`
- 模板仅用户自定义，不预设系统模板
- 从模板创建便签：复制模板内容到新便签，自动打开窗口
- 模板 CRUD：新建/编辑（名称+内容）/删除
- 模板必须随 Git 同步：导出到 `sync/templates/{id}.json`，导入时按 `updated_at` 仲裁（last-write-wins），与便签/提醒一致（INV-023）

**UI 入口**:
- 设置中心模板管理弹窗（CRUD + 从模板新建）
- 便签右键菜单两项并存：
  - 「从模板新建便签」→ 创建新便签并打开新窗口
  - 「应用模板到当前便签」→ 在当前便签 content 末尾追加 `\n\n` + 模板内容（非破坏性，不覆盖已有内容）
- 空便签编辑区顶部模板快捷条（一键填充当前便签内容，仅当内容为空时显示；多模板时横向单行滚动不换行）

**变化点**:
- 模板分类（当前仅 custom，未来可能扩展分类）
- 模板默认种子数据（当前 3 个，未来可能调整）

**对应代码**:
- `src-tauri/src/domain/template.rs`（Template 领域模型）
- `src-tauri/src/domain/repositories.rs`（TemplateRepository trait）
- `src-tauri/src/domain/mock_repo.rs`（InMemoryTemplateRepository 测试用 mock）
- `src-tauri/src/infrastructure/sqlite_template_repo.rs`（SQLite 实现）
- `src-tauri/src/infrastructure/database.rs`（templates 表 DDL + 默认种子）
- `src-tauri/src/application/commands.rs`（get_templates/save_template/delete_template/create_note_from_template 命令）
- `src-tauri/src/application/sync_json_io.rs`（模板导出/导入 + updated_at 仲裁）
- `src-tauri/src/application/git_sync.rs`（sync/auto_pull_on_startup 传 template_repo）
- `src/hub.ts`（模板管理弹窗 + CRUD UI）
- `src/main.ts`（空便签模板快捷条 setupTemplateQuickBar + 右键菜单 showTemplatePicker）

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
- Monthly 重复已改为精确日历月（月末溢出取目标月最后一天）
- LunarMonthly 重复类型在 application 层计算（domain 层不依赖农历库）
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
- 44 个命令集中在 `application/commands.rs`
- 可能并发的命令必须 `async` 避免死锁

### 前端多页面边界

- `index.html` → 便签窗口入口（`src/main.ts`）
- `hub.html` → 设置中心入口（`src/hub.ts`）
- 共享模块：`src/types.ts`（类型定义）、`src/api.ts`（IPC 封装）、`src/utils.ts`（工具函数）
- 两个页面独立加载，共享 CSS 变量（`--surface`、`--text-title` 等）
- Vite 多页面入口需在 `vite.config.ts` 的 `rollupOptions.input` 中显式配置（当前：index.html + hub.html）

### AI 嗅探

**能力定义**: 扫描便签正文，调用 AI 返回多种编辑建议，辅助用户完善便签内容。

**业务规则**:
- 未配置 AI（api_key 为空）或用户关闭嗅探（`sniff_enabled=false`）时静默跳过，返回空列表
- AI 一次分析可返回多条建议，前端按 `type` 字段分发处理
- 各类型在数据为空时跳过（不返回空建议）
- 未知类型跳过，不影响其他合法建议

**支持的建议类型**:
| 类型 | 说明 | data 结构 |
|------|------|-----------|
| reminder | 检测到时间/提醒信息 | `{ detected, time_text, start_time, title, repeat_type, repeat_day }` |
| todo_split | 可拆分为待办清单 | `Vec<String>`（todos 数组） |
| tidy | 口语化文本可规整 | `String`（tidy_text） |
| style | 文风可改善（正式场景） | `{ style_type, styled_text }` |
| tag_suggest | 推荐标签（最多 3 个） | `Vec<String>`（tags 数组） |

**变化点**:
- 建议类型可扩展（在 `sniff_suggestions` 的 match 分支追加新类型）
- Prompt 模板可调整（`prompts/sniff.rs`）

**对应代码**:
- `src-tauri/src/application/reminder_parser.rs`（`sniff_suggestions` 函数 + `Suggestion` 结构）
- `src-tauri/src/application/prompts/sniff.rs`（嗅探 Prompt 模板）
- `src-tauri/src/application/commands.rs`（`sniff_suggestions` 命令入口）

### 周报/月报生成

**能力定义**: 基于便签列表调用 AI 生成周报/月报 Markdown 草稿，按四个板块（重点/已完成/进行中/零散记录）输出。

**业务规则**:
- 未配置 AI（api_key 为空）时返回错误（"AI 未配置"），不静默跳过
- 数据拾取：按 updated_at 倒序，上限 20 条，每条取 content 前 200 字符
- 便签按 updated_at 日期部分（前 10 字符）过滤在 [start_date, end_date] 范围内
- 标题自动生成：周报 `YYYY-MM-DD ~ MM-DD 周报`，月报 `YYYY-MM 月报`
- 不修改便签/提醒数据，不触发自动同步

**变化点**:
- Prompt 模板可调整（`prompts/report.rs`）
- 报告板块结构可调整（当前四板块）

**对应代码**:
- `src-tauri/src/application/report_generator.rs`（`generate_report` 函数 + `ReportPeriod`/`ReportDraft` 结构）
- `src-tauri/src/application/prompts/report.rs`（报告 Prompt 模板）
- `src-tauri/src/application/commands.rs`（`generate_report` 命令入口）

### AI 文本重写

**能力定义**: 通过右键菜单对选中文本执行 5 种 AI 重写操作（规整/转清单/更正式/更精简/更温和），结果直接替换选中文本。

**业务规则**:
- 未配置 AI（api_key 为空）时返回错误，不静默跳过
- 选中文本长度 < 5 字符时返回错误（前端预检查 + 后端校验对齐）
- 支持 5 种操作：`tidy`（口语→书面）、`todo_split`（转待办清单）、`style_formal`（更正式）、`style_concise`（更精简）、`style_mild`（更温和）
- 前端支持编辑模式（textarea 选区）和查看模式（window.getSelection）双模式
- 替换后自动保存，支持 Ctrl+Z 撤销

**变化点**:
- 操作类型可扩展（`RewriteOperation` 枚举 + `prompts/rewrite.rs`）
- Prompt 模板可调整

**对应代码**:
- `src-tauri/src/application/prompts/rewrite.rs`（`RewriteOperation` 枚举 + `build_rewrite_messages`）
- `src-tauri/src/application/commands.rs`（`ai_rewrite_text` 命令入口）
- `src/main.ts`（右键菜单 + `rewriteText` 前端逻辑）

### 待办清单智能排序

**能力定义**: 当便签内未完成待办（`- [ ]`）超过 3 条时，调用 AI 按紧急程度重新排序。

**业务规则**:
- 待办条目 ≤ 3 时返回错误（"无需 AI 排序"），不调用 AI
- 排序权重（从高到低）：紧急词 > 近期时间 > 中期时间 > 远期时间 > 一般事项
- AI 返回 JSON 字符串数组，后端用 `extract_json_array` 提取
- 排序结果数量必须与输入一致，否则前端提示不匹配并取消
- 排序后自动保存便签内容

**变化点**:
- 排序权重规则可调整（`prompts/sort.rs`）
- 触发阈值（当前 > 3）可调整

**对应代码**:
- `src-tauri/src/application/prompts/sort.rs`（`build_sort_messages` 排序 Prompt）
- `src-tauri/src/application/commands.rs`（`ai_sort_todos` 命令 + `extract_json_array` 辅助函数）
- `src/main.ts`（`extractTodoItems`/`applySortedTodos`/`setupTodoSortButton` 前端逻辑）

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
| 重复类型 | Daily/Weekly/Monthly(精确月)/LunarMonthly(农历月) | 可能新增更多重复类型 | 修改 `next_trigger()` + `lunar_calendar.rs` |
| 通知方式 | 系统通知 + 弹窗 | 可能加邮件/推送 | 新增通知通道模块 |
| 快捷键 | 可配置（3 个动作：new_note/show_all/toggle_hub） | 可能新增动作 | `shortcut_manager.rs` + `shortcut_config.json` |
| 标签管理 | 手动标签 + 数量/长度限制 | 可能自动标签/标签颜色 | `domain/note.rs` tags 字段 |
| 搜索方式 | FTS5 trigram tokenizer + LIKE 短查询回退 | 可能引入分词器优化中文匹配 | `sqlite_note_repo.rs` search_notes + `database.rs` FTS5 虚拟表 |
| 便签模板 | 用户自定义模板（内置 3 个默认种子） | 可能扩展模板分类 | `domain/template.rs` + `sqlite_template_repo.rs` |
| AI 嗅探建议类型 | reminder/todo_split/tidy/style/tag_suggest 5 种 | 可能新增更多建议类型 | `reminder_parser.rs` match 分支 + `prompts/sniff.rs` |
| 报告周期类型 | Weekly/Monthly 2 种 | 可能新增自定义周期 | `report_generator.rs` ReportPeriod 枚举 + `commands.rs` generate_report 参数 |
| AI 文本重写操作 | tidy/todo_split/style_formal/style_concise/style_mild 5 种 | 可能新增更多操作类型 | `prompts/rewrite.rs` RewriteOperation 枚举 + `commands.rs` ai_rewrite_text |
| 待办排序触发阈值 | > 3 条待办时触发 | 可能调整为可配置阈值 | `commands.rs` ai_sort_todos 阈值判断 + `main.ts` setupTodoSortButton |

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
| 2026-07-15 | 迭代一 v0.2.0：Note 新增 tags 字段 + 标签管理能力；NoteRepository 新增 search_notes；新增 search_notes/update_note_tags 命令；新增标签侧边栏/后端搜索/排序 | — | #FEAT-002 同步更新 constraints.md |
| 2026-07-15 | 迭代二 v0.3.0：待办清单/复选框交互（GFM task list checkbox 可点击切换状态，自动保存） | — | #FEAT-003 |
| 2026-07-15 | 迭代三 v0.4.0：Monthly 改精确日历月；新增 LunarMonthly 重复类型 + tyme4rs 农历库；新增日历视图（Hub 月历展示提醒分布）；ReminderRepository 新增 find_pending_by_date_range；新增 get_reminders_by_month 命令 | — | #FEAT-004 同步更新 constraints.md/glossary.md |
| 2026-07-15 | 迭代三 v0.4.1：日历视图 7 项增强——显示提醒标题/农历日期/状态区分色/便签活动蓝点/今天本周高亮/点击日期创建提醒/年视图切换；find_pending_by_date_range 改为 find_by_date_range（含所有状态）；新增 get_lunar_dates/get_notes_activity_by_month 命令；NoteRepository 新增 find_activity_by_month | — | #FEAT-005 同步更新 constraints.md |
| 2026-07-16 | AI 嗅探扩展 4 种建议类型（todo_split/tidy/style/tag_suggest）；新增"AI 嗅探"支撑能力描述；扩展点分析表新增 AI 嗅探建议类型扩展点 | — | #FEAT-006 |
| 2026-07-16 | 新增"周报/月报生成"支撑能力（report_generator.rs + prompts/report.rs + generate_report 命令）；扩展点分析表新增报告周期类型扩展点；IPC 命令数修正为 42（历史不一致修正，以代码为准） | — | #FEAT-007 |
| 2026-07-17 | 新增"AI 文本重写"支撑能力（prompts/rewrite.rs + ai_rewrite_text 命令，5 种操作：tidy/todo_split/style_formal/style_concise/style_mild）；新增"待办清单智能排序"支撑能力（prompts/sort.rs + ai_sort_todos 命令，待办 > 3 时触发）；IPC 命令数 42 → 44 | — | #FEAT-008 |
| 2026-07-17 | v0.8.0：新增"批量操作"支撑能力（batch_archive_notes/batch_delete_notes/batch_update_color 命令）；删除 NoteColor 枚举（color 改为纯 String，前端定义快捷颜色）；IPC 命令数 44 → 47 | — | #FEAT-010 同步更新 constraints.md |
| 2026-07-18 | v0.8.1：搜索改用 FTS5 trigram tokenizer + LIKE 短查询回退 + snippet 高亮；新增"便签模板"能力（Template 领域模型 + TemplateRepository + 4 个命令 get_templates/save_template/delete_template/create_note_from_template + 默认种子）；快捷键新增 toggle_hub 动作（3 个动作）；IPC 命令数 47 → 51 | — | #FEAT-011 同步更新 constraints.md/glossary.md |
| 2026-07-18 | 模板能力扩展：模板 Git 同步（sync_json_io export/import 增加 templates 目录 + updated_at 仲裁）；搜索高亮修复（snippet 三列选择 + 选第一个含 `<mark>` 的）；新增 UI 入口三处——设置中心模板管理弹窗、便签右键菜单"从模板新建"、空便签编辑区顶部模板快捷条；新增 INV-023（模板必须 Git 同步） | — | #FEAT-012 同步更新 constraints.md/glossary.md |
| 2026-07-19 | 补充 delete_note 窗口关闭行为 | AI | v0.8.5 |
| 2026-07-18 | UI 修复：i18n 命名空间错误（tpl 键从 hub 移到 note）；模板快捷条 CSS 改为横向单行滚动（不换行不挤压内容区）；右键菜单改为两项并存——「从模板新建便签」+「应用模板到当前便签」（追加到末尾，非破坏性）；新增 showTemplateApplier；应用图标替换为 TIE 字母图标（替换 src-tauri/icons 全部 35 个文件） | — | #FEAT-013 同步更新 constraints.md/glossary.md |
