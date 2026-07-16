mod application;
mod domain;
mod infrastructure;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use application::{commands, reminder_scheduler, shortcut_manager, tray_manager};
use infrastructure::{Database, SqliteNoteRepository, SqliteReminderRepository};
use tauri::Manager;

/// 用户主动退出标志（区别于窗口全部关闭导致的退出）
pub static USER_QUIT: AtomicBool = AtomicBool::new(false);

/// 应用全局状态
///
/// 在 setup 中创建，通过 Tauri State 管理器注入到各命令。
/// 仓储通过 trait object 持有，遵循依赖倒置原则，支持未来替换实现。
pub struct AppState {
    pub note_repo: Box<dyn domain::NoteRepository>,
    pub reminder_repo: Box<dyn domain::ReminderRepository>,
    pub git_sync: application::git_sync::GitSync,
    pub shortcut_manager: application::shortcut_manager::ShortcutManager,
    pub scheduler: application::reminder_scheduler::ReminderScheduler,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { .. } => {
                    let label = window.label();
                    if let Some(note_id) = label.strip_prefix("note-") {
                        let app = window.app_handle();
                        let state = app.state::<crate::AppState>();
                        crate::application::note_service::close_note_if_empty(state.note_repo.as_ref(), note_id);
                    }
                }
                tauri::WindowEvent::Destroyed => {}
                _ => {}
            }
        })
        .setup(|app| {
            eprintln!("[setup] 开始初始化...");

            // ---- 初始化数据库 ----
            // 统一使用 exe 同级目录的 data 文件夹（避开沙箱限制）
            let db_dir = std::env::current_exe()
                .map_err(|e| format!("获取 exe 路径失败: {}", e))?
                .parent()
                .ok_or("无法获取父目录")?
                .join("data");
            std::fs::create_dir_all(&db_dir)
                .map_err(|e| format!("创建目录失败 {:?}: {}", db_dir, e))?;
            eprintln!("[setup] 数据库目录: {:?}", db_dir);

            // 设置 WebView2 用户数据目录，避免默认目录被沙箱限制
            let webview_data_dir = db_dir.join("webview");
            std::fs::create_dir_all(&webview_data_dir)
                .map_err(|e| format!("创建 WebView 数据目录失败: {}", e))?;
            std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &webview_data_dir);
            eprintln!("[setup] WebView2 数据目录: {:?}", webview_data_dir);
            let db_path = db_dir.join("notes.db");
            let db = Arc::new(Database::new(
                db_path.to_str().ok_or("路径转换失败")?,
            ).map_err(|e| format!("初始化数据库失败 {:?}: {}", db_path, e))?);
            eprintln!("[setup] 数据库初始化成功");

            let note_repo = Box::new(SqliteNoteRepository::new(db.clone()));
            let reminder_repo = Box::new(SqliteReminderRepository::new(db));
            let git_sync = application::git_sync::GitSync::new(&db_dir);
            let shortcut_mgr = application::shortcut_manager::ShortcutManager::new(&db_dir);
            let scheduler = application::reminder_scheduler::ReminderScheduler::new();

            app.manage(AppState {
                note_repo,
                reminder_repo,
                git_sync,
                shortcut_manager: shortcut_mgr,
                scheduler,
            });
            eprintln!("[setup] AppState 已注册");

            // ---- 系统托盘 ----
            tray_manager::setup_tray(app.handle())
                .map_err(|e| format!("设置系统托盘失败: {}", e))?;
            eprintln!("[setup] 系统托盘设置成功");

            // ---- 全局快捷键 ----
            shortcut_manager::setup_shortcuts(app.handle())
                .map_err(|e| format!("注册全局快捷键失败: {}", e))?;
            eprintln!("[setup] 全局快捷键注册成功");

            // ---- 提醒调度器 ----
            reminder_scheduler::start(app.handle().clone());
            eprintln!("[setup] 提醒调度器已启动");

            // ---- 自动同步：启动时拉取 ----
            {
                let state = app.state::<AppState>();
                state.git_sync.auto_pull_on_startup(state.note_repo.as_ref(), state.reminder_repo.as_ref());
            }

            // ---- 恢复所有便签窗口 ----
            match application::window_manager::restore_all_windows(app.handle()) {
                Ok(count) => eprintln!("[setup] 恢复了 {} 张便签", count),
                Err(e) => eprintln!("[setup] 恢复便签失败: {}", e),
            }

            eprintln!("[setup] 初始化完成!");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_note,
            commands::activate_note_by_id,
            commands::get_note,
            commands::get_all_notes,
            commands::open_note,
            commands::open_note_with_flag,
            commands::update_note_content,
            commands::update_note_title,
            commands::update_note_style,
            commands::update_note_window_state,
            commands::delete_note,
            commands::archive_note,
            commands::unarchive_note,
            commands::get_archived_notes,
            commands::search_notes,
            commands::update_note_tags,
            commands::create_reminder,
            commands::get_reminders,
            commands::snooze_reminder,
            commands::dismiss_reminder,
            commands::delete_reminder,
            commands::get_reminders_by_month,
            commands::get_lunar_dates,
            commands::get_notes_activity_by_month,
            commands::get_sync_config,
            commands::save_sync_config,
            commands::sync_notes,
            commands::check_git,
            commands::get_shortcut_config,
            commands::save_shortcut_config,
            commands::set_locale,
            commands::get_data_dir,
            commands::open_data_dir,
            commands::open_url,
            commands::save_image,
            commands::get_image_dir,
        ])
        .build(tauri::generate_context!())
        .expect("启动应用失败");

    // 阻止应用在所有窗口关闭时退出（托盘常驻应用）
    // 但允许用户主动退出（托盘"退出"菜单）
    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if USER_QUIT.load(Ordering::SeqCst) {
                eprintln!("[应用] 用户主动退出，允许退出");
            } else {
                eprintln!("[应用] 窗口关闭导致退出，阻止（保持托盘常驻）");
                api.prevent_exit();
            }
        }
    });
}
