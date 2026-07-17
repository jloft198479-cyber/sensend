// ═══ 优化方案 2：Rust 命令异步化 ═══
// 改动说明：
// 1. scan_user_fonts 改为 async fn，使用 tokio::fs 异步 I/O
// 2. delete_user_font 同步改异步
// 3. open_fonts_dir 保持同步（open::that 是单次系统调用）
// 4. strip_font_weight_suffix 逻辑不变
//
// 收益：字体目录扫描不再阻塞线程池，启动阶段可与其他命令并行

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
pub async fn scan_user_fonts(app: AppHandle) -> Result<Vec<UserFont>, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let fonts_dir = app_dir.join("fonts");
    if !fonts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut fonts: Vec<UserFont> = Vec::new();

    // 使用 tokio::fs 异步读取目录
    let mut dir = tokio::fs::read_dir(&fonts_dir)
        .await
        .map_err(|e| format!("无法读取字体目录: {}", e))?;

    while let Some(entry) = dir.next_entry().await.map_err(|e| format!("读取字体条目失败: {}", e))? {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !FONT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let display_name = strip_font_weight_suffix(stem);

        if !display_name.is_empty() && !fonts.iter().any(|f| f.name == display_name) {
            let file_path = path.to_string_lossy().replace("\\", "/");
            // Windows 盘符冒号 C: 需编码为 C%3A，否则 Tauri asset 协议会解析失败
            let encoded_path = if cfg!(windows) {
                file_path.replace(":", "%3A")
            } else {
                file_path
            };
            fonts.push(UserFont {
                name: display_name.to_string(),
                path: format!("https://asset.localhost/{}", encoded_path),
            });
        }
    }

    fonts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(fonts)
}

// open_fonts_dir 保持同步 — 单次系统调用
#[tauri::command]
pub fn open_fonts_dir(app: AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let fonts_dir = app_dir.join("fonts");
    if !fonts_dir.exists() {
        std::fs::create_dir_all(&fonts_dir)
            .map_err(|e| format!("无法创建字体目录: {}", e))?;
    }
    open::that(&fonts_dir).map_err(|e| format!("无法打开字体目录: {}", e))
}

#[tauri::command]
pub async fn delete_user_font(app: AppHandle, font_name: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let fonts_dir = app_dir.join("fonts");
    if !fonts_dir.exists() {
        return Err("字体目录不存在".into());
    }

    let mut deleted = false;

    let mut dir = tokio::fs::read_dir(&fonts_dir)
        .await
        .map_err(|e| format!("无法读取字体目录: {}", e))?;

    while let Some(entry) = dir.next_entry().await.map_err(|e| format!("读取字体条目失败: {}", e))? {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !FONT_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let display_name = strip_font_weight_suffix(stem);

        if display_name == font_name {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| format!("删除字体文件失败: {}", e))?;
            deleted = true;
        }
    }

    if !deleted {
        return Err(format!("未找到字体: {}", font_name));
    }
    Ok(())
}
