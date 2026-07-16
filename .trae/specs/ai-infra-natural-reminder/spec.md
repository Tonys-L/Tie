# AI 基础设施 + 自然语言提醒 Spec

## Why

便签应用需要接入 AI 能力，第一个用户可感知的功能是自然语言创建提醒——用户输入"明天上午9点提醒我开会"，AI 自动解析为提醒字段，免去手动翻日历选时间。

## What Changes

- 新增 AI 配置模型（Base URL / API Key / Model），本地存储，不随 Git 同步
- 新增 AI 调用服务（HTTP client、5秒超时、错误处理、离线降级）
- 新增 Prompt 框架（system/user 格式，统一管理）
- 新增提醒解析 Prompt + 命令（自然语言 → JSON 提醒字段）
- 新增 AI 配置命令（get / save / test_connection）
- Hub 新增「AI 配置」页面
- 便签提醒弹窗新增自然语言输入框 + ⚡ AI 解析按钮
- 未配置 API Key 时 AI 入口置灰
- 新增 reqwest + tokio 依赖

## Impact

- Affected code: `src-tauri/src/application/`（新增 ai_service.rs、ai_config.rs、prompts/）、`src-tauri/src/application/commands.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`、`hub.html`、`src/hub.ts`、`src/main.ts`、`src/api.ts`、`src/i18n/`
- domain 层不引入 AI 依赖，AI 能力完全在 application 层

## ADDED Requirements

### Requirement: AI 配置管理
系统 SHALL 提供 AI 配置的本地存储和读取能力，包括 Base URL、API Key、模型名。

#### Scenario: 读取配置
- **WHEN** 应用启动或进入 AI 配置页面
- **THEN** 返回当前保存的配置（未配置时返回空值）

#### Scenario: 保存配置
- **WHEN** 用户在 AI 配置页面填写并保存
- **THEN** 配置写入本地文件，不随 Git 同步

#### Scenario: 测试连接
- **WHEN** 用户点击"测试连接"按钮
- **THEN** 向 API 发送一个轻量请求，返回成功/失败结果

### Requirement: AI 调用服务
系统 SHALL 封装 HTTP 调用远程 AI API 的能力，支持超时和错误处理。

#### Scenario: 正常调用
- **WHEN** 发送合法请求到已配置的 API
- **THEN** 返回 AI 响应内容

#### Scenario: 超时
- **WHEN** API 5秒内未响应
- **THEN** 返回超时错误，不阻塞用户操作

#### Scenario: 未配置
- **WHEN** 未配置 API Key 时调用 AI 功能
- **THEN** 返回"未配置"错误，前端置灰入口

### Requirement: 自然语言创建提醒
系统 SHALL 解析自然语言文本为结构化提醒字段（title、start_time、repeat_type、repeat_day）。

#### Scenario: 一次性提醒
- **WHEN** 用户输入"明天上午9点提醒我开会"
- **THEN** 返回 title="开会"、start_time=明天09:00、repeat_type="once"

#### Scenario: 每周重复
- **WHEN** 用户输入"每周五下午5点提醒我发周报"
- **THEN** 返回 title="发周报"、start_time=本周五17:00、repeat_type="weekly"

#### Scenario: 解析失败
- **WHEN** AI 返回无法解析的结果或调用失败
- **THEN** 返回错误，前端提示"未能识别，请手动填写"并回退到表单模式

### Requirement: AI 配置页面
系统 SHALL 在 Hub 设置中心提供 AI 配置页面。

#### Scenario: 页面展示
- **WHEN** 用户进入 Hub → AI 配置
- **THEN** 显示 Base URL、API Key（密码框）、模型名输入框 + 测试连接按钮

#### Scenario: 离线降级
- **WHEN** 未配置 API Key
- **THEN** 便签中所有 AI 入口置灰，tooltip 提示"请先配置 AI"
