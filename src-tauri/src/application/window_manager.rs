//! Note 窗口生命周期管理：开窗、关闭、聚焦、置顶、闪烁、提醒激活、启动恢复
//!
//! 职责：
//! - Note 窗口创建（open_note_window / open_note_window_with_url）
//! - Note 窗口关闭（close_note_window，强制销毁）
//! - Note 窗口聚焦 + 事件发送（focus_note_window_and_emit）
//! - Note 窗口置顶状态（set_note_pinned / restore_note_on_top）
//! - 闪烁提示（flash_window，置顶 5s 匹配前端动画）
//! - 提醒触发激活（activate_note_for_reminder）
//! - 启动恢复（restore_all_windows，含空便签清理 + 重叠解析）
//!
//! 调用方：
//! - `commands/note_commands.rs`：开窗/关窗/置顶/恢复
//! - `note_service`：create_note / open_note / open_note_with_flag / update_note_style
//! - `template_service`：create_note_from_template
//! - `reminder_scheduler`：activate_note_for_reminder
//! - `tray_manager` / `shortcut_manager`：restore_all_windows
//! - `lib.rs` setup：restore_all_windows
//!
//! 依赖：
//! - `domain::{Note, value_objects::WindowState}`
//! - `application::window_overlap_resolver`（启动恢复时解析重叠）
//! - `tauri::AppHandle`
//!
//! 设计要点：
//! - Hub 窗口管理已拆到 `hub_window_manager`，重叠物理已拆到 `window_overlap_resolver`
//! - 本 module 聚焦 note 窗口生命周期，关注点单一

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::domain::{Note, value_objects::WindowState};

use super::window_overlap_resolver;
use super::note_service;
use super::event_names::{FLASH_WINDOW, REMINDER_TRIGGERED};

/// 闪烁提示：临时置顶 5s 匹配前端动画时长（2.5s × 2 次），立即定向发送 flash-window 事件
///
/// 关键时序：事件必须同步发送（不能放到线程延迟中），否则事件到达时窗口已恢复非置顶，
/// 被其他 always_on_top 便签遮挡，导致看不到闪烁。
///
/// 注意：必须使用 emit_to 定向发送到当前窗口，禁止使用 emit 广播（会导致所有便签都闪烁）
fn flash_window(_app: &AppHandle, window: &tauri::WebviewWindow, restore_on_top: bool) {
    let label = window.label().to_string();
    let _ = window.set_always_on_top(true);

    // 置顶时附加 pin（免疫 Win+D）
    let pin_enabled = super::pin_desktop_config::PinDesktopConfig::load()
        .map(|c| c.enabled)
        .unwrap_or(true);
    #[cfg(target_os = "windows")]
    if pin_enabled {
        let _ = super::win_pin::pin_window(window);
    }

    // 立即定向发送事件，前端开始闪烁动画（窗口处于置顶状态，可见）
    let _ = window.emit_to(&label, FLASH_WINDOW, ());
    let win_clone = window.clone();
    std::thread::spawn(move || {
        // 置顶保持 5s 匹配前端动画时长
        std::thread::sleep(std::time::Duration::from_millis(5000));
        let _ = win_clone.set_always_on_top(restore_on_top);
        // 闪烁结束后根据 restore_on_top 决定是否保持 pin
        #[cfg(target_os = "windows")]
        if !(restore_on_top && pin_enabled) {
            let _ = super::win_pin::unpin_window(&win_clone);
        }
    });
}

/// 为便签创建并显示独立窗口
pub fn open_note_window(app: &AppHandle, note: &Note) -> Result<(), String> {
    open_note_window_with_url(app, note, "index.html", note.is_pinned)
}

/// 为便签创建窗口，可指定自定义 URL（如带参数 ?reminder=1）
///
/// `keep_on_top`: 闪烁动画结束后是否保持置顶。
///   - 普通打开：传 `note.is_pinned`（恢复便签自身的置顶状态）
///   - 提醒触发：传 `true`（持续置顶直到用户操作横幅，由 restore_window_on_top 恢复）
pub fn open_note_window_with_url(app: &AppHandle, note: &Note, url: &str, keep_on_top: bool) -> Result<(), String> {
    let label = format!("note-{}", note.id);
    eprintln!("[窗口] 尝试创建窗口: label={}", label);

    // 窗口已存在 → 聚焦并闪烁提示
    if let Some(window) = app.get_webview_window(&label) {
        eprintln!("[窗口] 窗口已存在, 聚焦并闪烁");
        // 临时置顶确保窗口从最小化恢复后显示在最前
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let _ = window.set_focus();
        // flash_window 5s 后根据 keep_on_top 决定是否保持置顶
        flash_window(app, &window, keep_on_top);
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
        let app_clone = app.clone();
        let win_clone = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            flash_window(&app_clone, &win_clone, keep_on_top);
        });
    }

    Ok(())
}

