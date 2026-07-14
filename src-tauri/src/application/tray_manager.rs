use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use super::locale_manager;

/// 设置系统托盘图标和菜单
pub fn setup_tray(app: &AppHandle) -> Result<(), String> {
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

    let menu = Menu::with_items(app, &[&new_note, &show_all, &hub, &separator1, &sync_now, &separator2, &quit])
        .map_err(|e| e.to_string())?;

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

    let menu = Menu::with_items(app, &[&new_note, &show_all, &hub, &separator1, &sync_now, &separator2, &quit])
        .map_err(|e| e.to_string())?;

    let tray = app.tray_by_id("main-tray").ok_or("未找到托盘图标")?;
    tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    tray.set_tooltip(Some(locale_manager::menu_tooltip())).map_err(|e| e.to_string())?;

    Ok(())
}

fn handle_new_note(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    if let Err(e) = super::note_service::create_note(app, state.note_repo.as_ref(), None) {
        eprintln!("[托盘] 新建便签失败: {}", e);
    }
}

fn handle_show_all(app: &AppHandle) {
    if let Err(e) = super::window_manager::restore_all_windows(app) {
        eprintln!("恢复便签窗口失败: {}", e);
    }
}

fn handle_hub(app: &AppHandle) {
    use tauri::WebviewUrl;
    use tauri::WebviewWindowBuilder;

    if let Some(window) = app.get_webview_window("hub") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

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
        eprintln!("[托盘] 打开设置中心失败");
    }
}

fn handle_sync(app: &AppHandle) {
    use tauri_plugin_notification::NotificationExt;
    eprintln!("[同步] 托盘触发同步...");
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_clone.state::<crate::AppState>();
        match super::note_service::sync_notes(state.note_repo.as_ref(), state.reminder_repo.as_ref(), &state.git_sync, false) {
            Ok(msg) => {
                eprintln!("[同步] 完成: {}", msg);
                let _ = app_clone.notification().builder()
                    .title(locale_manager::notify_sync_ok())
                    .body(&msg)
                    .show();
            }
            Err(e) => {
                eprintln!("[同步] 失败: {}", e);
                let _ = app_clone.notification().builder()
                    .title(locale_manager::notify_sync_fail())
                    .body(&e)
                    .show();
            }
        }
    });
}
