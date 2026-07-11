# 国际化（i18n）实施计划

## Context

当前应用所有 UI 文本硬编码为中文。需要添加中英文国际化支持，让用户可切换语言。

**范围**：仅前端（TypeScript/HTML），后端 Rust 错误消息暂不国际化（面向开发者，用户不可见）。

## 方案

**不引入第三方 i18n 库**（项目规模小，自建更轻量）：

- 新建 `src/i18n/index.ts`：语言包定义 + `t(key)` 翻译函数 + `setLocale`/`getLocale`
- 新建 `src/i18n/zh.ts`：中文语言包
- 新建 `src/i18n/en.ts`：英文语言包
- 语言偏好存储在 `localStorage`，默认跟随系统语言
- Hub 设置页侧边栏添加语言切换按钮

## 语言包结构

```typescript
// src/i18n/zh.ts
export default {
  // 通用
  app: { name: 'AI 便签', settings: '设置中心', note: '便签' },
  // 便签窗口 (main.ts)
  note: {
    placeholder: '写点什么...',
    noSelection: '未选择便签',
    notExist: '便签不存在',
    loadFailed: '加载失败',
    reminderBanner: '提醒时间到了！',
    reminderMe: '提醒我',
    setReminder: '设置提醒',
    title: '标题',
    pin: '置顶',
    close: '关闭',
    archive: '归档',
    delete: '删除便签',
    existingReminders: '已有 {n} 条提醒',
    once: '一次', daily: '每天', weekly: '每周', monthly: '每月',
    oneHour: '1小时', threeHours: '3小时',
    tomorrow: '明天', nextMonday: '下周一',
    deleteConfirm: '确认删除此便签？',
    cancel: '取消',
  },
  // Hub 页面 (hub.ts + hub.html)
  hub: {
    noteManager: '便签管理',
    syncSettings: '同步设置',
    shortcuts: '快捷键',
    about: '关于',
    darkMode: '深色模式',
    lightMode: '浅色模式',
    // 便签管理
    activeNotes: '活跃便签',
    reminders: '提醒',
    archived: '已归档',
    searchNotes: '搜索便签...',
    noMatch: '没有匹配的便签',
    noReminders: '暂无提醒',
    noActive: '暂无活跃便签',
    noArchived: '暂无归档便签',
    noTitle: '无标题',
    noContent: '无内容',
    restore: '恢复',
    // 同步设置
    syncTitle: '同步设置',
    syncDesc: '通过 Git 仓库在多台设备间同步便签数据',
    repoConfig: '仓库配置',
    repoUrl: '仓库地址',
    username: '用户名',
    accessToken: '访问令牌',
    branch: '分支名',
    syncOptions: '同步选项',
    autoSync: '自动同步',
    autoSyncDesc: '内容变更后自动推送，启动时自动拉取',
    saveConfig: '保存配置',
    syncNow: '立即同步',
    syncing: '正在同步...',
    configSaved: '配置已保存',
    gitInstalled: 'Git 已安装，可以正常同步',
    gitNotInstalled: '未检测到 Git，请先安装并添加到 PATH',
    // 快捷键
    shortcutTitle: '快捷键设置',
    shortcutDesc: '自定义全局快捷键，修改后即时生效',
    newNote: '新建便签',
    showAll: '显示全部便签',
    shortcutSaved: '快捷键已保存并生效',
    shortcutEmpty: '快捷键不能为空',
    resetDefault: '恢复默认',
    saveShortcuts: '保存快捷键',
    // 删除确认
    deleteConfirm: '删除便签？',
    deleteIrreversible: '此操作不可撤销',
    // 关于
    techStack: '技术栈',
    dataStorage: '数据存储',
    features: '功能',
    shortcut: '快捷键',
    loading: '加载中...',
  },
};
```

## 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `src/i18n/index.ts` | **新建**：翻译函数 + 语言切换 |
| `src/i18n/zh.ts` | **新建**：中文语言包 |
| `src/i18n/en.ts` | **新建**：英文语言包 |
| `src/main.ts` | 所有中文硬编码替换为 `t()` 调用 |
| `src/hub.ts` | 所有中文硬编码替换为 `t()` 调用 |
| `src/hub.html` | 静态中文文本替换为 `data-i18n` 属性 + JS 初始化 |
| `src/utils.ts` | `repeatLabel` 改为基于 i18n |
| `src/index.html` | `<html lang>` 改为动态 |

## 核心实现

### i18n/index.ts

```typescript
import zh from './zh';
import en from './en';

const locales: Record<string, any> = { zh, en };
let current = 'zh';

export function getLocale(): string { return current; }

export function setLocale(lang: string) {
  current = lang;
  localStorage.setItem('locale', lang);
}

// 初始化：localStorage > navigator.language > 默认 zh
export function initLocale() {
  const saved = localStorage.getItem('locale');
  if (saved && locales[saved]) { current = saved; return; }
  const nav = navigator.language.toLowerCase();
  if (nav.startsWith('zh')) current = 'zh';
  else current = 'en';
}

// 按点号路径取值，支持插值 {n}
export function t(key: string, params?: Record<string, string | number>): string {
  const keys = key.split('.');
  let val: any = locales[current];
  for (const k of keys) { val = val?.[k]; }
  if (typeof val !== 'string') val = key; // fallback
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      val = val.replace(`{${k}}`, String(v));
    });
  }
  return val;
}
```

### hub.html 处理

静态 HTML 中的中文文本（侧边栏导航、表单标签等）用 `data-i18n` 属性标记，JS 初始化时统一替换：

```html
<div class="sidebar-title" data-i18n="app.name">AI 便签</div>
<div class="sidebar-subtitle" data-i18n="app.settings">设置中心</div>
```

```typescript
// 初始化时替换所有 data-i18n
document.querySelectorAll('[data-i18n]').forEach(el => {
  el.textContent = t(el.getAttribute('data-i18n')!);
});
document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
  (el as HTMLInputElement).placeholder = t(el.getAttribute('data-i18n-placeholder')!);
});
```

### 语言切换 UI

Hub 侧边栏 footer 区域添加语言切换按钮（主题按钮旁），点击切换中英文并刷新页面文本。

## 验证方式

1. `cargo check` 编译通过
2. 默认中文界面正常显示
3. 切换到英文后所有文本正确
4. 重启应用后语言偏好保持
5. 提醒面板、删除确认、同步状态等动态文本正确
