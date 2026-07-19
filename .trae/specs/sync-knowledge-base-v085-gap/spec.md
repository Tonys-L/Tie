# 知识库补充更新 Spec（v0.8.5 遗漏项）

## Why

v0.8.5 版本做了多项修改，知识库已部分更新（INV-026、图片宽度语法、glossary 术语、boundaries 窗口关闭），但经审查发现仍有遗漏：flows.md 中 Git 同步流程图未更新为"先拉后推"、便签窗口生命周期缺少 delete_note 路径、窗口最小尺寸不变量未记录、onCloseRequested 教训未记录。

## What Changes

- **flows.md**: Git 同步流程图更新为"先拉后推"（fetch→merge→import→export→commit→push）
- **flows.md**: 便签窗口生命周期补充 delete_note 关闭窗口路径
- **flows.md**: 跨模块事件联动表补充 delete_note → window destroy 事件
- **constraints.md**: 新增 INV-027（窗口最小尺寸 200×150，三处校验）
- **lessons/README.md**: 新增 LES-016（Tauri 2.x onCloseRequested 改变默认关闭行为）

## Impact

- Affected specs: flows.md（业务流程）、constraints.md（不变量）、lessons/README.md（经验教训）
- Affected code: 无代码变更，仅文档更新

## ADDED Requirements

### Requirement: Git 同步流程图更新

flows.md 中的 Git 同步流程图 SHALL 反映"先拉后推"流程（INV-024），当前流程图仍为旧版"先导出后拉取"。

#### Scenario: 流程图与 INV-024 一致
- **WHEN** 开发者查看 flows.md 的 Git 同步流程图
- **THEN** 流程图为：fetch→merge→import→export→commit→push

### Requirement: 便签窗口生命周期补充 delete_note 路径

flows.md 的便签窗口生命周期 SHALL 包含 delete_note 命令关闭窗口的路径（destroy），与 INV-026 对应。

#### Scenario: delete_note 关闭窗口在生命周期中可见
- **WHEN** 开发者查看便签窗口生命周期图
- **THEN** 可见 delete_note → destroy 窗口的路径

### Requirement: 窗口最小尺寸不变量

constraints.md SHALL 包含窗口最小尺寸不变量（INV-027）：便签窗口最小尺寸 200×150，三处校验（domain 层 clamp、window_manager min_inner_size、前端保存时拦截）。

#### Scenario: 窗口尺寸约束可查
- **WHEN** 开发者查阅 constraints.md
- **THEN** 可见 INV-027 描述窗口最小尺寸约束及三处校验位置

### Requirement: onCloseRequested 教训记录

lessons/README.md SHALL 包含 LES-016：Tauri 2.x 中注册 `win.onCloseRequested` 会改变默认关闭行为，导致 `win.close()` 不再直接关闭窗口。

#### Scenario: Tauri 2.x 关闭行为教训可查
- **WHEN** 开发者查阅经验教训库
- **THEN** 可见 LES-016 关于 onCloseRequested 行为变化的教训
