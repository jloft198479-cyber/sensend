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
use std::sync::atomic::{AtomicU64, Ordering};

/// [n-01] 保存串行锁。
/// 前端虽有 inFlightSave 守卫，但那只管「同一个编辑器实例」；命令层是全局的
/// （重试、退出前 flush、未来多窗口都会进来），两个 save 并发时旧实现共用
/// `.note.json.tmp` 这一个固定路径：A 正写一半，B 创建同名文件把 A 的内容截断，
/// A 再 rename → 落盘的是残缺内容。锁在这里，不依赖调用方自觉。
static SAVE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// tmp 文件序号：即便某种情况下绕过了锁（例如进程外的第二个实例），
/// 也不会再撞同一个临时文件名。
static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

/// 生成一次性的临时文件名（唯一真源：格式只在这里定义）
fn next_tmp_name() -> String {
    format!(
        ".note.json.tmp.{}.{}",
        std::process::id(),
        TMP_SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

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
    // 整个「建 tmp → 写 → sync → rename」必须在同一把锁里完成，
    // 否则后进的写会在中途把 tmp 覆盖掉
    let _guard = SAVE_LOCK.lock().await;

    let app_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let note_path = app_dir.join("note.json");
    // 带 pid + 序号：防跨进程/绕锁时撞名
    let tmp_path = app_dir.join(next_tmp_name());

    // 原子写入：先写临时文件，成功后 rename 覆盖原文件
    // 使用 tokio::fs 异步 I/O，不阻塞线程池线程
    let written: Result<(), String> = async {
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
            .map_err(|e| format!("重命名临时文件失败: {}", e))?;
        Ok(())
    }
    .await;

    if written.is_err() {
        // 失败必须清掉半成品：tmp 名现在带 pid+序号（不再复用同名覆盖），
        // 残留一份就永久躺在用户数据目录里，而且里面是**不完整的笔记正文**。
        let _ = tokio::fs::remove_file(&tmp_path).await;
    }
    written
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
    // 隐藏 10 秒后自动修剪进程树工作集（隐藏即瘦身）
    crate::memory_trim::on_window_hidden();
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

/// [m-08] 打开发送结果里的本地文件（本地平台发送成功后返回的是磁盘路径，
/// 前端 openUrl 只认 URL，传路径会被 opener 的 scope 拒掉 → 点「查看」没反应）。
/// 只接受**绝对路径且必须存在**：路径来自本应用后端，这条校验只是防止误传
/// （相对路径会相对进程工作目录解析，落到哪个目录完全不可预期）。
#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let p = validate_open_path(&path)?;
    open::that(p).map_err(|e| format!("无法打开：{}", e))
}

/// 打开前的路径校验（与真正的 open::that 解耦，便于单测）
fn validate_open_path(path: &str) -> Result<std::path::PathBuf, String> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err("只能打开绝对路径".to_string());
    }
    if !p.exists() {
        return Err(format!("路径不存在：{}", path));
    }
    Ok(p.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [n-01] 两次保存必须拿到不同的临时文件名（固定名并发写会互相截断）
    #[test]
    fn tmp_names_are_unique() {
        let a = next_tmp_name();
        let b = next_tmp_name();
        assert_ne!(a, b, "两次生成同名 tmp 会把并发写互相覆盖");
        assert!(a.starts_with(".note.json.tmp."), "tmp 前缀约定被改动: {}", a);
        assert!(
            a.contains(&std::process::id().to_string()),
            "tmp 名必须含 pid，否则跨进程会撞名: {}",
            a
        );
    }

    /// [n-01] 锁存在且能串行化——只能验证语义（同一 task 内可重入获取）
    #[tokio::test]
    async fn save_lock_serializes() {
        let g1 = SAVE_LOCK.lock().await;
        // try_lock 在持有期间必须失败，证明是同一把锁
        assert!(SAVE_LOCK.try_lock().is_err(), "SAVE_LOCK 未能串行化");
        drop(g1);
        assert!(SAVE_LOCK.try_lock().is_ok(), "释放后应可再次获取");
    }

    /// [m-08] 相对路径一律拒绝
    #[test]
    fn open_path_rejects_relative() {
        assert!(validate_open_path("note.json").is_err());
        assert!(validate_open_path(".\\note.json").is_err());
        assert!(validate_open_path("").is_err());
    }

    /// [m-08] 不存在的绝对路径要报错（而不是让系统弹一个「找不到文件」）
    #[test]
    fn open_path_rejects_missing() {
        let missing = std::env::temp_dir().join("sensend-definitely-not-here-987654.md");
        assert!(validate_open_path(&missing.to_string_lossy()).is_err());
    }

    /// [m-08] 存在的绝对路径通过校验
    #[test]
    fn open_path_accepts_existing() {
        let f = std::env::temp_dir().join("sensend-open-path-test.md");
        std::fs::write(&f, "x").unwrap();
        let ok = validate_open_path(&f.to_string_lossy());
        let _ = std::fs::remove_file(&f);
        assert!(ok.is_ok(), "存在的绝对路径不应被拒: {:?}", ok.err());
    }
}
