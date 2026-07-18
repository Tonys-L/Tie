# Tie

> 贴出来，想得到 / Tie it, find it

轻量桌面便签应用，随手记录想法、待办和灵感。

Lightweight desktop notes app for capturing ideas, to-dos, and inspiration.

**官网 / Website**: [tie.8421.fun](https://tie.8421.fun)

---

## 功能 / Features

### 编辑 / Edit
- Markdown 实时渲染，编辑/查看模式一键切换
- Live-rendered Markdown with one-click edit/view toggle
- 图片拖入自动保存并插入引用
- Drag-and-drop images auto-saved and inserted
- 待办清单、标签管理、置顶、自定义颜色、透明度、归档
- Checklists, tags, pin, custom colors, opacity, archive
- 便签模板，一键创建结构化便签
- Note templates for one-click structured note creation
- FTS5 全文搜索：标题/内容/标签秒级检索，关键词高亮
- FTS5 full-text search with keyword highlighting

### 提醒 / Remind
- 一次性 / 每天 / 每周 / 每月 / 农历每月
- One-time, daily, weekly, monthly, lunar monthly
- 日历视图：月历 + 年历，提醒分布一目了然
- Calendar view: monthly + yearly, reminders at a glance
- 窗口闪烁 + 横幅通知（贪睡 5 分钟 / 标记完成）
- Flash window + banner notification (snooze 5 min / mark done)

### AI
- AI 分析：保存便签自动识别提醒/拆任务/规整/标签
- AI Analysis: auto-detect reminders, split tasks, tidy text, suggest tags
- AI 文本重写：右键菜单 5 种操作
- AI Rewrite: 5 right-click operations (tidy/checklist/formal/concise/mild)
- AI 待办排序 / 周报月报
- AI Todo Sort / Weekly & Monthly Reports
- 支持本地或远端 LLM，可关闭可替换
- Local or remote LLM, toggleable & swappable

### 同步 / Sync
- 基于 Git 的多设备同步，数据完全本地存储
- Git-based multi-device sync, all data stored locally
- 支持 GitHub / Gitee 私有仓库
- Private GitHub / Gitee repos supported

### 其他 / More
- 中英双语 / 深色模式
- Bilingual (ZH/EN) / Dark mode
- 全局快捷键 / Global shortcuts
- 多选批量操作 / Batch operations (Ctrl+click)
- 自动更新检查 / Auto update check
- 开机自启动 / Launch on startup
- 安装包仅 ~3MB / Installer ~3MB

## 安装 / Install

从 [GitHub Releases](https://github.com/Tonys-L/Tie/releases) 下载安装包，运行即可。

Download from [GitHub Releases](https://github.com/Tonys-L/Tie/releases) and run.

## 开发 / Development

### 前置条件 / Prerequisites

- Node.js 18+
- Rust (rustup)
- Tauri CLI 2.0

### 启动开发模式 / Dev

```bash
npm install
npm run tauri dev
```

### 构建安装包 / Build

```bash
npm run tauri build
```

## 技术栈 / Tech Stack

| 层 / Layer | 技术 / Tech |
|----|------|
| 桌面框架 / Desktop | Tauri 2.0 |
| 后端 / Backend | Rust |
| 前端 / Frontend | TypeScript + Vite |
| 本地存储 / Storage | SQLite + FTS5 |
| 同步 / Sync | Git |
| 农历 / Lunar | tyme4rs |

## License

[MIT](LICENSE)
