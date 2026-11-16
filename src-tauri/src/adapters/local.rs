use super::{markdown, PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::Value;
use tokio::fs;
use std::path::PathBuf;

pub struct LocalAdapter;

impl LocalAdapter {
    pub fn new() -> Self { Self }

    /// 从 TipTap JSON 提取文件名安全的标题（清洗规则见 sanitize_title）
    fn safe_title(&self, content: &Value) -> String {
        sanitize_title(&markdown::extract_title(content))
    }
}

/// Windows 文件名标题清洗：
/// 1) 非法字符 \\/:*?\"<>| 与控制字符 → '_'；2) 去首尾空白；3) 空标题兜底 '_'；
/// 4) Windows 保留设备名（con/nul/com1…即使带扩展名也可能创建失败）→ 前缀 '_'。
/// 标题已被 extract_title 截到前 18 字，无需另设长度上限
fn sanitize_title(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| if r#"\/:*?\"<>|"#.contains(c) || c.is_control() { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() {
        return "_".to_string();
    }
    let stem = cleaned.split('.').next().unwrap_or("").to_lowercase();
    const RESERVED: [&str; 22] = [
        "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7",
        "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        format!("_{cleaned}")
    } else {
        cleaned
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
        match fs::write(&test_file, "ok").await {
            Ok(()) => {
                let _ = fs::remove_file(&test_file).await;
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

        fs::write(&filepath, &data).await.map_err(|e| format!("写入失败: {}", e))?;

        Ok(PublishResult {
            success: true,
            message: format!("已保存: {}", filename),
            url: Some(filepath.to_string_lossy().to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_title;

    #[test]
    fn sanitize_title_replaces_illegal_and_control_chars() {
        assert_eq!(sanitize_title("a/b\\c:d*e?f\"g<h>i|j\u{7}"), "a_b_c_d_e_f_g_h_i_j_");
    }

    #[test]
    fn sanitize_title_prefixes_reserved_names() {
        assert_eq!(sanitize_title("con"), "_con");
        assert_eq!(sanitize_title("NUL.backup"), "_NUL.backup");
        // 保留名按「第一个 . 前的整段主干」精确比对：带后缀/空格不算保留名
        assert_eq!(sanitize_title("com1.md"), "_com1.md");
        assert_eq!(sanitize_title("com1 报告"), "com1 报告");
    }

    #[test]
    fn sanitize_title_keeps_normal_names() {
        assert_eq!(sanitize_title(" 会议纪要 "), "会议纪要");
        assert_eq!(sanitize_title("2026.note"), "2026.note");
        assert_eq!(sanitize_title(""), "_");
    }
}