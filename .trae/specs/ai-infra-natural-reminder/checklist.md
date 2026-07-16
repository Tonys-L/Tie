# Checklist

## 配置管理（ai_config.rs）
- [x] `AiConfig` 结构包含 `base_url` / `api_key` / `model` 字段
- [x] 配置文件存储在本地用户目录，不随 Git 同步（`.gitignore` 已添加）
- [x] `get_ai_config` 命令返回当前配置（未配置时返回空值）
- [x] `save_ai_config` 命令成功写入文件
- [x] `test_ai_connection` 命令返回成功/失败结果

## AI 调用服务（ai_service.rs）
- [x] 未配置 API Key 时返回 `NotConfigured` 错误
- [x] API 调用超时时间为 5 秒
- [x] HTTP 请求使用 OpenAI 兼容格式（`POST /chat/completions`）
- [x] 错误类型清晰（`NotConfigured` / `Timeout` / `Network` / `ParseError`）
- [x] 单次请求 `max_tokens ≤ 500`，`temperature ≤ 0.3`

## Prompt 框架（prompts/）
- [x] `prompts/mod.rs` 提供统一的消息结构或 trait
- [x] `prompts/reminder.rs` 提供系统提示（要求返回 JSON）和用户输入拼接
- [x] 提醒解析 Prompt 明确要求字段：title / start_time / repeat_type / repeat_day

## 自然语言提醒解析
- [x] 命令接收文本输入，返回 `ReminderDraft`
- [x] `ReminderDraft` 包含 title / start_time / repeat_type / repeat_day 字段
- [x] 解析失败时返回 `ParseError`，前端能识别并提示
- [x] `parse_reminder_natural` 命令在 `lib.rs` 注册

## Hub AI 配置页面
- [x] Hub 导航新增「AI 配置」入口
- [x] 页面包含 Base URL / API Key（密码框）/ 模型名 输入框
- [x] 测试连接按钮显示成功/失败 toast
- [x] 保存按钮持久化配置
- [x] 中英文翻译齐全（zh.ts + en.ts）

## 便签提醒弹窗 AI 解析入口
- [x] 提醒弹窗新增自然语言输入框 + ⚡ AI 解析按钮
- [x] 解析成功自动填充提醒表单字段（标题/时间/重复类型）
- [x] 解析失败 toast 提示"未能识别，请手动填写"
- [x] 未配置 API Key 时按钮置灰，tooltip 提示"请先在 Hub 配置 AI"
- [x] AI 入口颜色为琥珀色 #F59E0B

## 架构约束
- [x] AI 能力仅在 application 层，domain 层无 AI 依赖
- [x] API Key 不随 Git 同步
- [x] application 层模块（ai_service / ai_config）单元测试覆盖率 ≥ 60%
- [x] `Cargo.toml` 新增 `reqwest`（含 `json` feature）+ `tokio` 依赖
- [x] `capabilities/default.json` 添加新命令权限

## 离线降级
- [x] 未配置 API Key 时，便签中所有 AI 入口置灰
- [x] 应用原有纯本地功能完全不受 AI 模块影响
- [x] AI 调用失败不影响用户继续手动填写表单
