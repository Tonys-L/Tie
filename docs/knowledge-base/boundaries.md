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
- delete_note 命令职责：删除便签及关联提醒；删除成功后调用 `window_manager::close_note_window` 关闭窗口（内部封装 `destroy()` 强制销毁，用 destroy 而非 close：close 可能因 Tauri 2.x 事件时序问题失败，destroy 是强制销毁，确保窗口立即消失）
- 搜索使用 FTS5 trigram tokenizer（CJK 子串匹配），查询 < 3 字符回退 LIKE
- 搜索结果支持高亮片段（FTS5 snippet 生成 `<mark>` 标签）

**变化点**:
- 前端渲染方式（当前 Markdown + 待办清单交互，未来可能富文本）
- 颜色选项扩展
- 搜索 tokenizer（当前 trigram，未来可能引入分词器优化中文匹配）

**对应代码**:
- `src-tauri/src/domain/note.rs`（领域模型，含 tags 字段 + set_tags/add_tag/remove_tag + highlight 搜索高亮片段 + is_empty/is_reminder_eligible 业务查询方法）
- `src-tauri/src/domain/repositories.rs`（NoteRepository trait，含 search_notes）
- `src-tauri/src/application/commands/`（命令入口模块，含 search_notes/update_note_tags，按业务域拆分为 7 个子模块）
- `src-tauri/src/application/image_service.rs`（图片文件名提取、孤儿图片清理、图片目录管理，被 commands/image_commands 和 commands/note_commands 复用；`image_dir` 基础路径委托 `application::paths::data_dir_path` 单一所有者，只追加 sync/images 子目录）
- `src-tauri/src/application/paths.rs`（`data_dir_path` 路径解析单一所有者：`exe 同级目录/data`，不做 canonicalize 避开 create_dir_all 前路径不存在失败；供 `commands::system_commands::data_dir_path` 转发、`lib.rs` setup、`image_service::image_dir` 共用，禁止他处内联重复解析）
- `src-tauri/src/application/note_service.rs`（便签编排：create_note 创建+开窗口（薄封装委托 create_note_with_deps 处理 save+emit，核心写逻辑脱离 AppHandle 可单测，仿 INV-028 模式）、close_note_if_empty 空便签自动删除 INV-003、open_note_with_flag 委托 window_manager::focus_note_window_and_emit、update_note_style 委托 window_manager::set_note_pinned、update_note_content 含孤儿图片清理、update_note_title/update_note_window_state/update_note_tags 单字段更新、archive_note/unarchive_note 状态切换、delete_note 删除前清理便签内容中的图片文件（单/批量路径统一，locality 下沉）+ 存在性守卫（find_by_id 返回 None 时 Ok(()) 不 emit 事件，LES-023）、batch_archive/batch_unarchive/batch_delete（内部循环调用 delete_note 自动触发图片清理，batch_delete 预检查存在性避免幂等 delete 错误计入 succeeded）/batch_update_color 批量操作返回成功 id 列表供命令层 emit；所有写操作完成后 emit `NoteWritten` 事件（ADR-007），由 lib.rs 监听器统一触发 schedule_auto_sync）
- `src-tauri/src/application/template_service.rs`（模板编排：save_template 通过 find_by_id 判断 Created/Updated、delete_template 加存在性守卫（find_by_id 返回 None 时 Ok(()) 不 emit 事件，LES-023）、create_note_from_template 查模板→建 Note→开窗→emit NoteWritten(Created)；所有写操作 emit `TemplateWritten`/`NoteWritten` 事件）
- `src-tauri/src/application/event_bus.rs`（事件总线：DomainEvent 枚举 + WriteAction 枚举 + EventPublisher trait + EventBus 同步实现 + MockEventPublisher 测试工具；解耦 service 层写操作与 schedule_auto_sync 副作用，详见 ADR-007）
- `src-tauri/src/application/window_manager.rs`（Note 窗口生命周期管理：open_note_window/open_note_window_with_url 创建、activate_note_for_reminder 提醒触发、restore_all_windows 启动恢复（空便签删除委托 note_service::delete_note，消除漏 emit NoteWritten(Deleted) 事件，LES-023）、close_note_window 销毁、set_note_pinned/restore_note_on_top 置顶、focus_note_window_and_emit 聚焦+事件、flash_window 闪烁；ADR-009 拆分后仅聚焦 note 窗口，Hub 窗口逻辑已迁到 hub_window_manager，重叠物理计算已迁到 window_overlap_resolver）
- `src-tauri/src/application/hub_window_manager.rs`（Hub 窗口管理：toggle_hub_window 切换可见性、open_or_focus_hub 托盘菜单调用、create_hub_window 统一创建入口；ADR-009 从 window_manager 拆出，tray_manager/shortcut_manager 委托本模块）
- `src-tauri/src/application/window_overlap_resolver.rs`（窗口重叠解析器：compute_overlaps 纯函数计算同位置便签级联偏移 30px + resolve_overlaps 执行 Tauri set_position 副作用；ADR-009 从 window_manager 拆出，纯函数部分无 Tauri 依赖可独立单测，5 个单测覆盖无重叠/2 同位/3 同位/多组独立/空输入场景）
- `src-tauri/src/infrastructure/database.rs`（FTS5 虚拟表 + 触发器迁移）
- `src-tauri/src/infrastructure/sqlite_note_repo.rs`（search_notes 实现：FTS5 MATCH + snippet + LIKE 短查询回退）
- `src/main.ts`（便签窗口前端入口，纯编排：initNoteWindow + setupNoteEvents 编排 5 个 bind 子函数 + 全局事件监听；1903→390 行，业务实现拆分至 15 个模块）
- `src/hub.ts`（Hub 前端入口，纯编排：页面切换 + 主题/语言 + 初始加载 + visibilitychange/focus 刷新；1441→131 行，业务实现拆分至 12 个模块）
- `src/note-renderer.ts`（renderNote 渲染便签到 DOM，callback 模式注入 setupEvents 避免循环依赖）
- `src/colors.ts`（COLOR_MAP/COLORS 颜色映射 + applyNoteStyle，被 note-renderer 与 context-menu 共享以破循环依赖；从原 note-style.ts 拆出，ADR-009 同批次；formatNoteTime 已迁到 datetime.ts，职责错位修正）
- `src/datetime.ts`（localISO/formatDate/formatNoteTime/quickDate/repeatLabel 日期时间工具，被 reminder-dialog/calendar-view/notes-list/note-renderer 等共享；从原 utils.ts 拆出，ADR-009 同批次；formatNoteTime 从 colors.ts 迁入）
- `src/toast.ts`（showToast 统一 toast 实现，被所有页面共享；从原 utils.ts 拆出，ADR-009 同批次）
- `src/html.ts`（escapeHtml HTML 转义工具，被 markdown-renderer 等共享；从原 utils.ts 拆出，ADR-009 同批次）
- `src/ai-client.ts`（AI 调用统一入口：isAiConfigured/getAiConfigCached 带 5 秒缓存 + ai-config-changed 事件清缓存 + runAi 统一 loading/success/error toast 包装；被 context-menu/ai-sniff/ai-todo-sort/ai-rewrite 复用；ADR-009 同批次）
- `src/context-menu.ts`（右键菜单 + showCustomColorPanel 自定义颜色面板）
- `src/note-context.ts`（便签上下文 state：getNote/setNote/setCurrentReminderId）
- `src/window-state.ts`（窗口事件 setupWindowEvents + setClosing）
- `src/tag-bar.ts`（标签栏 UI + add/remove tag）
- `src/title-edit.ts`（标题进入/退出编辑）
- `src/delete-confirm.ts`（删除确认弹窗：`showDeleteConfirm(noteId, target, onDeleted?)` 双模式——便签窗口模式（无 onDeleted，append 到 app，删除后依赖后端 destroy 关闭窗口；失败时降级 close）+ Hub 列表模式（onDeleted=loadNotes，append 到 body，删除成功后刷新列表）；统一便签窗口与 Hub 列表两处重复实现）
- `src/markdown-renderer.ts`（Markdown 渲染 + escapeHtml）
- `src/image-resize.ts`（图片缩放手柄）
- `src/reminder-panel.ts`（便签内提醒面板 showReminderPanel）
- `src/ai-sniff.ts`（AI 嗅探按钮 + sniffAfterSave）
- `src/ai-rewrite.ts`（AI 文本重写 rewriteText）
- `src/ai-todo-sort.ts`（AI 待办排序 extractTodoItems/applySortedTodos/setupTodoSortButton + clearSortedMark）
- `src/template-ui.ts`（便签内模板快捷条 setupTemplateQuickBar + showTemplatePicker/showTemplateApplier）
- `src/datetime-picker.ts`（时间选择器组件，含确认按钮）
- `src/notes-list.ts`（Hub 便签列表：load/render/search/sort/标签侧边栏/列表事件委托（归档/恢复/提醒/删除/单击打开/Ctrl+多选）；416→289 行，多选状态与批量操作栏下沉到 notes-multiselect.ts，内联 showDeleteConfirm 改调 delete-confirm.ts；导出 getActiveNotes/getArchivedNotes 供 calendar-view 复用）
- `src/notes-multiselect.ts`（Hub 便签多选与批量操作：selectedIds state + 批量操作栏 5 按钮事件（归档/恢复/删除/改色/取消）+ updateMultiSelectUI + Esc 退出多选；通过 `initMultiSelect({ reloadList, getCurrentTab })` 注入依赖打破与 notes-list 的循环依赖；导出 toggleSelection/clearSelection/hasSelection/refreshSelectionUI 供 notes-list 调用）
- `src/calendar-view.ts`（Hub 日历视图：月/年视图 + 日详情，依赖 notes-list 的 getter）
- `src/reminder-dialog.ts`（Hub 内嵌提醒设置弹窗，callback 模式 onNotesChanged 避免循环依赖）
- `src/ai-settings.ts`（Hub AI 配置 + 周报/月报生成）
- `src/template-manager.ts`（Hub 模板管理弹窗 CRUD，side-effect 模块）
- `src/sync-settings.ts`（Hub Git 同步配置 + 分支创建对话框）
- `src/general-settings.ts`（Hub 通用设置 loadGeneralSettings）
- `src/shortcut-settings.ts`（Hub 快捷键配置 loadShortcutConfig）
- `src/update-check.ts`（Hub 更新检查，side-effect 模块）

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
- `src-tauri/src/application/commands/`（get_templates/save_template/delete_template/create_note_from_template 命令薄壳，位于 template_commands.rs，调用 template_service 完成业务+emit 事件）
- `src-tauri/src/application/sync_json_io.rs`（模板导出/导入 + updated_at 仲裁）
- `src-tauri/src/application/git_sync.rs`（sync/auto_pull_on_startup 传 template_repo）
- `src/template-manager.ts`（Hub 模板管理弹窗 CRUD + 从模板新建便签，side-effect 模块）
- `src/template-ui.ts`（便签内空便签模板快捷条 setupTemplateQuickBar + 右键菜单 showTemplatePicker/showTemplateApplier）

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
- `src-tauri/src/domain/reminder.rs`（领域模型 + 状态机 + 状态转换前置校验（INV-031：mark_triggered/snooze/mark_done/cancel 返回 `Result<(), String>`，终态 Done/Cancelled 拒绝所有转换，Triggered 拒绝重复 mark_triggered；Triggered→Pending via snooze 合法；advance_state 内部调用 mark_triggered 用 expect 表达"仅对 Pending 调用"契约）+ `effective_time()` 单一真相源（snoozed_until 存在时返回 snoozed_until，否则返回 remind_at；is_due/find_next_due_time/find_by_date_range 统一委托，INV-008 扩展）+ `notification_title`/`notification_body` 通知展示方法：空标题 fallback "便签提醒"、content 80 字截断 + 省略号 + 空内容 fallback "点击查看便签"）
- `src-tauri/src/application/reminder_scheduler.rs`（事件驱动调度：单定时器 + Notify + fire_reminders_with_deps 触发编排；ReminderNotifier trait + TauriReminderNotifier 实现可注入 mock 测试；状态推进委托 Reminder::advance_state；归档便签判断委托 Note::is_reminder_eligible；通知标题/正文委托 Reminder::notification_title/notification_body；**ADR-008 扩展**：fire_reminders_with_deps 接收 `&dyn EventPublisher` + `&dyn ReminderQuery` 参数，save 推进状态后 emit `ReminderWritten(Updated)` 事件，由 lib.rs 监听器统一触发 `schedule_recalc` + `schedule_auto_sync`；**ADR-010**：start() 循环用 `state.reminder_query.find_next_due_time`，fire_reminders_with_deps 内部 `reminder_query.find_due` 替代 `reminder_repo.find_due`）
- `src-tauri/src/application/reminder_service.rs`（提醒 CRUD 编排：create_reminder/snooze_reminder/dismiss_reminder/delete_reminder，接收 `&dyn ReminderRepository` + `&dyn EventPublisher` 无 Tauri 依赖可单测；delete_reminder 加存在性守卫（find_by_id 返回 None 时 Ok(None) 不 emit 事件，LES-023）；所有写操作完成后 emit `ReminderWritten` 事件（ADR-007），由 lib.rs 监听器统一触发 `schedule_auto_sync` + `schedule_recalc`（ADR-008），命令层不再手动调用 `schedule_recalc`）
- `src-tauri/src/application/lunar_calendar.rs`（农历计算统一入口：`TymeCalendarAdapter` impl CalendarAdapter（LunarMonthly 重复计算）+ `lunar_date_text` 公开函数（农历日文本，初一返回"月份+日"，其他日只返回"日"）；命令层和调度器都不再直接 use tyme4rs，消除 shallow module）
- `src-tauri/src/application/commands/`（提醒命令薄壳，位于 reminder_commands.rs：调用 reminder_service 完成业务 → 执行 emit 副作用；`get_lunar_dates` 命令调用 `lunar_calendar::lunar_date_text` 而非内联 tyme4rs；schedule_auto_sync + schedule_recalc 由 service emit 事件触发，命令层不再手动调用（ADR-007 + ADR-008））

