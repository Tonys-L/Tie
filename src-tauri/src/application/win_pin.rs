//! Win32 窗口 pin 模块：使置顶窗口免疫 Win+D（显示桌面）。
//!
//! 使用 SetWinEventHook 监听最小化事件，检测到后立即恢复窗口。
//! 使用 SW_SHOWNOACTIVATE 无动画恢复，避免闪烁。

#![cfg(target_os = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::*;

/// 全局存储：被 pin 的窗口 HWND 集合 + 停止信号
struct PinState {
    hwnds: std::collections::HashSet<isize>,
    stop: Arc<AtomicBool>,
    hook_thread: Option<std::thread::JoinHandle<()>>,
}

static PIN_STATE: std::sync::Mutex<Option<PinState>> = std::sync::Mutex::new(None);

/// HWND_TOPMOST 常量
fn hwnd_topmost() -> HWND {
    HWND(-1isize as *mut core::ffi::c_void)
}

/// WinEvent 回调 — 在 hook 线程的消息循环中调用
unsafe extern "system" fn winevent_callback(
    _h_win_event_hook: HWINEVENTHOOK,
    _event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if hwnd.0.is_null() {
        return;
    }

    let key = hwnd.0 as isize;

    // 快速检查是否是被 pin 的窗口（不持锁调用 Win32 API）
    let should_restore = {
        let state = PIN_STATE.lock().unwrap();
        state.as_ref().map_or(false, |s| s.hwnds.contains(&key))
    };

    if should_restore {
        unsafe {
            // SW_SHOWNOACTIVATE: 无动画恢复窗口，避免闪烁
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            // 重新置顶（不激活、不移动、不改变大小）
            let _ = SetWindowPos(
                hwnd,
                Some(hwnd_topmost()),
                0, 0, 0, 0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

/// 确保 hook 线程正在运行
fn ensure_hook_thread(state: &mut Option<PinState>) {
    if state.is_some() {
        return;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();

    let handle = std::thread::Builder::new()
        .name("tie-winevent-hook".into())
        .spawn(move || {
            unsafe {
                // 注册 WinEvent hook：监听最小化开始事件
                let hook = SetWinEventHook(
                    EVENT_SYSTEM_MINIMIZESTART,
                    EVENT_SYSTEM_MINIMIZESTART,
                    None,
                    Some(winevent_callback),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                );

                if hook.is_invalid() {
                    eprintln!("[pin] SetWinEventHook 失败");
                    return;
                }

                // 消息循环 — WINEVENT_OUTOFCONTEXT 需要消息循环来接收回调
                let mut msg = MSG::default();
                while !stop_clone.load(Ordering::Relaxed) {
                    // PeekMessage 不阻塞，超时后 sleep 避免空转
                    if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    } else {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }

                // 清理 hook
                let _ = UnhookWinEvent(hook);
            }
        })
        .expect("启动 winevent hook 线程");

    *state = Some(PinState {
        hwnds: std::collections::HashSet::new(),
        stop,
        hook_thread: Some(handle),
    });
}

/// Pin 一个窗口：使其免疫 Win+D
pub fn pin_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|e| format!("获取窗口句柄失败: {}", e))?;

    let key = hwnd.0 as isize;

    let mut state = PIN_STATE.lock().unwrap();
    ensure_hook_thread(&mut state);

    if let Some(ref mut s) = *state {
        s.hwnds.insert(key);
    }

    Ok(())
}

/// Unpin 一个窗口：停止监听
pub fn unpin_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|e| format!("获取窗口句柄失败: {}", e))?;

    let key = hwnd.0 as isize;

    let mut state = PIN_STATE.lock().unwrap();
    if let Some(ref mut s) = *state {
        s.hwnds.remove(&key);
        // 如果没有 pin 窗口了，停止 hook 线程
        if s.hwnds.is_empty() {
            s.stop.store(true, Ordering::Relaxed);
            let handle = s.hook_thread.take();
            *state = None;
            drop(state);
            if let Some(h) = handle {
                let _ = h.join();
            }
            return Ok(());
        }
    }

    Ok(())
}
