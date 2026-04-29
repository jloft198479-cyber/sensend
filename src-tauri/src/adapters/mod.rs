use serde_json::Value;
use async_trait::async_trait;
use std::sync::OnceLock;

/// 全局 HTTP 客户端，整个应用生命周期复用一个实例（连接池、TLS 会话共享）
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 获取全局 HTTP 客户端引用
pub fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client")
    })
}

fn default_publish_mode() -> String { "page".to_string() }

/// 平台实例配置（用户自定义）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformInstance {
    pub id: String,
    pub name: String,          // 用户自定义命名，如“我的工作 Notion”
    pub platform_type: String, // 平台类型：notion / flowus / lark
    pub token: String,
    #[serde(default)]
    pub token2: String,        // 第二个凭证字段（飞书 App Secret）
    pub target_id: String,
    #[serde(default = "default_publish_mode")]
    pub publish_mode: String,  // "page" | "block"，默认 "page"
}

/// 平台类型元数据（前端 + 后端共用）
pub fn get_platform_types() -> Vec<PlatformTypeInfo> {
    vec![
        PlatformTypeInfo {
            key: "local".into(),
            name: "本地文件夹".into(),
            color: "#2CAF68".into(),
            fields: vec![
                ConfigField { key: "token".into(), label: "（无需填写）".into(), hint: "".into(), secret: true, hidden: true, browse: false, default_value: Some("local".into()), optional: true },
                ConfigField { key: "target_id".into(), label: "文件夹路径".into(), hint: "如 D:\\Notes 或 C:\\Users\\xxx\\Desktop".into(), secret: false, hidden: false, browse: true, default_value: None, optional: false },
            ],
        },
        PlatformTypeInfo {
            key: "notion".into(),
            name: "Notion".into(),
            color: "#2CAF68".into(),
            fields: vec![
                ConfigField { key: "token".into(), label: "Integration Token".into(), hint: "Notion Integration Token".into(), secret: true, hidden: false, browse: false, default_value: None, optional: false },
                ConfigField { key: "target_id".into(), label: "Parent Page".into(), hint: "粘贴页面链接，自动解析 ID".into(), secret: false, hidden: false, browse: false, default_value: None, optional: false },
            ],
        },
        PlatformTypeInfo {
            key: "flowus".into(),
            name: "FlowUs".into(),
            color: "#2CAF68".into(),
            fields: vec![
                ConfigField { key: "token".into(), label: "授权码".into(), hint: "FlowUs MCP 授权码".into(), secret: true, hidden: false, browse: false, default_value: None, optional: false },
                ConfigField { key: "target_id".into(), label: "目标页面".into(), hint: "粘贴页面链接，自动解析 ID".into(), secret: false, hidden: false, browse: false, default_value: None, optional: false },
            ],
        },
        PlatformTypeInfo {
            key: "lark".into(),
            name: "飞书".into(),
            color: "#2CAF68".into(),
            fields: vec![
                ConfigField { key: "token".into(), label: "App ID".into(), hint: "飞书应用的 App ID".into(), secret: false, hidden: false, browse: false, default_value: None, optional: false },
                ConfigField { key: "token2".into(), label: "App Secret".into(), hint: "飞书应用的 App Secret".into(), secret: true, hidden: false, browse: false, default_value: None, optional: false },
                ConfigField { key: "target_id".into(), label: "文档链接".into(), hint: "粘贴飞书文档链接，内容将追加到文档末尾".into(), secret: false, hidden: false, browse: false, default_value: None, optional: false },
            ],
        },
    ]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlatformTypeInfo {
    pub key: String,
    pub name: String,
    pub color: String,
    pub fields: Vec<ConfigField>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    pub hint: String,
    pub secret: bool,
    #[serde(default)]
    pub hidden: bool,          // true → 前端隐藏该字段（如 local 的 token）
    #[serde(default)]
    pub browse: bool,          // true → 前端显示“浏览…”按钮（如 local 的 target_id）
    pub default_value: Option<String>, // 用户未填时自动填充
    #[serde(default)]
    pub optional: bool,        // true → 该字段可以不填
}

/// 发布结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct PublishResult {
    pub success: bool,
    pub message: String,
    pub url: Option<String>,
}

