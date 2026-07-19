# Tasks

- [x] Task 1: 更新 glossary.md — 新增图片宽度语法条目
  - [x] 添加 `img:filename{width=N}` 语法定义
  - [x] 添加图片拖拽调整大小功能说明

- [x] Task 2: 更新 boundaries.md — 补充 delete_note 窗口关闭行为
  - [x] 在 delete_note 命令的职责描述中新增"关闭便签窗口"行为
  - [x] 说明窗口关闭使用 destroy 而非 close

- [x] Task 3: 更新 constraints.md — 新增图片语法约束 + delete_note 行为记录
  - [x] 新增约束：图片宽度语法中 `{` 为文件名终止符
  - [x] 新增不变量：delete_note 命令必须关闭对应窗口（INV-026）

# Task Dependencies
- Task 1, 2, 3 互相独立，可并行执行
