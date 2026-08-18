// ═══ 优化方案 2：Rust 命令异步化 ═══
// 改动说明：
// 1. read_note / save_note 改为 async fn，使用 tokio::fs 替代 std::fs
// 2. save_note 的原子写入逻辑（tmp + sync_all + rename）保持不变
// 3. open_data_dir 保持同步（open::that 是系统调用，无需异步）
// 4. request_quit / hide_window 本身无 I/O，无需改动
//
// 收益：文件 I/O 不再阻塞线程池工作线程，启动阶段多个命令可真正并行执行

use tauri::{AppHandle, Manager};
use tokio::io::AsyncWriteExt;

#[tauri::command]
pub async fn read_note(app: AppHandle) -> Result<String, String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let note_path = app_dir.join("note.json");
    // note.json 存在 → 直接读
    if note_path.exists() {
        tokio::fs::read_to_string(&note_path)
            .await
            .map_err(|e| e.to_string())
    } else {
        // 兜底：旧版 note.md（迁移期不删旧文件）
        let legacy_path = app_dir.join("note.md");
        if legacy_path.exists() {
            tokio::fs::read_to_string(&legacy_path)
                .await
                .map_err(|e| e.to_string())
        } else {
            Ok(String::new())
        }
    }
}

#[tauri::command]
pub async fn save_note(app: AppHandle, content: String) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let note_path = app_dir.join("note.json");
    let tmp_path = app_dir.join(".note.json.tmp");

    // 原子写入：先写临时文件，成功后 rename 覆盖原文件
    // 使用 tokio::fs 异步 I/O，不阻塞线程池线程
    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| format!("创建临时文件失败: {}", e))?;

    file.write_all(content.as_bytes())
        .await
        .map_err(|e| format!("写入临时文件失败: {}", e))?;

    // sync_all 确保数据落盘（异步版本）
    file.sync_all()
        .await
        .map_err(|e| format!("同步临时文件失败: {}", e))?;

    drop(file);

    tokio::fs::rename(&tmp_path, &note_path)
        .await
        .map_err(|e| {
            // 错误路径用同步清理（闭包内无法 .await，且此分支极罕见）
            let _ = std::fs::remove_file(&tmp_path);
            format!("重命名临时文件失败: {}", e)
        })
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

// open_data_dir 保持同步 — open::that 是单次系统调用，开销极低
#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir)
            .map_err(|e| format!("无法创建数据目录: {}", e))?;
    }
    open::that(&app_dir).map_err(|e| format!("无法打开数据目录: {}", e))
}
