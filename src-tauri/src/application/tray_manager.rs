use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use super::locale_manager;

/// 构造托盘菜单（setup_tray 与 rebuild_tray_menu 共用，消除重复）
fn build_tray_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, String> {
    let new_note = MenuItem::with_id(app, "new_note", locale_manager::menu_new_note(), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let show_all = MenuItem::with_id(app, "show_all", locale_manager::menu_show_all(), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let hub = MenuItem::with_id(app, "hub", locale_manager::menu_hub(), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let sync_now = MenuItem::with_id(app, "sync_now", locale_manager::menu_sync_now(), true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let separator2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let quit = MenuItem::with_id(app, "quit", locale_manager::menu_quit(), true, None::<&str>)
        .map_err(|e| e.to_string())?;

    Menu::with_items(app, &[&new_note, &show_all, &hub, &separator1, &sync_now, &separator2, &quit])
        .map_err(|e| e.to_string())
}

/// 设置系统托盘图标和菜单
pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let menu = build_tray_menu(app)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("未找到默认图标")?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip(locale_manager::menu_tooltip())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "new_note" => {
                    handle_new_note(app);
                }
                "show_all" => {
                    handle_show_all(app);
                }
                "hub" => {
                    handle_hub(app);
                }
                "sync_now" => {
                    handle_sync(app);
                }
                "quit" => {
                    crate::USER_QUIT.store(true, std::sync::atomic::Ordering::SeqCst);
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                handle_new_note(app);
            }
        })
        .build(app)
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 重建托盘菜单（语言切换后调用）
pub fn rebuild_tray_menu(app: &AppHandle) -> Result<(), String> {
    let menu = build_tray_menu(app)?;

    let tray = app.tray_by_id("main-tray").ok_or("未找到托盘图标")?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    tray.set_tooltip(Some(locale_manager::menu_tooltip())).map_err(|e| e.to_string())?;

    Ok(())
}

fn handle_new_note(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    // schedule_auto_sync 由 note_service emit NoteWritten(Created) 事件触发（ADR-007）
    if let Err(e) = super::note_service::create_note(
        app,
        state.note_repo.as_ref(),
        state.event_bus.as_ref(),
        None,
    ) {
        eprintln!("[托盘] 新建便签失败: {}", e);
    }
}

fn handle_show_all(app: &AppHandle) {
    if let Err(e) = super::window_manager::restore_all_windows(app) {
        eprintln!("恢复便签窗口失败: {}", e);
    }
}

fn handle_hub(app: &AppHandle) {
    // 委托 window_manager（消除内联 WebviewWindowBuilder 约束违规）
    super::window_manager::open_or_focus_hub(app);
}

fn handle_sync(app: &AppHandle) {
    eprintln!("[同步] 托盘触发同步...");
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_clone.state::<crate::AppState>();
        let _ = super::git_sync::sync_with_notification(
            &app_clone,
            &state.git_sync,
            state.note_repo.as_ref(),
            state.reminder_repo.as_ref(),
            state.template_repo.as_ref(),
            false,
        );
    });
}
