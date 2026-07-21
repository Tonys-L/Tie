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
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(800));
            flash_window(&win_clone, keep_on_top);
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
        flash_window(&window, true);
        let _ = window.set_focus();
        let _ = app.emit_to(&label, "reminder-triggered", serde_json::json!({ "reminder_id": reminder_id }));
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
/// 检测位置重叠的便签并级联偏移，避免完全遮挡。
pub fn restore_all_windows(app: &AppHandle) -> Result<usize, String> {
    let state = app.state::<crate::AppState>();
    let notes = state.note_repo.find_all()?;
    let mut count = 0;
    let mut valid_notes: Vec<&Note> = Vec::new();
    for note in &notes {
        // INV-003：空便签不应存在，启动时清理
        if note.is_empty() {
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

/// 计算便签位置重叠的偏移结果（纯函数，无 Tauri 依赖）
///
/// 对相同位置的便签按出现顺序级联偏移 30px（x 和 y 同时偏移）。
/// 第一个同位置便签不偏移，后续每个递增 30px。
///
/// 返回需要偏移的便签列表：(note_id, new_pos_x, new_pos_y)。
/// 不偏移的便签（首位）不在返回列表中。
fn compute_overlaps(notes: &[&Note]) -> Vec<(String, i32, i32)> {
    let mut seen_positions: HashMap<(i32, i32), usize> = HashMap::new();
    const OFFSET_PX: i32 = 30;
    let mut result = Vec::new();

    for note in notes {
        let key = (note.window_state.pos_x, note.window_state.pos_y);
        let dup_index = seen_positions.entry(key).or_insert(0);
        if *dup_index > 0 {
            let offset = (*dup_index as i32) * OFFSET_PX;
            result.push((
                note.id.clone(),
                note.window_state.pos_x + offset,
                note.window_state.pos_y + offset,
            ));
        }
        *dup_index += 1;
    }
    result
}

/// 检测位置重叠的便签窗口，对后续同位置便签级联偏移
///
/// 偏移量 = 重复序号 × 30px（x 和 y 同时偏移），形成层叠效果。
/// 仅移动窗口位置，不修改 DB 中的 window_state（下次启动仍会检测并偏移）。
///
/// 委托 `compute_overlaps` 计算偏移结果，再遍历执行 Tauri `set_position` 副作用。
fn resolve_overlaps(app: &AppHandle, notes: &[&Note]) {
    for (note_id, new_x, new_y) in compute_overlaps(notes) {
        let label = format!("note-{}", note_id);
        if let Some(win) = app.get_webview_window(&label) {
            let _ = win.set_position(tauri::Position::Logical(
                tauri::LogicalPosition::new(new_x as f64, new_y as f64),
            ));
        }
    }
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
/// 窗口不存在时静默跳过。用于 `update_note_style` 等场景同步窗口的 always_on_top。
pub fn set_note_pinned(app: &AppHandle, note_id: &str, is_pinned: bool) {
    let label = format!("note-{}", note_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_always_on_top(is_pinned);
    }
}

/// 恢复便签窗口置顶状态为便签自身的 `is_pinned` 值
///
/// 用于提醒触发时临时置顶后，用户操作横幅后恢复原始状态。
pub fn restore_note_on_top(app: &AppHandle, note: &Note) {
    let label = format!("note-{}", note.id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.set_always_on_top(note.is_pinned);
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

/// 切换 Hub（设置中心）窗口可见性：已显示则隐藏，隐藏或未创建则显示
pub fn toggle_hub_window(app: &AppHandle) {
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
    create_hub_window(app);
}

/// 打开或聚焦 Hub 窗口（托盘菜单调用）
///
/// 已存在则 unminimize + show + set_focus；不存在则创建。
/// 与 `toggle_hub_window` 的差异：本函数始终显示（用于托盘菜单点击），
/// `toggle_hub_window` 切换可见性（用于快捷键）。
pub fn open_or_focus_hub(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("hub") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    create_hub_window(app);
}

/// 创建 Hub 窗口（统一入口，消除 toggle_hub_window 与 tray_manager::handle_hub 的重复）
fn create_hub_window(app: &AppHandle) {
    use crate::application::locale_manager;
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
        eprintln!("[窗口] 创建 Hub 窗口失败");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Note;
    use crate::domain::value_objects::WindowState;

    /// 构造测试用 Note（指定 id + 位置）
    fn make_note(id: &str, pos_x: i32, pos_y: i32) -> Note {
        let mut note = Note::new("测试".to_string(), "amber".to_string());
        note.id = id.to_string();
        note.window_state = WindowState {
            pos_x,
            pos_y,
            width: 320,
            height: 280,
        };
        note
    }

    #[test]
    fn test_compute_overlaps_no_overlap() {
        // 所有位置唯一 → 返回空 Vec
        let n1 = make_note("n1", 100, 100);
        let n2 = make_note("n2", 200, 200);
        let n3 = make_note("n3", 300, 300);
        let notes: Vec<&Note> = vec![&n1, &n2, &n3];

        let result = compute_overlaps(&notes);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_overlaps_two_same_position() {
        // 2 个同位置 → 第 2 个偏移 30px
        let n1 = make_note("n1", 100, 100);
        let n2 = make_note("n2", 100, 100);
        let notes: Vec<&Note> = vec![&n1, &n2];

        let result = compute_overlaps(&notes);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "n2");
        assert_eq!(result[0].1, 130); // 100 + 30
        assert_eq!(result[0].2, 130);
    }

    #[test]
    fn test_compute_overlaps_three_same_position() {
        // 3 个同位置 → 第 2 个偏移 30px，第 3 个偏移 60px
        let n1 = make_note("n1", 50, 50);
        let n2 = make_note("n2", 50, 50);
        let n3 = make_note("n3", 50, 50);
        let notes: Vec<&Note> = vec![&n1, &n2, &n3];

        let result = compute_overlaps(&notes);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "n2");
        assert_eq!(result[0].1, 80); // 50 + 30
        assert_eq!(result[1].0, "n3");
        assert_eq!(result[1].1, 110); // 50 + 60
    }

    #[test]
    fn test_compute_overlaps_multiple_groups() {
        // 多组不同位置的重叠 → 各组独立计算
        // 组 A: (100,100) 出现 2 次
        // 组 B: (200,200) 出现 3 次
        let n1 = make_note("n1", 100, 100);
        let n2 = make_note("n2", 200, 200);
        let n3 = make_note("n3", 100, 100); // 组 A 第 2 个 → +30
        let n4 = make_note("n4", 200, 200); // 组 B 第 2 个 → +30
        let n5 = make_note("n5", 200, 200); // 组 B 第 3 个 → +60
        let notes: Vec<&Note> = vec![&n1, &n2, &n3, &n4, &n5];

        let result = compute_overlaps(&notes);
        assert_eq!(result.len(), 3);
        // n3: 组 A 第 2 个 → (130, 130)
        assert_eq!(result[0].0, "n3");
        assert_eq!(result[0].1, 130);
        // n4: 组 B 第 2 个 → (230, 230)
        assert_eq!(result[1].0, "n4");
        assert_eq!(result[1].1, 230);
        // n5: 组 B 第 3 个 → (260, 260)
        assert_eq!(result[2].0, "n5");
        assert_eq!(result[2].1, 260);
    }
}
