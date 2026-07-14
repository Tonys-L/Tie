use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::Note;

/// 闪烁提示：临时置顶 300ms 后恢复原状态
fn flash_window(window: &tauri::WebviewWindow, restore_on_top: bool) {
    let _ = window.set_always_on_top(true);
    let win_clone = window.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = win_clone.set_always_on_top(restore_on_top);
        // 发送事件让前端闪烁
        let _ = win_clone.emit("flash-window", ());
    });
}

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
        let was_on_top = window.is_always_on_top().unwrap_or(false);
        flash_window(&window, was_on_top);
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
        .disable_drag_drop_handler()
        .build()
        .map_err(|e| {
            eprintln!("[窗口] 创建失败: {}", e);
            e.to_string()
        })?;

    eprintln!("[窗口] 创建成功: {}", label);

    // 新建窗口需要显式置顶+显示，确保出现在最前面
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        flash_window(&win, note.is_pinned);
    }

    Ok(())
}

/// 提醒触发时激活便签窗口
///
/// - 窗口已存在：显示+聚焦+发送 reminder-triggered 事件+闪烁
/// - 窗口不存在：创建新窗口（URL 带 reminder 参数）
pub fn activate_note_for_reminder(app: &AppHandle, note: &Note) -> Result<(), String> {
    let label = format!("note-{}", note.id);

    if let Some(window) = app.get_webview_window(&label) {
        // 窗口已存在 → 显示+聚焦+发送事件+闪烁
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit_to(&label, "reminder-triggered", ());
        eprintln!("[调度器] 窗口已存在，发送 reminder-triggered 事件: note_id={}", note.id);
        flash_window(&window, note.is_pinned);
        Ok(())
    } else {
        // 窗口不存在 → 创建新窗口（URL 带 reminder 参数）
        match open_note_window_with_url(app, note, "index.html?reminder=1") {
            Ok(_) => {
                eprintln!("[调度器] 便签窗口已弹出: note_id={}", note.id);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
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
