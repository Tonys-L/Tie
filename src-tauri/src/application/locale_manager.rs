//! 托盘菜单 + 通知文本的 i18n 常量表。
//!
//! 职责：
//! - `LOCALE` 全局语言开关（0=zh, 1=en），运行时 AtomicU8
//! - `LocaleText` 双语文本结构体（zh + en），`.get()` 按当前 LOCALE 返回对应文本
//! - 9 个常量：MENU_NEW_NOTE / MENU_SHOW_ALL / MENU_HUB / MENU_SYNC_NOW / MENU_QUIT /
//!   MENU_TOOLTIP / MENU_HUB_TITLE / NOTIFY_SYNC_OK / NOTIFY_SYNC_FAIL
//!
//! 调用方：
//! - `tray_manager`：托盘菜单文本 + tooltip
//! - `hub_window_manager`：Hub 窗口标题
//! - `git_sync`：同步通知标题
//!
//! 设计要点：
//! - 用常量表替代 9 个浅包装函数（`pub fn menu_xxx() -> &'static str { t!(...) }`），
//!   减少样板代码，调用方 `MENU_NEW_NOTE.get()` vs `menu_new_note()` 语义更清晰
//! - `get_locale_code` / `set_locale_code` 保留 pub 供 `locale_commands::set_locale` 调用

use std::sync::atomic::{AtomicU8, Ordering};

/// 0 = zh, 1 = en
static LOCALE: AtomicU8 = AtomicU8::new(0);

pub fn get_locale_code() -> u8 {
    LOCALE.load(Ordering::SeqCst)
}

pub fn set_locale_code(code: u8) {
    LOCALE.store(code.min(1), Ordering::SeqCst);
}

/// 双语文本结构体（zh + en），`.get()` 按当前 LOCALE 返回对应文本
pub struct LocaleText {
    pub zh: &'static str,
    pub en: &'static str,
}

impl LocaleText {
    /// 按当前 LOCALE 全局开关返回对应文本
    pub fn get(&self) -> &'static str {
        if get_locale_code() == 0 { self.zh } else { self.en }
    }
}

// ============ 托盘菜单文本常量 ============

pub const MENU_NEW_NOTE: LocaleText = LocaleText { zh: "新建便签", en: "New Note" };
pub const MENU_SHOW_ALL: LocaleText = LocaleText { zh: "显示全部便签", en: "Show All Notes" };
pub const MENU_HUB: LocaleText = LocaleText { zh: "设置中心", en: "Settings" };
pub const MENU_SYNC_NOW: LocaleText = LocaleText { zh: "立即同步", en: "Sync Now" };
pub const MENU_QUIT: LocaleText = LocaleText { zh: "退出", en: "Quit" };
pub const MENU_TOOLTIP: LocaleText = LocaleText { zh: "Tie", en: "Tie" };
pub const MENU_HUB_TITLE: LocaleText = LocaleText { zh: "设置中心", en: "Settings" };

// ============ 同步通知文本常量 ============

pub const NOTIFY_SYNC_OK: LocaleText = LocaleText { zh: "同步成功", en: "Sync Complete" };
pub const NOTIFY_SYNC_FAIL: LocaleText = LocaleText { zh: "同步失败", en: "Sync Failed" };

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_locale(zh: bool) {
        set_locale_code(if zh { 0 } else { 1 });
    }

    #[test]
    fn test_locale_text_get_zh_when_locale_zero() {
        reset_locale(true);
        assert_eq!(MENU_NEW_NOTE.get(), "新建便签");
        assert_eq!(MENU_SHOW_ALL.get(), "显示全部便签");
        assert_eq!(MENU_HUB.get(), "设置中心");
        assert_eq!(MENU_SYNC_NOW.get(), "立即同步");
        assert_eq!(MENU_QUIT.get(), "退出");
        assert_eq!(MENU_TOOLTIP.get(), "Tie");
        assert_eq!(MENU_HUB_TITLE.get(), "设置中心");
        assert_eq!(NOTIFY_SYNC_OK.get(), "同步成功");
        assert_eq!(NOTIFY_SYNC_FAIL.get(), "同步失败");
    }

    #[test]
    fn test_locale_text_get_en_when_locale_one() {
        reset_locale(false);
        assert_eq!(MENU_NEW_NOTE.get(), "New Note");
        assert_eq!(MENU_SHOW_ALL.get(), "Show All Notes");
        assert_eq!(MENU_HUB.get(), "Settings");
        assert_eq!(MENU_SYNC_NOW.get(), "Sync Now");
        assert_eq!(MENU_QUIT.get(), "Quit");
        assert_eq!(MENU_TOOLTIP.get(), "Tie");
        assert_eq!(MENU_HUB_TITLE.get(), "Settings");
        assert_eq!(NOTIFY_SYNC_OK.get(), "Sync Complete");
        assert_eq!(NOTIFY_SYNC_FAIL.get(), "Sync Failed");
    }

    #[test]
    fn test_set_locale_code_clamps_to_one() {
        set_locale_code(99);
        assert_eq!(get_locale_code(), 1);
        reset_locale(true);
    }
}
