# i18n 收尾实施计划

## 背景

i18n 框架已完整搭建（`t()`、`applyLocale()`、语言包），`main.ts` 和 `hub.ts` 中绝大多数用户可见文本已使用 `t()` 替换。但仍有少量硬编码中文和硬编码 locale 未处理。

## 剩余问题

### 1. hub.ts 中 3 处硬编码错误消息

- L331: `'保存失败: ' + e` → 需替换为 `t()`
- L350: `'同步失败: ' + e` → 需替换为 `t()`
- L403: `'保存失败: ' + e` → 需替换为 `t()`

需要在 zh.ts/en.ts 中新增对应 key（如 `hub.saveFailed`、`hub.syncFailed`），并拼接错误详情。

### 2. formatDate 中硬编码 'zh-CN'

`src/utils.ts` L19 的 `d.toLocaleTimeString('zh-CN', ...)` 应根据当前语言动态选择 locale。

### 3. toLocaleString 调用中硬编码 'zh-CN'

- `src/main.ts` L412: `new Date(r.remind_at).toLocaleString('zh-CN', ...)`
- `src/hub.ts` L257: `new Date(r.remind_at).toLocaleString('zh-CN', ...)`

应改为根据 `getLocale()` 动态获取 locale。

## 修改范围

| 文件 | 修改内容 |
|------|----------|
| `src/i18n/zh.ts` | 新增 `hub.saveFailed`、`hub.syncFailed` |
| `src/i18n/en.ts` | 新增对应英文翻译 |
| `src/i18n/index.ts` | 新增 `getLocaleTag()` 辅助函数，返回 BCP 47 locale tag（zh → zh-CN, en → en-US） |
| `src/hub.ts` | 3 处硬编码错误消息替换为 `t()`；1 处 `toLocaleString('zh-CN')` 改为 `toLocaleString(getLocaleTag())` |
| `src/main.ts` | 1 处 `toLocaleString('zh-CN')` 改为 `toLocaleString(getLocaleTag())` |
| `src/utils.ts` | `formatDate` 中 `toLocaleTimeString('zh-CN')` 改为 `toLocaleTimeString(getLocaleTag())` |

## 实施步骤

1. 在 `src/i18n/index.ts` 中新增 `getLocaleTag()` 函数
2. 在 `zh.ts` 和 `en.ts` 中新增 `hub.saveFailed` / `hub.syncFailed` key
3. 修改 `src/utils.ts` 的 `formatDate` 使用动态 locale
4. 修改 `src/main.ts` 的 `toLocaleString` 使用动态 locale
5. 修改 `src/hub.ts` 的硬编码错误消息和 `toLocaleString`

## 验证

- 启动应用 `npm run tauri dev`，确认中文显示正常
- 在 Hub 设置中心切换到英文，确认所有文本切换为英文
- 便签窗口中切换语言，确认日期格式随语言变化
- 故意触发同步错误，确认错误消息随语言切换
