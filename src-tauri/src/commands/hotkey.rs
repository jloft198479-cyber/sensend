use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use serde_json::Value;

fn get_hotkey_config(app: &AppHandle) -> (String, String) {
    let store = app.store("config.json").ok();
    let show_hotkey = store
        .as_ref()
        .and_then(|s| s.get("hotkey_show"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "Alt+Shift+F".to_string());
    let send_hotkey = store
        .as_ref()
        .and_then(|s| s.get("hotkey_send"))
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "Control+Enter".to_string());
    (show_hotkey, send_hotkey)
}

pub fn register_show_hotkey(app: &AppHandle, hotkey_str: &str) -> Result<(), String> {
    let shortcut: Shortcut = hotkey_str
        .parse::<Shortcut>()
        .map_err(|e| e.to_string())?;
    app.global_shortcut().on_shortcuts(vec![shortcut], |app_handle, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            if let Some(window) = app_handle.get_webview_window("main") {
                if window.is_visible().unwrap_or(false) {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
    }).map_err(|e| e.to_string())
}

fn unregister_all_hotkeys(app: &AppHandle) -> Result<(), String> {
    app.global_shortcut().unregister_all().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_hotkeys(app: AppHandle) -> Result<Value, String> {
    let (show_hotkey, send_hotkey) = get_hotkey_config(&app);
    Ok(serde_json::json!({
        "show": show_hotkey,
        "send": send_hotkey,
    }))
}

#[tauri::command]
pub async fn save_hotkeys(app: AppHandle, show: String, send: String) -> Result<(), String> {
    let _: Shortcut = show.parse().map_err(|e| {
        format!("唤醒快捷键格式无效: {}", e)
    })?;

    unregister_all_hotkeys(&app)?;
    register_show_hotkey(&app, &show)?;

    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set("hotkey_show", serde_json::Value::String(show));
    store.set("hotkey_send", serde_json::Value::String(send));
    store.save().map_err(|e| e.to_string())?;

    Ok(())
}

/// setup 阶段调用，返回初始配置供调用方使用
pub fn init_hotkeys(app: &AppHandle) -> (String, String) {
    let (show_hotkey, send_hotkey) = get_hotkey_config(app);
    if let Err(e) = register_show_hotkey(app, &show_hotkey) {
        eprintln!("注册全局快捷键失败: {}", e);
    }
    (show_hotkey, send_hotkey)
}