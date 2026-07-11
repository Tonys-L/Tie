# Tie

桌面便签应用，基于 Tauri 2.0 + Rust + TypeScript 构建。

## 功能

- Markdown 编辑便签
- 定时提醒
- 多设备同步（Git）
- 归档管理
- 全局快捷键
- 中英双语 / 深色模式

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

### 构建并启动

```bash
# Windows
.\build.ps1

# Linux / macOS
./build.sh
```

仅启动（跳过构建）：

```bash
# Windows
.\build.ps1 -SkipBuild

# Linux / macOS
./build.sh --skip
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
│   │   ├── domain/       # 领域模型
│   │   ├── application/  # 应用服务
│   │   └── infrastructure/ # 基础设施
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
