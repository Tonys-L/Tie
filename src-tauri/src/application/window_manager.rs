use std::collections::HashMap;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::{Note, value_objects::WindowState};

/// 闪烁提示：临时置顶 5s 匹配前端动画时长（2.5s × 2 次），立即定向发送 flash-window 事件
///
/// 关键时序：事件必须同步发送（不能放到线程延迟中），否则事件到达时窗口已恢复非置顶，
/// 被其他 always_on_top 便签遮挡，导致看不到闪烁。
///
/// 注意：必须使用 emit_to 定向发送到当前窗口，禁止使用 emit 广播（会导致所有便签都闪烁）
fn flash_window(window: &tauri::WebviewWindow, restore_on_top: bool) {
    let label = window.label().to_string();
    let _ = window.set_always_on_top(true);
    // 立即定向发送事件，前端开始闪烁动画（窗口处于置顶状态，可见）
    let _ = window.emit_to(&label, "flash-window", ());
    let win_clone = window.clone();
    std::thread::spawn(move || {
        // 置顶保持 5s 匹配前端动画时长
        std::thread::sleep(std::time::Duration::from_millis(5000));
        let _ = win_clone.set_always_on_top(restore_on_top);
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

    // 修正异常尺寸（DB 中可能存了极小值）
    let w = (note.window_state.width as u32).max(WindowState::MIN_WIDTH) as f64;
    let h = (note.window_state.height as u32).max(WindowState::MIN_HEIGHT) as f64;

    let _window = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title("便签")
        .inner_size(w, h)
        .min_inner_size(WindowState::MIN_WIDTH as f64, WindowState::MIN_HEIGHT as f64)
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
    // 闪烁延迟 800ms，等前端 JS 加载并注册事件监听后再发送
    // （新建窗口的前端页面还在加载，立即 emit_to 事件会丢失）
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.show();
        let win_clone = win.clone();
        let is_pinned = note.is_pinned;
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            flash_window(&win_clone, is_pinned);
        });
    }

    Ok(())
}

/// 提醒触发时激活便签窗口
///
/// - 窗口已存在：显示+聚焦+发送 reminder-triggered 事件（携带 reminder_id）+闪烁
/// - 窗口不存在：创建新窗口（URL 带 reminder + rid 参数）
pub fn activate_note_for_reminder(app: &AppHandle, note: &Note, reminder_id: &str) -> Result<(), String> {
    let label = format!("note-{}", note.id);

    if let Some(window) = app.get_webview_window(&label) {
        // 窗口已存在 → 显示+聚焦+发送事件+闪烁
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit_to(&label, "reminder-triggered", serde_json::json!({ "reminder_id": reminder_id }));
        eprintln!("[调度器] 窗口已存在，发送 reminder-triggered 事件: note_id={}, reminder_id={}", note.id, reminder_id);
        flash_window(&window, note.is_pinned);
        Ok(())
    } else {
        // 窗口不存在 → 创建新窗口（URL 带 reminder + rid 参数）
        let url = format!("index.html?reminder=1&rid={}", reminder_id);
        match open_note_window_with_url(app, note, &url) {
            Ok(_) => {
                eprintln!("[调度器] 便签窗口已弹出: note_id={}, reminder_id={}", note.id, reminder_id);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

/// 打开所有已保存便签的窗口（启动时恢复）
///
/// 空便签（无标题且无内容）直接删除（INV-003），不创建窗口。
/// 检测位置重叠的便签并级联偏移，避免完全遮挡。
pub fn restore_all_windows(app: &AppHandle) -> Result<usize, String> {
    let state = app.state::<crate::AppState>();
    let notes = state.note_repo.find_all()?;
    let mut count = 0;
    let mut valid_notes: Vec<&Note> = Vec::new();
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
        } else {
            valid_notes.push(note);
        }
        count += 1;
    }
    // 防重叠：检测相同位置的便签，级联偏移 30px
    resolve_overlaps(app, &valid_notes);
    Ok(count)
}

/// 检测位置重叠的便签窗口，对后续同位置便签级联偏移
///
/// 偏移量 = 重复序号 × 30px（x 和 y 同时偏移），形成层叠效果。
/// 仅移动窗口位置，不修改 DB 中的 window_state（下次启动仍会检测并偏移）。
fn resolve_overlaps(app: &AppHandle, notes: &[&Note]) {
    let mut seen_positions: HashMap<(i32, i32), usize> = HashMap::new();
    const OFFSET_PX: i32 = 30;

    for note in notes {
        let key = (note.window_state.pos_x, note.window_state.pos_y);
        let dup_index = seen_positions.entry(key).or_insert(0);
        if *dup_index > 0 {
            let offset = (*dup_index as i32) * OFFSET_PX;
            let label = format!("note-{}", note.id);
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.set_position(tauri::Position::Logical(
                    tauri::LogicalPosition::new(
                        (note.window_state.pos_x + offset) as f64,
                        (note.window_state.pos_y + offset) as f64,
                    ),
                ));
            }
        }
        *dup_index += 1;
    }
}

/// 切换 Hub（设置中心）窗口可见性：已显示则隐藏，隐藏或未创建则显示
pub fn toggle_hub_window(app: &AppHandle) {
    use crate::application::locale_manager;
    if let Some(window) = app.get_webview_window("hub") {
        // 已存在：切换可见性
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
        return;
    }
    // 不存在：创建新窗口
    let _window = WebviewWindowBuilder::new(app, "hub", WebviewUrl::App("hub.html".into()))
        .title(locale_manager::menu_hub_title())
        .inner_size(640.0, 520.0)
        .decorations(true)
        .transparent(false)
        .resizable(true)
        .always_on_top(false)
        .disable_drag_drop_handler()
        .build();

    if _window.is_err() {
        eprintln!("[快捷键] 切换 Hub 窗口失败");
    }
}
