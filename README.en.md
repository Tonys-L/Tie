<p align="center">
  <img src="https://raw.githubusercontent.com/Tonys-L/Tie/main/src-tauri/icons/icon.png" width="80" height="80" alt="Tie">
</p>

<h1 align="center">Tie</h1>

<p align="center"><em>Tie it, find it</em></p>

<p align="center">
  Lightweight desktop notes app for capturing ideas, to-dos, and inspiration.<br>
  Website: <a href="https://tie.8421.fun?lang=en">tie.8421.fun</a> · <a href="README.md">中文</a>
</p>

---

## Features

### Edit
- Live-rendered Markdown with one-click edit/view toggle
- Drag-and-drop images auto-saved and inserted
- Checklists, tags, pin, custom colors, opacity, archive
- Note templates for one-click structured note creation
- FTS5 full-text search with keyword highlighting

### Remind
- Recurrence: one-time, daily, weekly, monthly, lunar monthly
- Calendar view: monthly + yearly, reminders at a glance
- Flash window + banner notification (snooze 5 min / mark done)

### AI
- AI Analysis: auto-detect reminders, split tasks, tidy text, suggest tags on save
- AI Rewrite: 5 right-click operations (tidy / checklist / formal / concise / mild)
- AI Todo Sort / Weekly & Monthly Reports
- Local or remote LLM, toggleable & swappable

### Sync
- Git-based multi-device sync, all data stored locally
- Private GitHub / Gitee repos supported

### More
- Bilingual (ZH/EN) / Dark mode
- Global shortcuts / Batch operations (Ctrl+click)
- Auto update check / Launch on startup
- Installer ~3MB

## Install

Download from [GitHub Releases](https://github.com/Tonys-L/Tie/releases) and run.

## Development

### Prerequisites

- Node.js 18+
- Rust (rustup)
- Tauri CLI 2.0

### Dev

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

## Tech Stack

| Layer | Tech |
|----|------|
| Desktop | Tauri 2.0 |
| Backend | Rust |
| Frontend | TypeScript + Vite |
| Storage | SQLite + FTS5 |
| Sync | Git |
| Lunar Calendar | tyme4rs |

## License

[MIT](LICENSE)
