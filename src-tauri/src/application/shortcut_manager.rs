use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// 快捷键配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConfig {
    pub new_note: String,
    pub show_all: String,
    pub toggle_hub: String,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            new_note: "ctrl+shift+n".to_string(),
            show_all: "ctrl+shift+s".to_string(),
            toggle_hub: "ctrl+shift+h".to_string(),
        }
    }
}

impl ShortcutConfig {
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(ShortcutConfig::default());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("读取快捷键配置失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析快捷键配置失败: {}", e))
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化快捷键配置失败: {}", e))?;
        std::fs::write(path, content).map_err(|e| format!("写入快捷键配置失败: {}", e))
    }
}

/// 快捷键管理器
pub struct ShortcutManager {
    config_path: PathBuf,
    current_config: Mutex<ShortcutConfig>,
}

impl ShortcutManager {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let config_path = data_dir.join("shortcut_config.json");
        let config = ShortcutConfig::load(&config_path).unwrap_or_default();
        Self {
            config_path,
            current_config: Mutex::new(config),
        }
    }

    pub fn get_config(&self) -> ShortcutConfig {
        self.current_config.lock().unwrap().clone()
    }

    pub fn save_and_reregister(&self, app: &AppHandle, config: ShortcutConfig) -> Result<(), String> {
        let old_config = self.get_config();

        // 先注销旧快捷键
        let gs = app.global_shortcut();
        let _ = gs.unregister(old_config.new_note.as_str());
        let _ = gs.unregister(old_config.show_all.as_str());
        let _ = gs.unregister(old_config.toggle_hub.as_str());

        // 注册新快捷键
        let new_note_key = config.new_note.clone();
        let show_all_key = config.show_all.clone();
        let toggle_hub_key = config.toggle_hub.clone();

        gs.on_shortcut(new_note_key.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let state = app.state::<crate::AppState>();
                if let Err(e) = super::note_service::create_note(app, state.note_repo.as_ref(), None) {
                    eprintln!("新建便签失败: {}", e);
                }
            }
        })
        .map_err(|e| format!("注册快捷键 '{}' 失败: {}", new_note_key, e))?;

        gs.on_shortcut(show_all_key.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                if let Err(e) = super::window_manager::restore_all_windows(app) {
                    eprintln!("恢复便签窗口失败: {}", e);
                }
            }
        })
        .map_err(|e| format!("注册快捷键 '{}' 失败: {}", show_all_key, e))?;

        gs.on_shortcut(toggle_hub_key.as_str(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                super::window_manager::toggle_hub_window(app);
            }
        })
        .map_err(|e| format!("注册快捷键 '{}' 失败: {}", toggle_hub_key, e))?;

        // 注册成功，保存配置
        config.save(&self.config_path)?;
        *self.current_config.lock().unwrap() = config;
        Ok(())
    }
}

/// 启动时注册快捷键（首次注册，无需注销旧快捷键）
pub fn setup_shortcuts(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let config = state.shortcut_manager.get_config();
    let gs = app.global_shortcut();

    let new_note_key = config.new_note.clone();
    let show_all_key = config.show_all.clone();
    let toggle_hub_key = config.toggle_hub.clone();

    gs.on_shortcut(new_note_key.as_str(), move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            let state = app.state::<crate::AppState>();
            if let Err(e) = super::note_service::create_note(app, state.note_repo.as_ref(), None) {
                eprintln!("新建便签失败: {}", e);
            }
        }
    })
    .map_err(|e| format!("注册快捷键 '{}' 失败: {}", new_note_key, e))?;

    gs.on_shortcut(show_all_key.as_str(), move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            if let Err(e) = super::window_manager::restore_all_windows(app) {
                eprintln!("恢复便签窗口失败: {}", e);
            }
        }
    })
    .map_err(|e| format!("注册快捷键 '{}' 失败: {}", show_all_key, e))?;

    gs.on_shortcut(toggle_hub_key.as_str(), move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            super::window_manager::toggle_hub_window(app);
        }
    })
    .map_err(|e| format!("注册快捷键 '{}' 失败: {}", toggle_hub_key, e))?;

    Ok(())
}
