# 国际化（i18n）实施计划

## 概要

为便签应用添加中英文国际化支持，不引入第三方库，自建轻量 i18n 系统。

## 当前状态分析

- 所有 UI 文本硬编码为中文，分布在 `main.ts`、`hub.ts`、`hub.html`、`utils.ts` 中
- `index.html` 的 `<html lang="zh-CN">` 和 `<title>便签</title>` 需动态化
- 不存在 `src/i18n/` 目录

## 修改范围

| 步骤 | 文件 | 操作 | 说明 |
|------|------|------|------|
| 1 | `src/i18n/index.ts` | 新建 | 翻译函数 `t(key)` + 语言初始化/切换 |
| 2 | `src/i18n/zh.ts` | 新建 | 中文语言包 |
| 3 | `src/i18n/en.ts` | 新建 | 英文语言包 |
| 4 | `src/main.ts` | 修改 | ~15处中文替换为 `t()` |
| 5 | `src/hub.ts` | 修改 | ~30+处中文替换为 `t()` |
| 6 | `hub.html` | 修改 | ~20+处静态中文加 `data-i18n` 属性 + 初始化脚本 |
| 7 | `src/utils.ts` | 修改 | `repeatLabel` 改为基于 i18n |
| 8 | `index.html` | 修改 | title 改为动态 |

## 详细设计

### 1. i18n/index.ts

```typescript
import zh from './zh';
import en from './en';

const locales: Record<string, any> = { zh, en };
let current = 'zh';

export function getLocale(): string { return current; }

export function setLocale(lang: string): void {
  current = lang;
  localStorage.setItem('locale', lang);
}

export function initLocale(): void {
  const saved = localStorage.getItem('locale');
  if (saved && locales[saved]) { current = saved; return; }
  current = navigator.language.toLowerCase().startsWith('zh') ? 'zh' : 'en';
}

export function t(key: string, params?: Record<string, string | number>): string {
  const keys = key.split('.');
  let val: any = locales[current];
  for (const k of keys) { val = val?.[k]; }
  if (typeof val !== 'string') val = key;
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      val = val.replace(`{${k}}`, String(v));
    });
  }
  return val;
}

// 替换所有 data-i18n 元素
export function applyLocale(): void {
  document.querySelectorAll('[data-i18n]').forEach(el => {
    el.textContent = t(el.getAttribute('data-i18n')!);
  });
  document.querySelectorAll('[data-i18n-placeholder]').forEach(el => {
    (el as HTMLInputElement).placeholder = t(el.getAttribute('data-i18n-placeholder')!);
  });
  document.querySelectorAll('[data-i18n-title]').forEach(el => {
    (el as HTMLElement).title = t(el.getAttribute('data-i18n-title')!);
  });
}
```

### 2. 语言包键值设计

```typescript
// zh.ts
export default {
  app: { name: 'AI 便签', settings: '设置中心', note: '便签' },
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
    deleteBtn: '删除',
  },
  hub: {
    noteManager: '便签管理',
    syncSettings: '同步设置',
    shortcuts: '快捷键',
    about: '关于',
    darkMode: '深色模式',
    lightMode: '浅色模式',
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
    reminderFor: '提醒：',
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
    shortcutTitle: '快捷键设置',
    shortcutDesc: '自定义全局快捷键，修改后即时生效',
    shortcutConfig: '快捷键配置',
    newNote: '新建便签',
    showAll: '显示全部便签',
    shortcutSaved: '快捷键已保存并生效',
    shortcutEmpty: '快捷键不能为空',
    resetDefault: '恢复默认',
    saveShortcuts: '保存快捷键',
    deleteConfirm: '删除便签？',
    deleteIrreversible: '此操作不可撤销',
    techStack: '技术栈',
    dataStorage: '数据存储',
    features: '功能',
    shortcut: '快捷键',
    loading: '加载中...',
    langSwitch: 'English',
    desktopApp: '桌面便签应用',
    privateRepo: '在 Gitee / GitHub 创建一个私有仓库，填入 HTTPS 地址',
    giteeToken: 'Gitee 生成令牌',
    githubToken: 'GitHub 生成令牌',
    pasteToken: '粘贴访问令牌',
    yourUsername: '你的用户名',
    shortcutFormat: '格式：ctrl+alt/shift+字母，如 ctrl+shift+n',
  },
};
```

