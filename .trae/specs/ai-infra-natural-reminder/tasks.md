# Tasks

> 开发模式：TDD（RED → GREEN → REFACTOR）
> 原则：每个任务先写测试（RED），再实现到测试通过（GREEN），不超前实现

- [x] Task 1: AI 配置模型与持久化（ai_config.rs）
  - [x] 1.1 编写测试：读取未配置的配置文件返回默认空值
  - [x] 1.2 编写测试：保存配置后重新加载应得到相同数据
  - [x] 1.3 实现 `AiConfig` 结构（base_url / api_key / model）+ load / save 函数
  - [x] 1.4 配置文件路径加入 `.gitignore`，确保不随 Git 同步

- [x] Task 2: AI 调用服务（ai_service.rs）
  - [x] 2.1 编写测试：未配置 API Key 时调用返回 `NotConfigured` 错误
  - [x] 2.2 编写测试：超时（>5秒）返回 `Timeout` 错误（用 mockito 模拟慢响应）
  - [x] 2.3 编写测试：HTTP 错误状态码返回 `Network` 错误
  - [x] 2.4 实现 `AiService::call(messages) -> Result<String, AiError>`，使用 reqwest + tokio，5秒超时
  - [x] 2.5 实现 `AiService::test_connection()`，发送轻量请求验证配置

- [x] Task 3: Prompt 框架（prompts/）
  - [x] 3.1 编写测试：reminder 模板能正确填充用户输入并输出 system + user 两条消息
  - [x] 3.2 实现 `prompts/mod.rs`（统一 trait 或常量导出）
  - [x] 3.3 实现 `prompts/reminder.rs`（系统提示要求返回 JSON：title / start_time / repeat_type / repeat_day）

- [x] Task 4: 自然语言提醒解析命令
  - [x] 4.1 编写测试：合法 JSON 响应正确解析为 `ReminderDraft`
  - [x] 4.2 编写测试：非法 JSON 返回 `ParseError`
  - [x] 4.3 编写测试：字段缺失（如无 start_time）返回 `ParseError`
  - [x] 4.4 实现 `parse_reminder_natural(text) -> Result<ReminderDraft, AiError>`，组合 AiService + reminder prompt + JSON 解析

- [x] Task 5: AI 配置命令注册
  - [x] 5.1 实现 `get_ai_config` 命令（返回当前配置，API Key 可脱敏）
  - [x] 5.2 实现 `save_ai_config` 命令（写入本地文件）
  - [x] 5.3 实现 `test_ai_connection` 命令（调用 `AiService::test_connection`）
  - [x] 5.4 在 `lib.rs` 的 `invoke_handler` 注册三个命令 + `parse_reminder_natural` 命令
  - [x] 5.5 在 `capabilities/default.json` 添加命令权限

- [x] Task 6: Hub AI 配置页面
  - [x] 6.1 `hub.html` 新增 AI 配置页面 HTML（导航项 + 表单 + 测试连接按钮）
  - [x] 6.2 `src/api.ts` 新增 `getAiConfig / saveAiConfig / testAiConnection / parseReminderNatural` 封装
  - [x] 6.3 `src/hub.ts` 实现页面加载配置 / 保存配置 / 测试连接交互
  - [x] 6.4 `src/i18n/zh.ts` + `en.ts` 添加 AI 配置相关文案

- [x] Task 7: 便签提醒弹窗 AI 解析入口
  - [x] 7.1 `src/hub.ts` 提醒弹窗新增自然语言输入框 + ⚡ AI 解析按钮（琥珀色 #F59E0B）
  - [x] 7.2 调用 `parseReminderNatural` 命令，成功则自动填充提醒表单字段
  - [x] 7.3 解析失败 toast 提示"未能识别，请手动填写"，不阻断表单使用
  - [x] 7.4 未配置 API Key 时按钮置灰 + tooltip 提示"请先在 Hub 配置 AI"

# Task Dependencies
- Task 2 depends on Task 1
- Task 3 无依赖（可与 Task 1/2 并行）
- Task 4 depends on Task 2, Task 3
- Task 5 depends on Task 1, Task 2, Task 4
- Task 6 depends on Task 5
- Task 7 depends on Task 5, Task 6
