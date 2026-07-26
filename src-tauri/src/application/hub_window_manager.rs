//! Hub 窗口管理：Hub（设置中心）窗口的创建、切换、聚焦（从 window_manager 拆出）
//!
//! 职责：
//! - `toggle_hub_window`：切换 Hub 窗口可见性（已显示则隐藏，隐藏或未创建则显示）
//! - `open_or_focus_hub`：打开或聚焦 Hub 窗口（始终显示，托盘菜单调用）
//! - `create_hub_window`：创建 Hub 窗口（统一入口，消除重复）
//!
//! 调用方：
//! - `tray_manager`：托盘菜单点击调用 `open_or_focus_hub`
//! - `shortcut_manager`：快捷键调用 `toggle_hub_window`
//!
//! 依赖：
//! - `tauri::AppHandle`
//! - `application::locale_manager`（Hub 窗口标题本地化）
//!
//! 设计要点：
//! - Hub 窗口与 note 窗口是不同生命周期，独立 module 隔离关注点
//! - 与 `window_manager`（note 窗口生命周期）单向依赖无环

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

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
        .title(locale_manager::MENU_HUB_TITLE.get())
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
