use std::fs;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub fn read_note(app: AppHandle) -> Result<String, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let note_path = app_dir.join("note.md");
    if note_path.exists() {
        fs::read_to_string(&note_path).map_err(|e| e.to_string())
    } else {
        Ok(String::new())
    }
}

#[tauri::command]
pub fn save_note(app: AppHandle, content: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let note_path = app_dir.join("note.md");
    fs::write(&note_path, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn request_quit(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn hide_window(app: AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("窗口不存在")?;
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).map_err(|e| format!("无法创建数据目录: {}", e))?;
    }
    open::that(&app_dir).map_err(|e| format!("无法打开数据目录: {}", e))
}