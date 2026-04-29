use std::fs;
use serde::Serialize;
use tauri::{AppHandle, Manager};

const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "woff2", "ttc"];

#[derive(Serialize, Clone)]
pub struct UserFont {
    name: String,
    path: String,
}

fn strip_font_weight_suffix(stem: &str) -> &str {
    stem.trim_end_matches("-Regular")
        .trim_end_matches("-Bold")
        .trim_end_matches("-Italic")
        .trim_end_matches("-Light")
        .trim_end_matches("-Medium")
        .trim_end_matches("-Semibold")
        .trim_end_matches("-Thin")
        .trim_end_matches("-Black")
        .trim_end_matches("-ExtraBold")
        .trim_end_matches("-ExtraLight")
}

#[tauri::command]
pub fn scan_user_fonts(app: AppHandle) -> Result<Vec<UserFont>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let fonts_dir = app_dir.join("fonts");
    if !fonts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut fonts: Vec<UserFont> = Vec::new();

    for entry in fs::read_dir(&fonts_dir).map_err(|e| format!("无法读取字体目录: {}", e))? {
        let entry = entry.map_err(|e| format!("读取字体条目失败: {}", e))?;
        let path = entry.path();

        if !path.is_file() { continue; }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !FONT_EXTENSIONS.contains(&ext.as_str()) { continue; }

        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let display_name = strip_font_weight_suffix(stem);

        if !display_name.is_empty() && !fonts.iter().any(|f| f.name == display_name) {
            let file_path = path.to_string_lossy().replace("\\", "/");
            fonts.push(UserFont {
                name: display_name.to_string(),
                path: format!("https://asset.localhost/{}", file_path),
            });
        }
    }

    fonts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fonts)
}

#[tauri::command]
pub fn open_fonts_dir(app: AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let fonts_dir = app_dir.join("fonts");
    if !fonts_dir.exists() {
        fs::create_dir_all(&fonts_dir).map_err(|e| format!("无法创建字体目录: {}", e))?;
    }
    open::that(&fonts_dir).map_err(|e| format!("无法打开字体目录: {}", e))
}

#[tauri::command]
pub fn delete_user_font(app: AppHandle, font_name: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let fonts_dir = app_dir.join("fonts");
    if !fonts_dir.exists() {
        return Err("字体目录不存在".into());
    }

    let mut deleted = false;

    for entry in fs::read_dir(&fonts_dir).map_err(|e| format!("无法读取字体目录: {}", e))? {
        let entry = entry.map_err(|e| format!("读取字体条目失败: {}", e))?;
        let path = entry.path();

        if !path.is_file() { continue; }

        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !FONT_EXTENSIONS.contains(&ext.as_str()) { continue; }

        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let display_name = strip_font_weight_suffix(stem);

        if display_name == font_name {
            fs::remove_file(&path).map_err(|e| format!("删除字体文件失败: {}", e))?;
            deleted = true;
        }
    }

    if !deleted {
        return Err(format!("未找到字体: {}", font_name));
    }
    Ok(())
}