/// 提醒触发时激活便签窗口
///
/// - 窗口已存在：显示+闪烁（含持续置顶）+发送 reminder-triggered 事件
/// - 窗口不存在：创建新窗口（URL 带 reminder + rid 参数），keep_on_top=true 保持置顶
///
/// 置顶持续到用户操作横幅（关闭/贪睡/完成），由前端调用 restore_window_on_top 恢复 is_pinned。
pub fn activate_note_for_reminder(app: &AppHandle, note: &Note, reminder_id: &str) -> Result<(), String> {
    let label = format!("note-{}", note.id);

    if let Some(window) = app.get_webview_window(&label) {
        // 窗口已存在 → 显示+闪烁（flash_window 内部立即置顶 + 5s 后保持置顶）+ 发送事件
        let _ = window.show();
        let _ = window.unminimize();
        flash_window(app, &window, true);
        let _ = window.set_focus();
        let _ = app.emit_to(&label, REMINDER_TRIGGERED, serde_json::json!({ "reminder_id": reminder_id }));
        eprintln!("[调度器] 窗口已存在，发送 reminder-triggered 事件: note_id={}, reminder_id={}", note.id, reminder_id);
        Ok(())
    } else {
        // 窗口不存在 → 创建新窗口（URL 带 reminder + rid 参数），keep_on_top=true 保持置顶
        let url = format!("index.html?reminder=1&rid={}", reminder_id);
        match open_note_window_with_url(app, note, &url, true) {
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
/// 检测位置重叠的便签并级联偏移，避免完全遮挡（委托 `window_overlap_resolver`）。
pub fn restore_all_windows(app: &AppHandle) -> Result<usize, String> {
    let state = app.state::<crate::AppState>();
    let notes = state.note_repo.find_all()?;
    let mut count = 0;
    let mut valid_notes: Vec<&Note> = Vec::new();
    for note in &notes {
        // INV-003：空便签不应存在，启动时清理（委托 note_service，含 emit NoteWritten(Deleted) 事件）
        if note.is_empty() {
            if let Err(e) = note_service::delete_note(
                state.note_repo.as_ref(),
                state.reminder_repo.as_ref(),
                state.event_bus.as_ref(),
                &note.id,
            ) {
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
    // 防重叠：检测相同位置的便签，级联偏移 30px（委托 window_overlap_resolver）
    window_overlap_resolver::resolve_overlaps(app, &valid_notes);
    Ok(count)
}

/// 关闭便签窗口（强制销毁，对应 INV-026）
///
/// 窗口不存在时静默跳过。使用 `destroy()` 而非 `close()`，
/// 避免 Tauri 2.x `onCloseRequested` 时序问题导致关闭失败（LES-016）。
pub fn close_note_window(app: &AppHandle, note_id: &str) {
    let label = format!("note-{}", note_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.destroy();
    }
}

/// 设置便签窗口置顶状态
///
/// 罗口时根据全局配置决定是否附加 pin()（免疫 Win+D）；
/// 取消置顶时如果之前有 pin，则 unpin() 恢复正常。
pub fn set_note_pinned(app: &AppHandle, note_id: &str, is_pinned: bool) {
    let label = format!("note-{}", note_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_always_on_top(is_pinned);

        // pin/unpin 使窗口免疫 Win+D（拦截 WM_SHOWWINDOW + WM_WINDOWPOSCHANGING）
        let pin_enabled = super::pin_desktop_config::PinDesktopConfig::load()
            .map(|c| c.enabled)
            .unwrap_or(true);
        eprintln!("[窗口] set_note_pinned: label={}, is_pinned={}, pin_enabled={}", label, is_pinned, pin_enabled);
        #[cfg(target_os = "windows")]
        {
            if is_pinned && pin_enabled {
                if let Err(e) = super::win_pin::pin_window(&win) {
                    eprintln!("[窗口] pin_window 失败: {} - {}", label, e);
                }
            } else {
                let _ = super::win_pin::unpin_window(&win);
            }
        }
    } else {
        eprintln!("[窗口] set_note_pinned: 窗口不存在 label={}", label);
    }
}

/// 恢复便签窗口置顶状态为便签自身的 `is_pinned` 值
///
/// 用于提醒触发时临时置顶后，用户操作横幅后恢复原始状态。
/// 如果恢复到非置顶，则同时 unpin。
pub fn restore_note_on_top(app: &AppHandle, note: &Note) {
    let label = format!("note-{}", note.id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_always_on_top(note.is_pinned);

        let pin_enabled = super::pin_desktop_config::PinDesktopConfig::load()
            .map(|c| c.enabled)
            .unwrap_or(true);
        #[cfg(target_os = "windows")]
        {
            if note.is_pinned && pin_enabled {
                let _ = super::win_pin::pin_window(&win);
            } else {
                let _ = super::win_pin::unpin_window(&win);
            }
        }
    }
}

/// 聚焦便签窗口并向该窗口定向发送事件
///
/// 窗口存在时聚焦 + emit_to 定向发送事件，返回 `true`；
/// 窗口不存在时返回 `false`，调用方可据此决定是否创建新窗口。
///
/// `event` 参数由调用方提供（如 `"show-reminder-panel"`），window_manager 不关心事件语义。
pub fn focus_note_window_and_emit(app: &AppHandle, note_id: &str, event: &str) -> bool {
    let label = format!("note-{}", note_id);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.set_focus();
        let _ = app.emit_to(&label, event, ());
        true
    } else {
        false
    }
}
