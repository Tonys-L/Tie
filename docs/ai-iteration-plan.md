# AI 轻量增强 · 迭代计划

> 基于 ai-plan.md V1.1  
> 核心节奏：**先通管道，再挂功能**


## 迭代一：AI 基础设施 + 自然语言提醒（P0）

**版本**：v0.5.0  
**目标**：打通 AI 调用链路，实现第一个用户可感知的 AI 功能

### 后端

| 任务 | 文件 | 说明 |
| :--- | :--- | :--- |
| AI 配置模型 | `application/ai_config.rs` | AiConfig { base_url, api_key, model }，读写本地配置文件（不随 Git 同步） |
| AI 调用服务 | `application/ai_service.rs` | 封装 HTTP 调用（reqwest）、5秒超时、错误处理、离线降级判断 |
| Prompt 框架 | `application/prompts/mod.rs` | Prompt 模板注册 + 渲染，统一 system/user 格式 |
| 提醒解析 Prompt | `application/prompts/reminder.rs` | 自然语言→JSON 提醒字段的 system prompt |
| 提醒解析命令 | `application/commands.rs` | 新增 `parse_reminder_natural_language(text) -> ReminderParseResult` |
| AI 配置命令 | `application/commands.rs` | 新增 `get_ai_config` / `save_ai_config` / `test_ai_connection` |
| reqwest 依赖 | `Cargo.toml` | 添加 reqwest + tokio（HTTP 客户端） |

### 前端

| 任务 | 文件 | 说明 |
| :--- | :--- | :--- |
| AI 配置页面 | `hub.html` + `hub.ts` | Hub 侧边栏新增「AI 配置」页面：Base URL / API Key / 模型名 / 测试连接 |
| 自然语言输入框 | `main.ts` | 提醒弹窗新增文本输入框 + ⚡ AI 解析按钮 |
| 解析结果回填 | `main.ts` | AI 返回 JSON → 自动填充提醒表单字段，用户确认后保存 |
| 离线降级 | `main.ts` + `hub.ts` | 未配置 API Key 时，AI 入口置灰 + tooltip 提示"请先配置 AI" |

### 验证

- [ ] AI 配置页面：填入 Key → 测试连接成功
- [ ] 自然语言提醒："明天上午9点提醒我开会" → 解析为 title="开会" start_time=明天09:00 repeat=once
- [ ] 自然语言提醒："每周五下午5点提醒我发周报" → repeat=weekly
- [ ] 未配置 Key 时 AI 按钮置灰
- [ ] API 超时/报错 → toast 提示，不影响原有功能


## 迭代二：提醒嗅探 + 智能文本规整 + 文风切换（P0+P1）

**版本**：v0.6.0  
**目标**：右键菜单 AI 增强，让便签编辑"说人话变工整"

### 后端

| 任务 | 文件 | 说明 |
| :--- | :--- | :--- |
| 规整 Prompt | `application/prompts/tidy.rs` | 口语转书面 / 转清单 / 精简 三个 system prompt |
| 文风 Prompt | `application/prompts/style.rs` | 更正式 / 更精简 / 更温和 三个 system prompt |
| AI 文本处理命令 | `application/commands.rs` | `ai_tidy_text(text, mode)` / `ai_style_text(text, style)` |
| 提醒嗅探 | `application/commands.rs` | `detect_reminder_in_text(content) -> Vec<DetectedReminder>` |

### 前端

| 任务 | 文件 | 说明 |
| :--- | :--- | :--- |
| 右键菜单 AI 子菜单 | `main.ts` | 右键菜单新增「✨ AI」子菜单：规整文本 / 转为清单 / 精简表达 / 更正式 / 更精简 / 更温和 |
| 选中文字 AI 操作 | `main.ts` | 选中文字 → 右键 → AI 操作 → 替换选中文本（支持 Ctrl+Z 撤销） |
| 提醒嗅探气泡 | `main.ts` | 便签含时间关键词时，提醒设置区显示嗅探气泡，点击一键填充 |

### 验证

- [ ] 选中口语文字 → 右键 → "规整文本" → 文字变书面语
- [ ] 选中列举文字 → 右键 → "转为清单" → 生成 checkbox 列表
- [ ] 选中冗余文字 → 右键 → "精简表达" → 去掉废话
- [ ] 选中文字 → 右键 → "更正式" → 语气变正式
- [ ] AI 操作后 Ctrl+Z 可撤销
- [ ] 便签含"明天下午3点" → 提醒区出现嗅探气泡


## 迭代三：周报/月报 + 待办排序 + 标签速配（P1+P2）

**版本**：v0.7.0  
**目标**：回顾增强，让便签从"只记不看"到"定期回顾"

### 后端

| 任务 | 文件 | 说明 |
| :--- | :--- | :--- |
| 周报 Prompt | `application/prompts/report.rs` | 周报/月报生成的 system prompt |
| 排序 Prompt | `application/prompts/sort.rs` | 待办排序的 system prompt |
| 周报数据拾取 | `application/commands.rs` | `generate_report(type: week/month, end_date?) -> String` |
| 待办排序命令 | `application/commands.rs` | `ai_sort_tasks(items: Vec<String>) -> Vec<String>` |
| 标签推荐命令 | `application/commands.rs` | `ai_suggest_tags(content: String) -> Vec<String>` |

### 前端

| 任务 | 文件 | 说明 |
| :--- | :--- | :--- |
| 日历视图周报按钮 | `hub.ts` | 月视图顶部「📊 生成月报」按钮（≥3条便签时亮起） |
| 侧边栏周报入口 | `hub.html` | 侧边栏底部「📋 生成周报」快捷入口 |
| 周报结果展示 | `hub.ts` | AI 返回 → 新建便签 + 自动打开编辑模式 |
| 待办排序按钮 | `main.ts` | 清单上方「🔽 AI 排序」按钮（>3项时启用） |
| 标签推荐 | `main.ts` | 标签框旁 AI 图标 → 推荐3个标签 → 点击添加 |

### 验证

- [ ] 日历月视图 → "生成月报" → 新建便签含结构化周报内容
- [ ] 侧边栏 → "生成周报" → 汇总上周便签
- [ ] 周报内容可编辑修改后保存
- [ ] 便签含5+待办 → "AI 排序" → 按紧急度重排
- [ ] 便签标签框旁 AI 图标 → 推荐标签 → 点击添加


## 里程碑总览

```
v0.5.0  AI 基础设施 + 自然语言提醒          ← 基础打通，第一个 AI 功能
  ↓
v0.6.0  提醒嗅探 + 文本规整 + 文风切换       ← 右键菜单 AI 增强
  ↓
v0.7.0  周报/月报 + 待办排序 + 标签速配       ← 回顾增强
```

每个迭代结束：编译测试 + 提交 + 打包 + 发布 GitHub Release