```typescript
// en.ts - 对应英文翻译
export default {
  app: { name: 'AI Notes', settings: 'Settings', note: 'Note' },
  note: {
    placeholder: 'Write something...',
    noSelection: 'No note selected',
    notExist: 'Note not found',
    loadFailed: 'Failed to load',
    reminderBanner: "It's reminder time!",
    reminderMe: 'Remind me',
    setReminder: 'Set Reminder',
    title: 'Title',
    pin: 'Pin',
    close: 'Close',
    archive: 'Archive',
    delete: 'Delete Note',
    existingReminders: '{n} reminder(s)',
    once: 'Once', daily: 'Daily', weekly: 'Weekly', monthly: 'Monthly',
    oneHour: '1h', threeHours: '3h',
    tomorrow: 'Tomorrow', nextMonday: 'Next Mon',
    deleteConfirm: 'Delete this note?',
    cancel: 'Cancel',
    deleteBtn: 'Delete',
  },
  hub: {
    noteManager: 'Notes',
    syncSettings: 'Sync',
    shortcuts: 'Shortcuts',
    about: 'About',
    darkMode: 'Dark Mode',
    lightMode: 'Light Mode',
    activeNotes: 'Active',
    reminders: 'Reminders',
    archived: 'Archived',
    searchNotes: 'Search notes...',
    noMatch: 'No matching notes',
    noReminders: 'No reminders',
    noActive: 'No active notes',
    noArchived: 'No archived notes',
    noTitle: 'Untitled',
    noContent: 'No content',
    restore: 'Restore',
    reminderFor: 'Remind: ',
    syncTitle: 'Sync Settings',
    syncDesc: 'Sync notes across devices via Git repository',
    repoConfig: 'Repository',
    repoUrl: 'Repository URL',
    username: 'Username',
    accessToken: 'Access Token',
    branch: 'Branch',
    syncOptions: 'Sync Options',
    autoSync: 'Auto Sync',
    autoSyncDesc: 'Auto push on changes, auto pull on startup',
    saveConfig: 'Save Config',
    syncNow: 'Sync Now',
    syncing: 'Syncing...',
    configSaved: 'Config saved',
    gitInstalled: 'Git installed, ready to sync',
    gitNotInstalled: 'Git not detected. Please install and add to PATH',
    shortcutTitle: 'Shortcut Settings',
    shortcutDesc: 'Customize global shortcuts, takes effect immediately',
    shortcutConfig: 'Shortcuts',
    newNote: 'New Note',
    showAll: 'Show All Notes',
    shortcutSaved: 'Shortcuts saved',
    shortcutEmpty: 'Shortcuts cannot be empty',
    resetDefault: 'Reset Default',
    saveShortcuts: 'Save Shortcuts',
    deleteConfirm: 'Delete note?',
    deleteIrreversible: 'This action cannot be undone',
    techStack: 'Tech Stack',
    dataStorage: 'Data Storage',
    features: 'Features',
    shortcut: 'Shortcuts',
    loading: 'Loading...',
    langSwitch: '中文',
    desktopApp: 'Desktop Notes App',
    privateRepo: 'Create a private repo on Gitee/GitHub and enter the HTTPS URL',
    giteeToken: 'Gitee Token',
    githubToken: 'GitHub Token',
    pasteToken: 'Paste access token',
    yourUsername: 'Your username',
    shortcutFormat: 'Format: ctrl+alt/shift+letter, e.g. ctrl+shift+n',
  },
};
```

### 3. hub.html 处理

静态 HTML 中的中文用 `data-i18n` 属性标记，JS 初始化时通过 `applyLocale()` 统一替换。

示例改动：
```html
<!-- 之前 -->
<div class="sidebar-title">AI 便签</div>
<!-- 之后 -->
<div class="sidebar-title" data-i18n="app.name">AI 便签</div>

<!-- 之前 -->
<input type="text" id="search" placeholder="搜索便签..." />
<!-- 之后 -->
<input type="text" id="search" data-i18n-placeholder="hub.searchNotes" placeholder="搜索便签..." />
```

### 4. 语言切换 UI

在 Hub 侧边栏 footer 区域，主题切换按钮上方添加语言切换按钮：
```html
<button class="theme-toggle-btn" id="lang-btn">
  <svg>...</svg>
  <span id="lang-label">English</span>
</button>
```

点击切换逻辑：
```typescript
document.getElementById('lang-btn')?.addEventListener('click', () => {
  const newLang = getLocale() === 'zh' ? 'en' : 'zh';
  setLocale(newLang);
  applyLocale();
  document.getElementById('lang-label')!.textContent = t('hub.langSwitch');
  loadNotes(); // 刷新列表中的动态文本
});
```

### 5. main.ts 改动要点

便签窗口是独立 webview，也需要初始化 i18n。在文件顶部调用 `initLocale()`，然后所有中文硬编码替换为 `t()` 调用。

### 6. utils.ts 改动

`repeatLabel` 改为：
```typescript
import { t } from './i18n';

export function repeatLabel(type: string): string {
  const map: Record<string, string> = {
    none: '', once: '',
    daily: t('note.daily'),
    weekly: t('note.weekly'),
    monthly: t('note.monthly'),
  };
  return map[type] || type;
}
```

### 7. index.html

```html
<title data-i18n="app.note">便签</title>
```

## 实施步骤

1. 创建 `src/i18n/index.ts`
2. 创建 `src/i18n/zh.ts`
3. 创建 `src/i18n/en.ts`
4. 修改 `src/utils.ts` - repeatLabel 改为基于 i18n
5. 修改 `src/main.ts` - 替换所有中文为 t() 调用
6. 修改 `src/hub.ts` - 替换所有中文为 t() 调用
7. 修改 `hub.html` - 添加 data-i18n 属性 + 语言切换按钮
8. 修改 `index.html` - title 动态化

## 验证

1. 默认中文界面正常
2. Hub 页面切换到英文后所有文本正确
3. 便签窗口文本跟随语言设置
4. 重启应用后语言偏好保持（localStorage）
5. 提醒面板、删除确认、同步状态等动态文本正确
