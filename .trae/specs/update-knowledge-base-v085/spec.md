# 知识库文档更新 (v0.8.5) Spec

## Why
v0.8.5 版本引入了多项行为变更（delete_note 关闭窗口、图片宽度语法、extract_image_filenames 兼容性），知识库需要同步更新以保持代码与文档一致。

## What Changes
- **glossary.md**：新增 `img:filename{width=N}` 语法条目
- **boundaries.md**：更新 delete_note 命令的职责描述（新增窗口关闭行为）
- **constraints.md**：新增图片宽度语法的约束说明 + delete_note 窗口关闭行为记录
- **lessons/README.md**：无需更新（本次改动无踩坑经验）

## Impact
- Affected specs: glossary.md, boundaries.md, constraints.md
- Affected code: commands.rs, main.ts, note_service.rs

## ADDED Requirements

### Requirement: 图片宽度语法
系统 SHALL 支持 `img:filename{width=N}` Markdown 语法，用于指定便签内图片的显示宽度。

#### Scenario: 带宽度的图片渲染
- **WHEN** 便签内容包含 `![](img:photo.png{width=300})`
- **THEN** 图片以 300px 宽度渲染，保持原始宽高比

#### Scenario: 拖拽调整图片宽度
- **WHEN** 用户在查看模式下拖拽图片右下角手柄
- **THEN** 图片宽度实时调整，松开后 `img:filename{width=N}` 写回内容并持久化

### Requirement: delete_note 关闭窗口
delete_note 命令 SHALL 在删除数据后关闭对应便签窗口（使用 destroy 强制销毁）。

#### Scenario: 删除便签后窗口消失
- **WHEN** 用户确认删除便签
- **THEN** 便签数据被删除，关联提醒被级联删除，便签窗口被 destroy

## MODIFIED Requirements

### Requirement: 图片文件名提取（constraints.md 补充说明）
extract_image_filenames 函数 SHALL 将 `{` 作为文件名终止符，以兼容 `img:filename{width=N}` 语法。

## REMOVED Requirements
无