/// 探测结果（目标类型）
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProbeResult {
    pub target_type: String, // "page" | "database" | "bitable"
}

/// 平台适配器统一接口
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String>;
    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String>;

    /// 探测目标类型（默认返回 "page"，各适配器按需覆盖）
    async fn probe_type(&self, _instance: &PlatformInstance) -> Result<String, String> {
        Ok("page".to_string())
    }

    /// 追加内容到已有页面（默认不支持，各适配器按需覆盖）
    async fn append_blocks(&self, _content: &Value, _instance: &PlatformInstance) -> Result<PublishResult, String> {
        Err("该平台不支持追加写入".into())
    }
}

pub mod markdown;
pub mod notion;
pub mod local;
pub mod flowus;
pub mod lark;

/// 从 URL 或纯文本中提取平台 ID
/// 各适配器调用此函数解析，前端不做任何解析
pub fn resolve_target_id(platform_type: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    match platform_type {
        "notion" => resolve_notion_id(trimmed),
        "flowus" => resolve_flowus_id(trimmed),
        "lark" => lark::LarkAdapter::resolve_lark_id(trimmed),
        _ => trimmed.to_string(),
    }
}

/// Notion URL → 纯 ID（32 位 hex，去连字符）
/// 例如: 
///   个人空间: https://www.notion.so/my-page-34a6a7109f4580f4b78df783f9905cd4 → 34a6a7109f4580f4b78df783f9905cd4
///   团队空间: https://www.notion.so/{space-id}/{page-id}-{slug} → {page-id}
fn resolve_notion_id(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let hex = |c: char| c.is_ascii_hexdigit();

    // 从右到左扫描，找最后一个 32 位连续 hex
    let mut i = chars.len();
    while i > 0 {
        i -= 1;
        // 找 hex 结束位置（当前是 hex，下一个不是 hex 或到边界）
        if hex(chars[i]) && (i + 1 >= chars.len() || !hex(chars[i + 1])) {
            // 向左找起始位置
            let mut j = i;
            while j > 0 && hex(chars[j - 1]) {
                j -= 1;
            }
            // 检查长度
            if i - j + 1 >= 32 {
                let segment: String = chars[j..=i].iter().collect();
                // 确认前面是分隔符或开头
                if j == 0 || !hex(chars[j - 1]) {
                    return segment.replace("-", "");
                }
            }
        }
    }

    // UUID 格式（带 - 的版本）
    if raw.len() == 36 && raw.chars().filter(|&c| c == '-').count() == 4 {
        if is_uuid(raw) {
            return raw.replace("-", "");
        }
    }

    // 兜底：原样返回
    raw.to_string()
}

/// FlowUs URL → 页面 ID（取最后一个路径段）
/// 例如: https://flowus.cn/weimabbs/640e8519-4ee1-44f9-97ec-ce3dd9788fb5 → 640e8519-4ee1-44f9-97ec-ce3dd9788fb5
fn resolve_flowus_id(raw: &str) -> String {
    let without_trailing = raw.trim_end_matches('/');
    let segments: Vec<&str> = without_trailing.split('/').filter(|s| !s.is_empty()).collect();
    if let Some(&last) = segments.last() {
        return last.to_string();
    }
    raw.to_string()
}

/// 检查是否为 UUID 格式（8-4-4-4-12）
fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 { return false; }
    let hex = |s: &str| s.len() > 0 && s.chars().all(|c| c.is_ascii_hexdigit());
    hex(parts[0]) && hex(parts[1]) && hex(parts[2]) && hex(parts[3]) && hex(parts[4])
}