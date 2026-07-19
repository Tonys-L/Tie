# 业务流程与状态机

> **TL;DR**: 关键流程：Git 同步、应用启动、便签窗口生命周期。关键状态机：Reminder。⚠️ 提醒一旦 Done/Cancelled 不可恢复；一次性提醒触发后不可重新触发。

---

## 业务流程

### Git 同步流程

**触发条件**: 用户点击"立即同步"或自动同步触发（30 秒防抖）

**参与者**: GitSync 模块、Git 平台、本地 SQLite

```mermaid
flowchart TD
    A[开始同步] --> B[加载配置 repo_url/token]
    B --> C[init_repo git init]
    C --> D[设置 remote origin auth_url]
    D --> E[git fetch origin]
    E --> F{HEAD != origin?}
    F -->|否| G[export_to_json 导出便签+提醒+模板]
    F -->|是| H[git merge --no-edit --allow-unrelated-histories]
    H --> I{有冲突?}
    I -->|是| J[resolve_conflicts 按updated_at取最新]
    J --> K[git add + commit]
    I -->|否| L[import_from_json 导入回DB]
    K --> L
    L --> G
    G --> M[git add -A + status]
    M --> N{有本地变更?}
    N -->|是| O[git commit 时间戳消息]
    N -->|否| P{需要push?}
    O --> P
    P -->|是| Q[安全检查 删除文件占比]
    Q --> R{删除>50%?}
    R -->|是| S[拒绝推送 返回错误]
    R -->|否| T[git push --force-with-lease]
    T --> U[完成]
    P -->|否| U
```

**异常处理**:

| 异常场景 | 处理方式 |
|----------|----------|
| Git 未安装 | `check_git` 返回 false，设置页显示警告 |
| 网络/认证失败 | 返回错误字符串，前端显示同步失败 |
| merge 冲突 | 解析冲突标记，按 updated_at last-write-wins |
| merge 失败后仍有未解决冲突 | 拒绝 push，返回错误 |
| push 前删除文件占比>50% | 拒绝推送，防止远程数据被覆盖 |
| push 被拒绝 | --force-with-lease 强制推送 |

### 已知策略缺口

无（提醒导入已遵循 last-write-wins，与便签导入逻辑一致）。

---

### 应用启动流程

**触发条件**: 应用启动

**参与者**: lib.rs setup、数据库、调度器、窗口管理器

```mermaid
flowchart TD
    A[main → lib::run] --> B[注册插件]
    B --> C[setup]
    C --> D[初始化数据库 WAL+迁移]
    D --> E[构造仓储注入 AppState]
    E --> F[设置系统托盘]
    F --> G[注册全局快捷键]
    G --> H[启动提醒调度器 5秒后开始]
    H --> I[自动拉取同步 auto_pull]
    I --> J[恢复所有未归档便签窗口]
    J --> K[初始化完成]
```

---

### 便签窗口生命周期

**触发条件**: 用户创建/打开便签、启动恢复、提醒触发

**参与者**: window_manager、前端 main.ts

```mermaid
flowchart TD
    A[open_note_window] --> B{窗口已存在?}
    B -->|是| C[聚焦+闪烁提示]
    B -->|否| D[WebviewWindowBuilder 创建]
    D --> E[前端 initNoteWindow]
    E --> F[invoke get_note 加载便签]
    F --> G[renderNote 渲染]
    G --> H[setupWindowEvents 绑定事件]
    H --> I[win.show 显示窗口]
    I --> J[用户编辑 自动保存+防抖]
    J --> K[CloseRequested]
    K --> L{title+content 均空?}
    L -->|是| M[从DB删除]
    L -->|否| N[保存窗口状态 隐藏窗口]
    O[delete_note 命令] --> P[删除DB数据+级联删除Reminder]
    P --> Q[destroy 强制销毁窗口]
```

---

## 状态机

### Reminder 状态机

**初始状态**: Pending

```mermaid
stateDiagram-v2
    [*] --> Pending : create_reminder
    Pending --> Pending : snooze(n) 设snoozed_until
    Pending --> Triggered : mark_triggered (仅一次性)
    Pending --> Pending : 调度器触发+is_repeating (重置remind_at)
    Triggered --> Done : mark_done
    Pending --> Done : mark_done
    Pending --> Cancelled : cancel
    Triggered --> Cancelled : cancel
    Done --> [*]
    Cancelled --> [*]
```

### 状态说明

| 状态 | 含义 | 允许的操作 |
|------|------|------------|
| Pending | 等待触发 | snooze, mark_done, cancel, 调度器触发 |
| Triggered | 已触发（一次性提醒） | mark_done, cancel |
| Done | 已完成 | 无（终态） |
| Cancelled | 已取消 | 无（终态） |

### 转换规则

| 从 | 到 | 触发条件 | 副作用 |
|----|-----|----------|--------|
| (创建) | Pending | create_reminder | 写入 DB |
| Pending | Pending | snooze(n) | 设置 snoozed_until |
| Pending | Pending | 调度器触发 + is_repeating | 计算 next_trigger，更新 remind_at |
| Pending | Triggered | 调度器触发 + !is_repeating | 发送通知，创建便签窗口 |
| Pending | Done | mark_done (dismiss_reminder) | 更新 status |
| Pending | Cancelled | cancel (delete_reminder) | 从 DB 删除 |
| Triggered | Done | mark_done | 更新 status |
| Triggered | Cancelled | cancel | 从 DB 删除 |

### 禁止的转换

| 从 | 到 | 原因 |
|----|-----|------|
| Done | Pending | 已完成不可恢复 |
| Cancelled | Pending | 已取消不可恢复 |
| Triggered | Pending | 一次性提醒触发后不可重置（周期提醒不进入 Triggered） |
| Done | Triggered | 终态不可转换 |
| Cancelled | Triggered | 终态不可转换 |

---

## 跨模块事件联动

| 触发方 | 事件 | 受影响方 | 联动动作 | 失败处理 |
|--------|------|----------|----------|----------|
| Reminder Scheduler | 提醒到期 | Note Window | 后端直接创建便签窗口（URL 带 reminder 参数） | 窗口创建失败则仅发通知 |
| Reminder Scheduler | 提醒到期 | Notification | 发送系统通知（标题=note_title） | 通知失败不影响窗口创建 |
| delete_note 命令 | Note 删除 | Reminder | 级联删除关联 Reminder | DB ON DELETE CASCADE 兜底 |
| delete_note 命令 | Note 删除 | Note Window | destroy 强制销毁窗口（INV-026） | 窗口获取失败仅记录日志 |
| Git Sync | 同步完成 | Note + Reminder | import_from_json 更新本地数据 | 导入失败回滚 |
| Note 窗口关闭 | CloseRequested | Note 数据 | 空便签删除，非空保存窗口状态 | 保存失败记录日志 |

---

## 变更记录

| 日期 | 变更内容 | 变更人 | 关联变更 |
|------|----------|--------|----------|
| 2026-07-09 | 初始版本，按模板结构填充 | — | — |
| 2026-07-09 | 更新提醒导入策略缺口为已修复 | — | #REFACTOR-001 |
| 2026-07-09 | 调度器周期重置改用 domain 方法 reset_for_next_trigger | — | #REFACTOR-002 |
| 2026-07-19 | Git 同步流程图更新为“先拉后推”；便签窗口生命周期补充 delete_note 路径；跨模块事件联动表补充 delete_note→window destroy | AI | v0.8.5 同步更新 constraints.md |
