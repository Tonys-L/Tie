# Tie

> 贴出来，想得到

轻量桌面便签应用，随手记录想法、待办和灵感。

官网：[tie.8421.fun](https://tie.8421.fun)

[English](README.en.md)

## 功能

### 编辑
- Markdown 实时渲染，编辑/查看模式一键切换
- 图片拖入自动保存并插入引用
- 待办清单、标签管理、置顶、自定义颜色、透明度、归档
- 便签模板，一键创建结构化便签
- FTS5 全文搜索：标题/内容/标签秒级检索，关键词高亮

### 提醒
- 重复类型：一次性 / 每天 / 每周 / 每月 / 农历每月
- 日历视图：月历 + 年历，提醒分布一目了然
- 提醒触发：窗口闪烁 + 横幅通知（贪睡 5 分钟 / 标记完成）

### AI
- AI 分析：保存便签自动识别提醒/拆任务/规整/标签
- AI 文本重写：右键菜单 5 种操作（规整/转清单/正式/精简/温和）
- AI 待办排序 / 周报月报
- 支持本地或远端 LLM，可关闭可替换

### 同步
- 基于 Git 的多设备同步，数据完全本地存储
- 支持 GitHub / Gitee 私有仓库

### 其他
- 中英双语 / 深色模式
- 全局快捷键 / 多选批量操作
- 自动更新检查 / 开机自启动
- 安装包仅 ~3MB

## 安装

从 [GitHub Releases](https://github.com/Tonys-L/Tie/releases) 下载安装包，运行即可。

## 开发

### 前置条件

- Node.js 18+
- Rust (rustup)
- Tauri CLI 2.0

### 启动开发模式

```bash
npm install
npm run tauri dev
```

### 构建安装包

```bash
npm run tauri build
```

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri 2.0 |
| 后端 | Rust |
| 前端 | TypeScript + Vite |
| 本地存储 | SQLite + FTS5 |
| 同步 | Git |
| 农历 | tyme4rs |

## License

[MIT](LICENSE)
