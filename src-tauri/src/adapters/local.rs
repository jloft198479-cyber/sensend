use super::{markdown, PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub struct LocalAdapter;

impl LocalAdapter {
    pub fn new() -> Self { Self }

    /// 从 TipTap JSON 提取文件名安全的标题
    fn safe_title(&self, content: &Value) -> String {
        let title = markdown::extract_title(content);
        title
            .chars()
            .map(|c| if r#"\/:*?\"<>|"#.contains(c) { '_' } else { c })
            .collect()
    }
}

#[async_trait]
impl PlatformAdapter for LocalAdapter {
    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String> {
        let folder = PathBuf::from(&instance.target_id);
        if !folder.exists() {
            return Err(format!("文件夹不存在: {}", instance.target_id));
        }
        if !folder.is_dir() {
            return Err(format!("路径不是文件夹: {}", instance.target_id));
        }
        // 尝试写入测试文件
        let test_file = folder.join(".sensend_test");
        match fs::write(&test_file, "ok") {
            Ok(()) => {
                let _ = fs::remove_file(&test_file);
                Ok(())
            }
            Err(e) => Err(format!("文件夹无写入权限: {}", e)),
        }
    }

    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        let folder = PathBuf::from(&instance.target_id);
        if !folder.is_dir() {
            return Err(format!("目标文件夹不存在: {}", instance.target_id));
        }

        let title = self.safe_title(content);
        let md_text = markdown::tiptap_to_markdown(content);

        // 生成文件名：标题 + 时间戳（避免同名覆盖）
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.md", title, timestamp);
        let filepath = folder.join(&filename);

        // 写入文件（UTF-8 BOM 让 Windows 记事本友好）
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend(md_text.as_bytes());

        fs::write(&filepath, &data).map_err(|e| format!("写入失败: {}", e))?;

        Ok(PublishResult {
            success: true,
            message: format!("已保存: {}", filename),
            url: Some(filepath.to_string_lossy().to_string()),
        })
    }
}