---

### 数据同步

**能力定义**: 基于 Git 仓库的多设备数据同步，JSON 文件为传输载体。

**业务规则**:
- SQLite 为运行时存储，JSON 文件为同步载体
- 冲突解决：last-write-wins，按 updated_at 取最新
- push 策略：--force-with-lease
- 自动同步防抖：30 秒延迟
- 写操作副作用解耦（ADR-007）：service 层完成写操作后 emit `DomainEvent`，lib.rs setup 注册监听器接收事件并触发 `schedule_auto_sync`；命令层不再手动调用 `schedule_auto_sync`，消除散布问题与漏调风险（INV-013）

**变化点**:
- 同步协议（当前 Git HTTPS，未来可能其他）
- 冲突解决策略（当前 last-write-wins，未来可能语义级合并）
- 事件总线实现（当前同步 `EventBus`，未来可替换为 `ChannelPublisher` 异步实现，仅实现 `EventPublisher` trait 即可）

**对应代码**:
- `src-tauri/src/application/git_sync.rs`（GitSync struct + sync() 编排 + 调度；`sync_with_notification` 自由函数统一 sync+通知格式，被 sync_commands::sync_notes 和 tray_manager::handle_sync 共用）
- `src-tauri/src/application/sync_config.rs`（SyncConfig + 配置读写 + 认证 URL）
- `src-tauri/src/application/sync_json_io.rs`（DB ↔ JSON 文件转换：`export_to_json`/`import_from_json` 公开入口 + `export_entity_to_json`/`import_entity_from_json` 泛型实现消除三重重复 + `extract_updated_at` 纯字符串解析供 git_ops 冲突解决复用）
- `src-tauri/src/application/git_ops.rs`（Git 子进程执行 + 冲突解决；`pub const CREATE_NO_WINDOW: u32 = 0x08000000` 公开常量，供 git_sync/sync_commands 等模块统一引用，消除 magic number 散布）
- `src-tauri/src/application/event_bus.rs`（事件总线：`DomainEvent`/`WriteAction`/`EventPublisher` trait + `EventBus` 同步实现 + `MockEventPublisher` 测试工具；service 层 emit 事件，lib.rs setup 注册两条监听器：所有 `DomainEvent` → `schedule_auto_sync`（ADR-007），`ReminderWritten` → `schedule_recalc`（ADR-008 扩展）；详见 ADR-007 + ADR-008）
- `src-tauri/src/lib.rs`（AppState 含 `event_bus: Arc<EventBus>` 字段 + `note_query: Box<dyn NoteQuery>` + `reminder_query: Box<dyn ReminderQuery>` 字段（ADR-010）；setup 中创建 EventBus + 注册两条监听器；on_window_event 中 close_note_if_empty 传 event_bus 引用）

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
- 45 个命令集中在 `application/commands` 模块（按业务域拆分为 9 个子模块：note/reminder/sync/shortcut/locale/system/ai/template/image；原 sync_commands 已按业务能力拆为 sync/shortcut/locale/system 四个子模块，ADR-009 同批次）
- `application::paths::data_dir_path()` 是 `exe 同级目录/data` 路径解析的单一所有者，`commands::system_commands::data_dir_path` 转发本函数；供 `get_data_dir`/`open_data_dir` 命令、`lib.rs` setup、`image_service::image_dir` 共用，禁止他处内联重复解析
- 可能并发的命令必须 `async` 避免死锁

