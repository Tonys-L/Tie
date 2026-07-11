use std::sync::atomic::{AtomicU8, Ordering};

/// 0 = zh, 1 = en
static LOCALE: AtomicU8 = AtomicU8::new(0);

pub fn get_locale_code() -> u8 {
    LOCALE.load(Ordering::SeqCst)
}

pub fn set_locale_code(code: u8) {
    LOCALE.store(code.min(1), Ordering::SeqCst);
}

/// 获取托盘菜单文本
macro_rules! t {
    ($zh:expr, $en:expr) => {
        if get_locale_code() == 0 { $zh } else { $en }
    };
}

pub fn menu_new_note() -> &'static str { t!("新建便签", "New Note") }
pub fn menu_show_all() -> &'static str { t!("显示全部便签", "Show All Notes") }
pub fn menu_hub() -> &'static str { t!("设置中心", "Settings") }
pub fn menu_sync_now() -> &'static str { t!("立即同步", "Sync Now") }
pub fn menu_quit() -> &'static str { t!("退出", "Quit") }
pub fn menu_tooltip() -> &'static str { t!("Tie", "Tie") }
pub fn menu_hub_title() -> &'static str { t!("设置中心", "Settings") }
pub fn notify_sync_ok() -> &'static str { t!("同步成功", "Sync Complete") }
pub fn notify_sync_fail() -> &'static str { t!("同步失败", "Sync Failed") }
