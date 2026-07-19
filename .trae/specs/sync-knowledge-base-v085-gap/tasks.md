# Tasks

- [x] Task 1: 更新 flows.md — Git 同步流程图改为"先拉后推"
  - [x] 将 Git 同步流程图从旧版（export→commit→fetch→merge→import→push）更新为新版（fetch→merge→import→export→commit→push）
  - [x] 更新异常处理表：merge 失败后检查冲突、push 前安全检查（删除>50%拒绝）
  - [x] 填写变更记录

- [x] Task 2: 更新 flows.md — 补充便签窗口生命周期和跨模块事件
  - [x] 便签窗口生命周期图补充 delete_note → destroy 窗口路径
  - [x] 跨模块事件联动表补充 delete_note → window destroy 事件
  - [x] 填写变更记录

- [x] Task 3: 更新 constraints.md — 新增 INV-027 窗口最小尺寸不变量
  - [x] 新增 INV-027：便签窗口最小尺寸 200×150，三处校验（domain note.rs update_window_state clamp、window_manager.rs min_inner_size、main.ts 保存时拦截）
  - [x] 填写变更记录

- [x] Task 4: 更新 lessons/README.md — 新增 LES-016 onCloseRequested 教训
  - [x] 新增 LES-016：Tauri 2.x 中注册 onCloseRequested 改变默认关闭行为，导致 win.close() 不再直接关闭窗口
  - [x] 更新文件索引和检索指引
  - [x] 填写变更记录

# Task Dependencies
- Task 1, 2 可合并执行（同文件 flows.md）
- Task 3, 4 互相独立，可并行执行