### 前端多页面边界

- `index.html` → 便签窗口入口（`src/main.ts`，纯编排 390 行）
- `hub.html` → 设置中心入口（`src/hub.ts`，纯编排 131 行）
- 共享模块：`src/types.ts`（类型定义）、`src/api.ts`（IPC 封装）、`src/i18n.ts`（国际化）、`src/colors.ts`（颜色映射 + applyNoteStyle）、`src/datetime.ts`（日期时间工具 localISO/formatDate/formatNoteTime/quickDate/repeatLabel）、`src/toast.ts`（showToast 统一实现）、`src/html.ts`（escapeHtml）、`src/ai-client.ts`（AI 调用统一入口 isAiConfigured/getAiConfigCached/runAi，ADR-009 同批次）
- **入口文件编排约束**：main.ts 与 hub.ts 仅负责编排（init/load/页面切换/全局事件），具体业务实现拆分至独立模块；函数 < 100 行；每个模块文件含 JSDoc 头（职责/被调用方/依赖）
- **单向依赖约束**：模块间禁止循环依赖。当前依赖方向：main.ts → note-renderer → colors；context-menu → colors；hub.ts → notes-list → reminder-dialog；calendar-view → notes-list；notes-list → notes-multiselect + delete-confirm
- **callback 模式约束**：跨模块回调避免循环依赖。`renderNote(note, setupEventsCallback)`、`showReminderDialog(noteId, noteTitle, onNotesChanged)`、`showTemplateDialog(title, app, onSelect)`、`initMultiSelect({ reloadList, getCurrentTab })`（notes-multiselect 通过 callback 接收 notes-list 的刷新/tab 状态，避免反向 import）
- **side-effect 模块约束**：`import './template-manager'` 和 `import './update-check'` 顶层执行按钮绑定，无 named export
- **state 就近原则**：模块级私有 state + getter/setter 导出。`note-context.ts`（getNote/setNote）、`notes-list.ts`（getActiveNotes/getArchivedNotes）、`sync-settings.ts`（syncConfigLoaded 防重复绑定）
- **IPC seam 约束**：所有前端模块均通过 `import * as api from './api'` 调用后端命令，禁止直接 `invoke('xxx', ...)`（`convertFileSrc` 等 Tauri SDK 非 invoke API 不在此限）
- **Toast 统一约束**：两页面均使用 `toast.ts` 的 `showToast`，签名 `(message, type: 'info'|'success'|'error', persistent?)`；不再保留各自的本地实现（原 utils.ts 已拆分，showToast 迁到 toast.ts）
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
- `src-tauri/src/application/reminder_parser.rs`（`sniff_suggestions` async 函数 + `build_suggestions` 纯函数（类型分发包装，无 I/O 可独立单测）+ `Suggestion` 结构；JSON 对象提取委托 `application::json_extract::extract_object`）
- `src-tauri/src/application/json_extract.rs`（AI 返回文本 JSON 片段提取工具：`extract_object` 提取 `{` 开头对象 + `extract_array` 提取 `[` 开头数组；用 `Deserializer::into_iter::<IgnoredAny>` 流式解析 + `byte_offset` 精确边界，正确处理字符串值内含 `{`/`}`/`[`/`]` 字符；被 ai_commands 和 reminder_parser 共用，消除两处独立实现）
- `src-tauri/src/application/prompts/sniff.rs`（嗅探 Prompt 模板）
- `src-tauri/src/application/commands/`（`sniff_suggestions` 命令入口，位于 ai_commands.rs，通过 `AiConfig::load_default()` 加载配置）
- `src-tauri/src/application/ai_config.rs`（`AiConfig` 配置管理：`load_default` 便捷加载 + `default_path` + `load` + `save` + `is_configured`，消除命令层 7 处重复加载；`is_configured` 报错语义单一检查点为 `AiService::call`，`sniff_suggestions` 为静默语义唯一例外）

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
- `src-tauri/src/application/report_generator.rs`（`generate_report` 函数 + `filter_notes_by_date` 公开纯函数（按 updated_at 日期部分过滤）+ `parse_period` 公开纯函数（period_type 字符串解析为 ReportPeriod 枚举，消除命令层 28 行内联解析）+ `ReportPeriod`/`ReportDraft` 结构）
- `src-tauri/src/application/prompts/report.rs`（报告 Prompt 模板）
- `src-tauri/src/application/commands/`（`generate_report` 命令入口，位于 ai_commands.rs，调用 `parse_period` 解析周期 + `filter_notes_by_date` 完成日期过滤后传入 `generate_report`）

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
- `src-tauri/src/application/ai_validation.rs`（`validate_rewrite_text` 校验 5~500 字符边界，命令层调用前校验消除内联逻辑）
- `src-tauri/src/application/commands/`（`ai_rewrite_text` 命令入口，位于 ai_commands.rs，通过私有 `ai_call_raw` 统一 AI 调用链）
- `src/ai-rewrite.ts`（`rewriteText` 前端逻辑，编辑/查看双模式选区处理）
- `src/context-menu.ts`（右键菜单入口，调用 ai-rewrite）

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
- `src-tauri/src/application/ai_validation.rs`（`validate_sort_todos` 校验待办 ≤3 拒绝，命令层调用前校验消除内联逻辑）
- `src-tauri/src/application/commands/`（`ai_sort_todos` 命令入口，位于 ai_commands.rs，通过私有 `ai_call_raw` 统一 AI 调用链；JSON 数组提取委托 `application::json_extract::extract_array`）
- `src/ai-todo-sort.ts`（`extractTodoItems`/`applySortedTodos`/`setupTodoSortButton`/`clearSortedMark` 前端逻辑）

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
| 重复类型 | Daily/Weekly/Monthly(精确月)/LunarMonthly(农历月) | 可能新增更多重复类型 | 修改 `Reminder::next_trigger()` 返回 `NextTrigger::DateTime/External`；若需外部计算则实现 `CalendarAdapter` trait |
| 通知方式 | 系统通知 + 弹窗 | 可能加邮件/推送 | 实现 `ReminderNotifier` trait 注入 `fire_reminders_with_deps` |
| 快捷键 | 可配置（3 个动作：new_note/show_all/toggle_hub） | 可能新增动作 | `shortcut_manager.rs` + `shortcut_config.json` |
| 标签管理 | 手动标签 + 数量/长度限制 | 可能自动标签/标签颜色 | `domain/note.rs` tags 字段 |
| 搜索方式 | FTS5 trigram tokenizer + LIKE 短查询回退 | 可能引入分词器优化中文匹配 | `sqlite_note_repo.rs` search_notes + `database.rs` FTS5 虚拟表 |
| 便签模板 | 用户自定义模板（内置 3 个默认种子） | 可能扩展模板分类 | `domain/template.rs` + `sqlite_template_repo.rs` |
| 同步实体 | Note/Reminder/Template 三种（分别对应 notes/reminders/templates 目录） | 可能新增同步实体（如标签定义、设置项） | `application/sync_json_io.rs` `export_entity_to_json`/`import_entity_from_json` 泛型函数 + 调用处追加一行 |
| AI 嗅探建议类型 | reminder/todo_split/tidy/style/tag_suggest 5 种 | 可能新增更多建议类型 | `reminder_parser.rs` `build_suggestions` match 分支 + `prompts/sniff.rs` |
| 报告周期类型 | Weekly/Monthly 2 种 | 可能新增自定义周期 | `report_generator.rs` ReportPeriod 枚举 + `commands.rs` generate_report 参数 |
| AI 文本重写操作 | tidy/todo_split/style_formal/style_concise/style_mild 5 种 | 可能新增更多操作类型 | `prompts/rewrite.rs` RewriteOperation 枚举 + `commands.rs` ai_rewrite_text |
| 待办排序触发阈值 | > 3 条待办时触发 | 可能调整为可配置阈值 | `commands.rs` ai_sort_todos 阈值判断 + `src/ai-todo-sort.ts` setupTodoSortButton |
| 仓储 trait 分离 | NoteRepository/NoteQuery + ReminderRepository/ReminderQuery（CQRS 风味，ADR-010） | 可能新增 CachedNoteQuery 装饰器或只读副本 | 实现 `NoteQuery`/`ReminderQuery` trait 新实现，service/scheduler 签名零改动 |
| 事件总线实现 | 同步 `EventBus`（ADR-007 + ADR-008） | 可能切异步 channel | 新增 `ChannelPublisher` impl `EventPublisher`，service/scheduler 签名零改动 |

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
| 2026-07-19 | 合并 reminder_service 到 reminder_scheduler（删除 reminder_service.rs）；fire_reminders 成为 reminder_scheduler 的 pub fn；对应代码列表合并为单条；消除 pass-through 浅模块 | — | #REFACTOR-015 同步更新 constraints.md |
| 2026-07-19 | 拆分 commands.rs（814 行）为 commands/ 目录 7 个子模块（note/reminder/sync/ai/template/image/mod）；新增 application/image_service.rs（图片文件名提取/孤儿清理/目录管理，从 commands 下沉为 service）；commands/mod.rs 用 `pub use *` glob 重导出（保留 `#[tauri::command]` 生成的 `__cmd__xxx` 辅助项，lib.rs 调用路径 `commands::xxx` 不变）；更新对应代码列表中所有 commands.rs 引用 | AI | #REFACTOR-017 同步更新 constraints.md |
| 2026-07-19 | 新增 batch_unarchive_notes 命令（批量恢复便签，补全批量操作能力：archive/unarchive/delete/update_color 四类齐全）；hub.ts 批量恢复从 Promise.all 逐个调用改为单次 IPC 批量命令；api.ts 新增 batchUnarchiveNotes 封装；IPC 命令数 44 → 45 | AI | #REFACTOR-018 |
| 2026-07-19 | 深化 fire_reminders：domain/reminder.rs 新增 `NextTrigger` enum（None/DateTime/External）+ `CalendarAdapter` trait + `Reminder::advance_state` 方法；删除 `reset_for_next_trigger`（被 advance_state 替代）；application/reminder_scheduler.rs 新增 `ReminderNotifier` trait + `TauriReminderNotifier` 实现 + `fire_reminders_with_deps` 可测试入口；application/lunar_calendar.rs 新增 `TymeCalendarAdapter` impl CalendarAdapter；LunarMonthly 不再绕过 domain seam，由 trait 注入计算；新增 11 个测试（6 个 advance_state + 5 个 fire_reminders_with_deps mock）；182 个测试全部通过 | AI | #REFACTOR-019 同步更新 constraints.md/flows.md |
| 2026-07-19 | Candidate 02 窗口操作统一到 window_manager：新增 `close_note_window`/`set_note_pinned`/`restore_note_on_top`/`focus_note_window_and_emit` 4 个 pub fn；替换 `note_commands::delete_note`/`batch_delete_notes`/`restore_window_on_top` 和 `note_service::update_note_style`/`open_note_with_flag` 5 处直接 `app.get_webview_window().destroy()/set_always_on_top()/set_focus()+emit()` 调用；note_commands.rs 移除 `Manager` 导入，note_service.rs 移除 `Manager`/`Emitter` 导入；更新对应代码描述（note_service/window_manager 职责清单）；扩展模块边界规则到 application 层所有模块；182 个测试全部通过 | AI | #REFACTOR-020 同步更新 constraints.md |
| 2026-07-19 | Candidate 03 sync_json_io 泛化：新增私有泛型函数 `export_entity_to_json`（序列化+写文件）和 `import_entity_from_json`（反序列化+last-write-wins 仲裁）；用泛型 + 闭包参数化差异点（id/updated_at/find_by_id/save），消除 Note/Reminder/Template 三种实体的导出/导入三重重复；`export_to_json` 从 33 行降至 21 行，`import_from_json` 从 82 行降至 35 行；`R: ?Sized` 约束支持 `&dyn Trait` 传入；调用处用 turbofish `::<Note, dyn NoteRepository>` 明确泛型参数；扩展点分析表新增"同步实体"扩展点；未新增 INV（纯重构，无新业务不变量）；182 个测试全部通过 | AI | #REFACTOR-021 同步更新 constraints.md |
| 2026-07-19 | Candidate 04 Note 聚合根完善：domain/note.rs 新增 `is_empty()`（title+content 均空，INV-003）和 `is_reminder_eligible()`（!is_archived，归档不触发提醒）2 个业务查询方法；window_manager::restore_all_windows 和 note_service::close_note_if_empty 2 处 `note.title.is_empty() && note.content.is_empty()` 散布判断替换为 `note.is_empty()`；reminder_scheduler::fire_reminders_with_deps 1 处 `note.is_archived` 判断替换为 `!note.is_reminder_eligible()`；更新对应代码描述（note.rs 新增业务查询方法清单 + reminder_scheduler 委托说明）；新增 6 个 domain 单元测试覆盖 4 种 is_empty 边界 + 2 种 is_reminder_eligible 状态；188 个测试全部通过 | AI | #REFACTOR-022 同步更新 constraints.md |
| 2026-07-19 | Candidate 05 AI 配置管理统一：ai_config.rs 新增 `load_default()` 便捷方法（封装 `default_path() + load`）；ai_commands.rs 7 处 `let path = AiConfig::default_path(); let config = AiConfig::load(&path)?;` 重复模式替换为 `AiConfig::load_default()?`（get_ai_config/test_ai_connection/parse_reminder_natural/sniff_suggestions/generate_report/ai_rewrite_text/ai_sort_todos）；save_ai_config 保留 `default_path()` 调用（写入场景非加载）；新增 use `super::super::ai_config::AiConfig` 简化引用；ai_commands.rs 从 205 行降至 192 行；未新增 INV（纯重构）；新增 1 个单元测试覆盖 load_default；189 个测试全部通过 | AI | #REFACTOR-023 |
| 2026-07-19 | Candidate 06 命令层可测试性提升：新建 `application/reminder_service.rs` 承载 CRUD 编排（create_reminder/snooze_reminder/dismiss_reminder/delete_reminder），接收 `&dyn ReminderRepository` 无 Tauri 依赖可单测；reminder_commands.rs 4 处命令改为薄壳（调用 service → 统一副作用：scheduler.schedule_recalc + app.emit + git_sync.schedule_auto_sync）；snooze/dismiss 副作用模式统一（原 snooze/dismiss 直接 return save 结果，现改为先 service 完成业务再执行副作用）；application/mod.rs 新增 `pub mod reminder_service`；新增 9 个 service 单元测试覆盖 create/snooze/dismiss/delete + not_found 场景；198 个测试全部通过 | AI | #REFACTOR-024 同步更新 constraints.md |
| 2026-07-19 | Candidate 07 resolve_overlaps 抽取为纯函数：window_manager.rs 新增私有纯函数 `compute_overlaps`（接收 `&[&Note]` 返回 `Vec<(note_id, new_pos_x, new_pos_y)>`，无 Tauri 依赖可单测）；`resolve_overlaps` 重构为"调用 compute_overlaps → 遍历执行 set_position 副作用"两步；新增 4 个单元测试覆盖无重叠/2 同位/3 同位/多组独立重叠场景；window_manager.rs 从 0 测试覆盖提升到 4 个纯函数测试；202 个测试全部通过 | AI | #REFACTOR-025 |
| 2026-07-19 | Candidate 09 消除 note_service::sync_notes pass-through：删除 `note_service::sync_notes` 函数（纯 pass-through 无业务逻辑）；`sync_commands::sync_notes` 和 `tray_manager::handle_sync` 2 处调用方改为直接调用 `state.git_sync.sync(...)`；note_service.rs 移除 `git_sync::GitSync` 和 `TemplateRepository` 导入；通过"删除测试"：删掉后复杂度消失无副作用；202 个测试全部通过 | AI | #REFACTOR-026 |
| 2026-07-20 | Candidate 11+12 note_commands 9 个命令下沉 note_service：note_service.rs 新增 6 个单字段更新函数（update_note_content 含孤儿图片清理 / update_note_title / update_note_window_state / update_note_tags / archive_note / unarchive_note）+ 4 个批量函数（batch_archive / batch_unarchive / batch_delete / batch_update_color，返回 `Vec<String>` 成功 id 列表供命令层 emit）；note_commands.rs 9 个命令改为薄壳（调用 service → emit/schedule_recalc/schedule_auto_sync/window_manager 副作用）；image_service::cleanup_removed_images 从 update_note_content/batch_delete 命令下沉到 service；新增 24 个 service 单测覆盖持久化/not_found/部分成功/空列表/级联删除/INV-019 去重/INV-027 clamp 场景；226 个测试全部通过（1 个 ai_config 沙箱文件访问失败与本次无关） | AI | #REFACTOR-028 同步更新 constraints.md |
| 2026-07-20 | Candidate 13 tray_manager 三处重复 + 约束违规：13a 抽取 `build_tray_menu` 私有函数，setup_tray 与 rebuild_tray_menu 共用（消除 15 行菜单构造重复）；13b git_sync.rs 新增 `sync_with_notification` 自由函数封装 sync+通知格式，sync_commands::sync_notes（async）与 tray_manager::handle_sync（spawn_blocking）共用；13c window_manager.rs 新增 `create_hub_window` 私有 + `open_or_focus_hub` 公开，tray_manager::handle_hub 委托（消除内联 WebviewWindowBuilder 约束违规）；附带修复 INV-013 违规：handle_new_note 补 `schedule_auto_sync` 调用；tray_manager.rs 从 161 行降至 126 行；227 个测试全部通过 | AI | #REFACTOR-029 同步更新 constraints.md |
| 2026-07-20 | Candidate 14 main.ts → api.ts 迁移：api.ts 补齐 7 个缺失封装（setLocale/getImageDir/restoreWindowOnTop/openUrl/saveImage/aiRewriteText/aiSortTodos）；main.ts 37 处直接 `invoke(...)` 全部替换为 `api.xxx(...)`，移除 `import { invoke } from '@tauri-apps/api/core'`（仅保留 `convertFileSrc`）；main.ts 类型导入清理 `Reminder`/`AiConfig`（已下沉到 api.ts）；utils.ts 新增统一 `showToast(message, type: 'info'\|'success'\|'error', persistent?)` 函数（含渐隐动画+持久化选项）；hub.ts 删除本地 showToast（16 处 `'ok'`/`'err'` 调用替换为 `'success'`/`'error'`，加入 utils 导入）；前端多页面边界新增 IPC seam 约束与 Toast 统一约束；TypeScript 编译通过；通过"删除测试"：api.ts 集中 invoke 调用，前端模块不感知 Tauri 命令名 | AI | #REFACTOR-030 |
| 2026-07-20 | Candidate 15 ai_commands AI 调用链内联：ai_commands.rs 新增私有函数 `ai_call_raw(messages: Vec<ChatMessage>) -> Result<String, String>` 封装 `load_default + is_configured + AiService::new + call + map_err` 链；`test_ai_connection` 改为 `ai_call_raw(vec![ChatMessage::user("ping")])`（1 行）；`ai_rewrite_text`/`ai_sort_todos` 删除内联 `load_default + is_configured + AiService::new` 重复，改为调用 `ai_call_raw(messages)`；解析逻辑（trim/JSON 数组提取）保留各命令；附带删除 `AiService::test_connection` 方法及其测试（与 ai_call_raw 重复，通过"删除测试"：删掉后复杂度消失无副作用）；ai_commands.rs 从 199 行降至 184 行；226 个测试全部通过 | AI | #REFACTOR-031 |
| 2026-07-20 | Candidate 16 generate_report 便签日期过滤下沉：report_generator.rs 新增 `pub fn filter_notes_by_date(notes: &[Note], start_date: &str, end_date: &str) -> Vec<Note>` 公开纯函数（基于 updated_at 前 10 字符 YYYY-MM-DD 字符串比较，闭区间）；ai_commands.rs `generate_report` 命令删除内联 `notes.into_iter().filter(...)` 7 行闭包，改为调用 `report_generator::filter_notes_by_date(&notes, &start_date, &end_date)`；新增 5 个纯函数单测覆盖空输入/全部在范围内/边界闭区间/跨年/无匹配场景；231 个测试全部通过；通过"删除测试"：报告数据拾取规则（boundaries.md 周报/月报业务规则）从命令层下沉到 report_generator，可在不启动 Tauri 命令的情况下单测 | AI | #REFACTOR-032 |
| 2026-07-20 | Candidate 17 fire_reminders 通知正文构造下沉：domain/reminder.rs 新增 `pub fn notification_title() -> String`（note_title 为空时 fallback "便签提醒"）和 `pub fn notification_body(content: &str) -> String`（前 80 字符 + 省略号 + 空内容 fallback "点击查看便签"）；reminder_scheduler.rs `fire_reminders_with_deps` 删除 13 行内联 title/body 构造逻辑，改为 `let title = reminder.notification_title(); let body = reminder.notification_body(&note.content);`；新增 7 个 domain 单测覆盖空标题/有标题/短内容/空内容/超长截断/恰好 80 字符/UTF-8 字符计数场景；238 个测试全部通过；通过"删除测试"：通知展示规则从调度器下沉到 domain，未来通知模板变化时集中修改 | AI | #REFACTOR-033 |
| 2026-07-21 | 前端模块化拆分（AI 可读性优化）：main.ts 1903→390 行（拆分 15 个模块——note-renderer/note-style/context-menu/note-context/window-state/tag-bar/title-edit/delete-confirm/markdown-renderer/image-resize/reminder-panel/ai-sniff/rewrite-text/todo-sort/template-ui/datetime-picker），hub.ts 1441→131 行（拆分 12 个模块——notes-list/calendar-view/reminder-dialog/ai-settings/template-manager/sync-settings/general-settings/shortcut-settings/update-check）；新增 note-style.ts 破 note-renderer ↔ context-menu 循环依赖；callback 模式（renderNote/setupEventsCallback、showReminderDialog/onNotesChanged、showTemplateDialog/onSelect）破跨模块循环依赖；6 项 AI 可读性原则（文件名=业务名、JSDoc 三段头、单向依赖无环、state 就近+getter/setter、入口仅编排、函数 < 100 行）；前端多页面边界新增 4 项约束（入口编排/单向依赖/callback 模式/side-effect 模块/state 就近）；TypeScript 编译通过（50 modules transformed） | AI | #REFACTOR-034 |
| 2026-07-21 | AI 文件命名规范化：rewrite-text.ts → ai-rewrite.ts，todo-sort.ts → ai-todo-sort.ts（统一 ai- 前缀，与 ai-sniff.ts/ai-settings.ts 一致，AI 一看前缀即知 AI 模块）；4 处 import 更新（main.ts/note-renderer.ts/template-ui.ts/context-menu.ts）；main.ts 与 hub.ts imports 加分组注释（Tauri SDK / 共享 / 便签基础 / 便签 UI / AI 能力 / Hub 页面 / side-effect）；TypeScript 编译通过（50 modules transformed） | AI | #REFACTOR-035 |
| 2026-07-21 | 后端内部事件总线（ADR-007）：新增 `application/event_bus.rs`（DomainEvent/WriteAction/EventPublisher trait + EventBus 同步实现 + MockEventPublisher）；新增 `application/template_service.rs`（save_template/delete_template/create_note_from_template，emit TemplateWritten/NoteWritten 事件，消除 template_commands 直接 repo 访问）；`note_service.rs`/`reminder_service.rs` 全部写方法增加 `publisher: &dyn EventPublisher` 参数 + emit NoteWritten/ReminderWritten 事件；`commands/*` 删除 20 处 schedule_auto_sync 手动调用；`shortcut_manager.rs` 提取 `register_handlers` 共用函数（消除 setup_shortcuts 与 save_and_reregister ~50 行重复）；`tray_manager.rs` handle_new_note 传 event_bus 删除手动 schedule_auto_sync；`lib.rs` AppState 增加 `event_bus: Arc<EventBus>` 字段 + setup 注册监听器（写操作事件 → schedule_auto_sync）；修复 3 处 INV-013 违规（shortcut_manager 漏调 + template_commands save/delete 漏调）；244 个测试全部通过 | AI | #REFACTOR-036 同步更新 constraints.md/glossary.md/adr/README.md |
| 2026-07-21 | 架构深化第二轮（5 候选 + 1 附加）：候选1 IPC seam 违规修复（hub.ts 3 处直接 invoke 替换为 api.setLocale/activateNoteById 封装，api.ts 补 activateNoteById）；候选2 lunar_calendar.rs shallow module 深化（新增 `lunar_date_text` 统一入口，reminder_commands::get_lunar_dates 从 27 行内联 tyme4rs 降为 1 行调用）；候选3 delete_note 图片清理 locality 下沉（cleanup 移到 note_service::delete_note 内部，batch_delete 内部循环调用 delete_note 自动触发，消除单/批量不对称）；候选4 前端模块拆分（新建 `src/notes-multiselect.ts` 承载 selectedIds state + 批量操作栏 5 按钮 + updateMultiSelectUI + Esc 退出，通过 initMultiSelect callback 注入依赖破循环依赖；delete-confirm.ts 扩展双模式支持 Hub 列表；notes-list.ts 从 416→289 行删除内联多选代码和重复 showDeleteConfirm）；候选5 ai_commands 业务规则下沉（report_generator.rs 新增 `parse_period` 消除 28 行内联 period_type 解析；新建 `application/ai_validation.rs` 承载 `validate_rewrite_text`/`validate_sort_todos` 校验，3 个 AI 命令薄壳化）；附加A CREATE_NO_WINDOW 常量集中（git_ops.rs `const` → `pub const`，git_sync.rs/sync_commands.rs 2 处 magic number `0x08000000` 替换为常量引用）；新增 17 个单测覆盖 lunar_date_text/parse_period/ai_validation；261 个测试全部通过 | AI | #REFACTOR-037 同步更新 constraints.md |
| 2026-07-21 | 架构深化第三轮（6 候选合并提交）：候选1 hub.ts IPC seam 修复；候选2 window_manager 拆分为 `hub_window_manager`（Hub 窗口管理）+ `window_overlap_resolver`（compute_overlaps 纯函数 + resolve_overlaps 副作用，5 个单测迁移）+ `window_manager`（Note 窗口生命周期）三模块（ADR-009）；候选3 前端 `utils.ts`+`note-style.ts` 拆为 `colors.ts`（COLOR_MAP/COLORS/applyNoteStyle/formatNoteTime）+`datetime.ts`（localISO/formatDate/quickDate/repeatLabel）+`toast.ts`（showToast）+`html.ts`（escapeHtml）4 个独立 module（14 处调用方同步）；候选4 新建 `src/ai-client.ts` 统一 AI 配置缓存+调用包装（`isAiConfigured`/`getAiConfigCached` 带 5 秒缓存 + ai-config-changed 事件清缓存 / `runAi` 统一 loading/success/error toast 包装，4 个 AI module 改为薄调用）；候选5 `sync_commands` 拆为 `sync_commands`+`shortcut_commands`+`locale_commands`+`system_commands` 4 个子模块（commands/mod.rs glob 重导出 9 个子模块，IPC 命令数不变 45 个）；候选6 Repository trait CQRS 拆分（ADR-010）：`NoteRepository`（5 方法）+ `NoteQuery`（2 方法）+ `ReminderRepository`（6 方法）+ `ReminderQuery`（3 方法）4 个 trait，sqlite/mock 双 impl，AppState 新增 note_query/reminder_query 字段，scheduler 依赖 `&dyn ReminderQuery`；更新对应代码描述（reminder_scheduler 含 ADR-008 emit + ADR-010 ReminderQuery 参数，reminder_service 含 ADR-008 schedule_recalc 监听器触发，window_manager 拆分三模块，event_bus/lib.rs 含 ADR-008 第二条监听器）；前端多页面边界更新（共享模块列表替换 utils.ts→colors/datetime/toast/html/ai-client，单向依赖 note-style→colors，Toast 统一引用 toast.ts）；扩展点分析表新增 2 行（仓储 trait 分离/事件总线实现）；264 个测试全部通过 | AI | #REFACTOR-038 同步更新 ADR-008/009/010/constraints.md/glossary.md/lessons/README.md |
| 2026-07-22 | Reminder 状态机补齐转换合法性校验（INV-031）：domain/reminder.rs 4 个转换方法（mark_triggered/snooze/mark_done/cancel）返回 `Result<(), String>` 表达转换合法性，终态 Done/Cancelled 拒绝所有转换，Triggered 拒绝重复 mark_triggered，Triggered→Pending via snooze 合法；advance_state 内部调用 mark_triggered 用 expect 表达契约；reminder_service snooze/dismiss 加 `?` 传播错误；新增 13 个 domain 测试 + 2 个 service 测试；278 个测试全部通过；更新 reminder.rs 对应代码描述补充 INV-031 检查位置 | AI | #REFACTOR-039 同步更新 constraints.md/flows.md/glossary.md/lessons/README.md |
| 2026-07-22 | data_dir 路径解析收敛单一所有者：system_commands.rs 新增 `pub fn data_dir_path() -> Result<PathBuf, String>` 统一 `exe 同级目录/data` 路径解析（不做 canonicalize，与原始内联代码行为一致，避开 create_dir_all 前路径不存在失败）；`get_data_dir`/`open_data_dir` 命令、`lib.rs` setup、`image_service::image_dir` 4 处内联解析全部改为调用 `data_dir_path()`，image_dir 只追加 sync/images 子目录；通过"删除测试"：路径规则变更集中一处修改，消除 4 处重复解析漂移风险；280 个测试全部通过 | AI | #REFACTOR-041 |
| 2026-07-22 | is_configured 冗余检查消除：删除 `ai_call_raw`/`generate_report` 命令/`report_generator::generate_report` 3 处冗余 `is_configured` 检查，确立 `AiService::call` 为报错语义单一检查点（返回 `AiError::NotConfigured` → `to_string()` 为 "AI 未配置：缺少 API Key"），`sniff_suggestions` 为静默语义唯一例外（`Ok(vec![])`）；错误消息从命令层硬编码 "AI 未配置" 统一为 "AI 未配置：缺少 API Key"（更具体）；更新 ai_rewrite_text/generate_report doc comment 错误消息文案；通过"删除测试"：is_configured 检查逻辑集中一处，修改检查规则只需改 AiService::call；280 个测试全部通过（含 test_generate_report_returns_error_when_not_configured 验证错误传播路径） | AI | #REFACTOR-042 |
| 2026-07-22 | sniff_suggestions 提取 build_suggestions 纯函数：reminder_parser.rs 提取 102 行建议项遍历包装逻辑为 `fn build_suggestions(response: SniffResponse) -> Result<Vec<Suggestion>, AiError>` 纯函数（无 async 无 I/O），sniff_suggestions 从 136 行降至 34 行，解析 JSON 后直接调用 `build_suggestions(response)`；新增 11 个纯函数单测（无 mockito 毫秒级）覆盖 5 种建议类型 + 未知类型跳过 + 4 种空数据跳过 + 空响应 + 3 种混合 + reminder 解析错误；通过"删除测试"：类型分发逻辑可独立单测无需启动 mock 服务器，bug 定位从秒级降至毫秒级；291 个测试全部通过 | AI | #REFACTOR-043 |
| 2026-07-22 | 架构深化第四轮（7 候选合并）：候选1 mock/sqlite 保真度缺口修复（mock delete 幂等化 + find_all/find_by_note_id 加排序 + service 层 delete 加存在性守卫 + 7 个一致性测试，LES-023）；候选2 image_service 反向依赖修复（data_dir_path 从 commands 层提升到 application/paths.rs，image_service 正向依赖 application::paths，system_commands 转发）；候选3 restore_all_windows 委托 note_service::delete_note（消除漏 emit NoteWritten(Deleted) 事件）；候选4 Reminder::effective_time() 单一真相源（is_due/find_next_due_time/find_by_date_range 统一委托，EFFECTIVE_TIME_EXPR SQL 常量）；候选5 手写 JSON 切片改用 serde_json 流式解析（sync_json_io::extract_updated_at + reminder_parser::extract_json + ai_commands::extract_json_array 3 处）；候选6 create_note 抽取 create_note_with_deps 脱离 AppHandle 可单测；候选7a 前端颜色表 purple 缺失 bug 修复 + BATCH_COLORS 统一引用；更新对应代码描述（image_service/paths.rs/note_service/template_service/window_manager/reminder.rs/reminder_service/ipc 通信 data_dir_path）；308 个测试全部通过 | AI | #REFACTOR-044 同步更新 constraints.md/lessons/README.md |
| 2026-07-24 | 架构深化第五轮（3 候选合并）：候选6 JSON 提取逻辑两处独立实现合并——新建 `application/json_extract.rs`（`extract_object`/`extract_array` 两个语义函数 + 共享 `extract_first` 私有函数 + 8 个测试），ai_commands 删除 `extract_json_array` 及 4 测试改调 `json_extract::extract_array`，reminder_parser 删除 `extract_json` 及 3 测试改调 `json_extract::extract_object`；候选7 formatNoteTime 职责错位修正——从 `colors.ts` 迁到 `datetime.ts`（颜色模块不应承载时间格式化，LES-020 拆分粒度原则），note-renderer.ts import 拆分（colors 取 COLORS/applyNoteStyle，datetime 取 formatNoteTime）；候选8 locale_manager 9 个浅包装函数改为常量表——`LocaleText` 结构体 + 9 个 `pub const`（MENU_NEW_NOTE/MENU_SHOW_ALL/MENU_HUB/MENU_SYNC_NOW/MENU_QUIT/MENU_TOOLTIP/MENU_HUB_TITLE/NOTIFY_SYNC_OK/NOTIFY_SYNC_FAIL）+ `.get()` 方法，tray_manager 7 处 + hub_window_manager 1 处 + git_sync 2 处调用方适配为 `XXX.get()`；321 个测试全部通过 | AI | #REFACTOR-045 同步更新 constraints.md/lessons/README.md |
