# Tie - 桌面便签

轻量桌面便签应用，随手记录想法、待办和灵感。

## 功能

### 便签编辑
- Markdown 实时渲染，编辑/查看模式一键切换
- 图片拖入自动保存并插入引用
- 待办清单：输入 `- [ ] 任务` 查看模式显示可点击 checkbox，点击自动切换状态并保存
- 6 种配色 + 透明度调节
- 置顶 / 归档 / 删除

### 定时提醒
- 重复类型：一次性 / 每天 / 每周 / 每月（精确日历月）/ 农历每月
- 提醒触发：窗口闪烁 + 横幅通知（贪睡 5 分钟 / 标记完成）
- 农历每月基于 tyme4rs 农历库，公历↔农历精确转换

### 日历视图
- 月视图：显示提醒标题 + 农历日期 + 提醒状态色 + 便签活动标记
- 年视图：3×4 缩略网格，点击切换月份
- 点击日期 → 查看当天提醒 / 快速创建提醒
- 今天高亮 + 本周背景色

### 标签与搜索
- 标签管理，侧边栏快速筛选
- 全局搜索：模糊匹配标题 + 内容 + 标签，跨活跃/归档

### 多设备同步
- 基于 Git 的同步机制，数据完全本地存储
- 支持 GitHub / Gitee 私有仓库
- 同步状态显示分支名

### 其他
- 中英双语 / 深色模式
- 全局快捷键：Ctrl+Shift+N 新建、Ctrl+Shift+S 显示全部
- 便签内快捷键：Ctrl+S 保存、Esc 退出编辑、Ctrl+N 新建
- 右键菜单：置顶 / 归档 / 删除 / 换色
- 开机自启动
- 安装包仅 ~3MB

## 使用方式

| 操作 | 方式 |
|------|------|
| 新建便签 | 双击托盘图标 / Ctrl+Shift+N / Ctrl+N |
| 编辑便签 | 单击便签内容区进入编辑，失焦自动保存 |
| 待办清单 | 输入 `- [ ] 任务` 或 `- [x] 已完成`，查看模式点击 checkbox 切换 |
| 插入图片 | 拖入图片文件，自动保存到本地并插入引用 |
| 设置提醒 | 点击便签顶部闹钟图标，选择时间和重复类型 |
| 右键菜单 | 右键便签内容区：置顶 / 归档 / 删除 / 换色 |
| 搜索便签 | Hub 设置中心顶部搜索框，实时模糊匹配 |
| 查看日历 | Hub → 日历，月历/年历查看提醒分布 |
| 多设备同步 | Hub → 同步，配置 Git 私有仓库地址和令牌 |

## 安装

从 [GitHub Releases](https://github.com/Tonys-L/Tie/releases) 下载安装包，运行即可。

## 开发

### 前置条件

- Node.js 18+
- Rust (rustup)
- Tauri CLI 2.0

### 启动开发模式

```bash
# Windows
.\dev.ps1

# Linux / macOS
./dev.sh
```

### 构建安装包

```bash
# Windows
.\build.ps1

# Linux / macOS
./build.sh
```

### 手动方式

```bash
npm install
npm run tauri dev      # 开发
npm run tauri build    # 构建
```

## 项目结构

```
tie/
├── src/               # 前端源码 (TypeScript)
├── src-tauri/         # 后端源码 (Rust / Tauri)
│   ├── src/
│   │   ├── domain/       # 领域模型（Note、Reminder、Tag）
│   │   ├── application/  # 应用服务（编排、农历、调度）
│   │   └── infrastructure/ # 基础设施（SQLite、Git、窗口）
│   └── Cargo.toml
├── index.html         # 便签窗口入口
├── hub.html           # 设置中心入口
├── package.json
└── vite.config.ts
```

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | Tauri 2.0 |
| 后端 | Rust |
| 前端 | TypeScript + Vite |
| 本地存储 | SQLite |
| 同步 | Git |
| 农历 | tyme4rs |
