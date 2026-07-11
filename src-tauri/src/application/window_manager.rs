use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::Note;

/// 为便签创建并显示独立窗口
pub fn open_note_window(app: &AppHandle, note: &Note) -> Result<(), String> {
    open_note_window_with_url(app, note, "index.html")
}

/// 为便签创建窗口，可指定自定义 URL（如带参数 ?reminder=1）
pub fn open_note_window_with_url(app: &AppHandle, note: &Note, url: &str) -> Result<(), String> {
    let label = format!("note-{}", note.id);
    eprintln!("[窗口] 尝试创建窗口: label={}", label);

    // 窗口已存在 → 聚焦并闪烁提示
    if let Some(window) = app.get_webview_window(&label) {
        eprintln!("[窗口] 窗口已存在, 聚焦并闪烁");
        let _ = window.set_focus();
        let _ = window.show();
        // 闪烁：临时置顶 → 延时恢复原状态
        let was_on_top = window.is_always_on_top().unwrap_or(false);
        let _ = window.set_always_on_top(true);
        let win_clone = window.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = win_clone.set_always_on_top(was_on_top);
            // 发送事件让前端闪烁
            let _ = win_clone.emit("flash-window", ());
        });
        return Ok(());
    }

    eprintln!("[窗口] 正在构建窗口, pos=({},{}) size=({},{})",
        note.window_state.pos_x, note.window_state.pos_y,
        note.window_state.width, note.window_state.height);

    let _window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title("便签")
        .inner_size(note.window_state.width as f64, note.window_state.height as f64)
        .position(note.window_state.pos_x as f64, note.window_state.pos_y as f64)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(note.is_pinned)
        .skip_taskbar(false)
        .resizable(true)
        .visible(false)
        .build()
        .map_err(|e| {
            eprintln!("[窗口] 创建失败: {}", e);
            e.to_string()
        })?;

    eprintln!("[窗口] 创建成功: {}", label);

    // 新建窗口需要显式置顶+显示，确保出现在最前面
    let is_pinned = note.is_pinned;
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_always_on_top(true);
        let _ = win.show();
        let win_clone = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = win_clone.set_always_on_top(is_pinned);
        });
    }

    Ok(())
}

/// 打开所有已保存便签的窗口（启动时恢复）
///
/// 空便签（无标题且无内容）直接删除（INV-003），不创建窗口。
pub fn restore_all_windows(app: &AppHandle) -> Result<usize, String> {
    let state = app.state::<crate::AppState>();
    let notes = state.note_repo.find_all()?;
    let mut count = 0;
    for note in &notes {
        // INV-003：空便签不应存在，启动时清理
        if note.title.is_empty() && note.content.is_empty() {
            if let Err(e) = state.note_repo.delete(&note.id) {
                eprintln!("[恢复] 空便签删除失败 {}: {}", note.id, e);
            } else {
                eprintln!("[恢复] 空便签已清理: {}", note.id);
            }
            continue;
        }
        if let Err(e) = open_note_window(app, note) {
            eprintln!("[恢复] 便签 {} 窗口创建失败: {}", note.id, e);
        }
        count += 1;
    }
    Ok(count)
}
