use serde_json::Value;
use async_trait::async_trait;
use std::sync::OnceLock;

/// 全局 HTTP 客户端，整个应用生命周期复用一个实例（连接池、TLS 会话共享）
static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// 获取全局 HTTP 客户端引用
pub fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            // 显式禁用代理：更新检查的代理探测会设置 HTTPS_PROXY（进程级），
            // 若平台发布客户端在探测之后首次构建会继承代理，代理挂掉则发布静默失效
            .no_proxy()
            .build()
            .expect("Failed to build HTTP client")
    })
}

fn default_publish_mode() -> String { "page".to_string() }

// ── 出站请求的限流重试（M-11 配套）──
//
// 分片写入（Notion/FlowUs 100 块一批、飞书 50 块一批）中途被 429 打断时，
// 已写入的分片留在平台上；用户手动重点一次就会重复写入。分片内没法做幂等
// （平台 API 不支持「按内容去重」），所以只能把「服务端明确没处理」的失败
// 就地吸收掉，减少用户手动重试的机会。

/// 值得重试的状态码。
/// - **429 限流**：请求被服务端明确拒绝、没处理过，任何方法都能安全重放；
/// - **502/503/504 网关错误**：只对 GET 放宽——网关超时时后端的写操作可能已经落地，
///   重放 POST/PATCH 会重复创建（Notion 会多出一整个页面）。
/// - 网络超时 / 连接错误一律不重试：同样是因为无法判断服务端到底写了没有。
pub fn is_retryable_status(status: u16, method: &str) -> bool {
    if status == 429 {
        return true;
    }
    matches!(status, 502 | 503 | 504) && method.eq_ignore_ascii_case("GET")
}

/// 重试退避：429 的 Retry-After 通常是秒级，两次固定退避足够吸收抖动
pub fn retry_backoff(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_millis(if attempt <= 1 { 1_000 } else { 3_000 })
}

/// 单个请求最多重试次数（不含首次）
pub const MAX_RETRIES: u32 = 2;

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
            supported_modes: vec!["page".into()],
            fields: vec![
                ConfigField { key: "token".into(), label: "（无需填写）".into(), hint: "".into(), secret: true, hidden: true, browse: false, default_value: Some("local".into()), optional: true },
                ConfigField { key: "target_id".into(), label: "文件夹路径".into(), hint: "如 D:\\Notes 或 C:\\Users\\xxx\\Desktop".into(), secret: false, hidden: false, browse: true, default_value: None, optional: false },
            ],
        },
        PlatformTypeInfo {
            key: "notion".into(),
            name: "Notion".into(),
            supported_modes: vec!["page".into(), "block".into()],
            fields: vec![
                ConfigField { key: "token".into(), label: "Integration Token".into(), hint: "Notion Integration Token".into(), secret: true, hidden: false, browse: false, default_value: None, optional: false },
                ConfigField { key: "target_id".into(), label: "Parent Page".into(), hint: "粘贴页面链接，自动解析 ID".into(), secret: false, hidden: false, browse: false, default_value: None, optional: false },
            ],
        },
        PlatformTypeInfo {
            key: "flowus".into(),
            name: "FlowUs".into(),
            supported_modes: vec!["page".into(), "block".into()],
            fields: vec![
                ConfigField { key: "token".into(), label: "授权码".into(), hint: "FlowUs MCP 授权码".into(), secret: true, hidden: false, browse: false, default_value: None, optional: false },
                ConfigField { key: "target_id".into(), label: "目标页面".into(), hint: "粘贴页面链接，自动解析 ID".into(), secret: false, hidden: false, browse: false, default_value: None, optional: false },
            ],
        },
        PlatformTypeInfo {
            key: "lark".into(),
            name: "飞书".into(),
            supported_modes: vec!["block".into()],
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
    pub fields: Vec<ConfigField>,
    /// 平台支持的写入模式（"page"=创建子页面 / "block"=追加到页面）。
    /// SSOT：前端「写入方式」显隐与合法性唯一依据；serde default 兼容旧序列化数据
    #[serde(default)]
    pub supported_modes: Vec<String>,
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

/// 平台适配器统一接口
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String>;
    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String>;

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
pub mod ir;

#[cfg(test)]
mod test_helpers;

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
///   UUID 形态: https://www.notion.so/笔记-34a6a710-9f45-80f4-b78d-f783f9905cd4 → 34a6a710...
fn resolve_notion_id(raw: &str) -> String {
    // 先剥 query 与 #锚点：块分享链接（…#32hex）的锚点同样是 32 位 hex，
    // 不剥会被下方「找 32hex」误当成页面 ID
    let base = raw.split(['?', '#']).next().unwrap_or(raw);
    // 只看路径最后一段：Notion 的页面 ID 一定在最后一段里。
    // 全域扫描会被前面的 workspace / space id 抢走——团队空间链接里有两段 32 位 hex。
    // 先剥尾部斜杠：`.../{id}/` 是常见的复制形态，不剥会取到空段
    let trimmed = base.trim_end_matches('/');
    let last_seg = trimmed.rsplit('/').next().unwrap_or(trimmed);
    let chars: Vec<char> = last_seg.chars().collect();
    let is_hex = |c: char| c.is_ascii_hexdigit();
    let len = chars.len();

    // 情形 1：整段就是 ID（无连字符 32hex，或 8-4-4-4-12 的 UUID）
    let whole: String = chars.iter().copied().filter(|&c| c != '-').collect();
    if whole.len() == 32 && whole.chars().all(is_hex) {
        return whole;
    }

    // 情形 2：ID 嵌在 slug 里 —— `{slug}-{32hex}` 或 `{uuid}-{slug}`。
    // 从左往右找第一个「去连字符后恰好 32 位 hex」的子串：
    // 32 位是硬约束，slug 里正常会带非 hex 字母，所以不会误吞 slug。
    for start in 0..len {
        // 不从一个更长的 hex 串中间起算（否则会截出半截 ID）
        if start > 0 && is_hex(chars[start - 1]) {
            continue;
        }
        // 32 位 hex 最多占 36 个字符（UUID 形态多 4 个连字符）
        for end in (start + 32)..=(len.min(start + 36)) {
            // 右边还是 hex → 这是更长 hex 串的前半截，不能截断当 ID
            // （只排 hex，不排连字符：`{uuid}-slug` 形态的右边就是 `-`）
            if end < len && is_hex(chars[end]) {
                continue;
            }
            let digits: String = chars[start..end]
                .iter()
                .copied()
                .filter(|&c| c != '-')
                .collect();
            if digits.len() == 32 && digits.chars().all(is_hex) {
                return digits;
            }
        }
    }

    // 兜底：原样返回（已剥 query/hash），让用户看到自己填的东西
    base.to_string()
}

/// FlowUs URL → 页面 ID（先剥 query/hash，再取最后一个路径段）
/// 例如: https://flowus.cn/weimabbs/640e8519-4ee1-44f9-97ec-ce3dd9788fb5 → 640e8519-4ee1-44f9-97ec-ce3dd9788fb5
/// 带参分享链接: https://flowus.cn/xxx/640e...?from=share → 640e...
fn resolve_flowus_id(raw: &str) -> String {
    let cleaned = raw.split(['?', '#']).next().unwrap_or(raw);
    let without_trailing = cleaned.trim_end_matches('/');
    let segments: Vec<&str> = without_trailing.split('/').filter(|s| !s.is_empty()).collect();
    if let Some(&last) = segments.last() {
        return last.to_string();
    }
    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_flowus_id_strips_query_and_hash() {
        // 带参分享链接：query/hash 不能混进 target_id，否则 API 400
        assert_eq!(
            resolve_flowus_id("https://flowus.cn/xxx/640e8519-4ee1-44f9-97ec-ce3dd9788fb5?from=share"),
            "640e8519-4ee1-44f9-97ec-ce3dd9788fb5"
        );
        assert_eq!(
            resolve_flowus_id("https://flowus.cn/xxx/640e8519-4ee1-44f9-97ec-ce3dd9788fb5#anchor"),
            "640e8519-4ee1-44f9-97ec-ce3dd9788fb5"
        );
        assert_eq!(
            resolve_flowus_id("https://flowus.cn/xxx/640e8519-4ee1-44f9-97ec-ce3dd9788fb5"),
            "640e8519-4ee1-44f9-97ec-ce3dd9788fb5"
        );
    }

    #[test]
    fn resolve_notion_id_strips_anchor_and_query() {
        // 块分享链接的 #锚点同样是 32 位 hex，必须剥掉取页面 ID
        assert_eq!(
            resolve_notion_id("https://www.notion.so/Notes-34a6a7109f4580f4b78df783f9905cd4#b8834a7109f4580f4b78df783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        assert_eq!(
            resolve_notion_id("https://www.notion.so/34a6a7109f4580f4b78df783f9905cd4?v=2"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        assert_eq!(
            resolve_notion_id("34a6a7109f4580f4b78df783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
    }

    #[test]
    fn resolve_notion_id_handles_hyphenated_uuid_in_url() {
        // [m-05 修复] 用户从 Notion「复制链接」默认给的正是带连字符的 UUID 形态。
        // 旧实现只扫连续 32 位 hex、且 36 位分支要求整串输入都是 UUID，
        // 结果是整条 URL 原样当 target_id 存下，之后每次发送都 400。
        assert_eq!(
            resolve_notion_id("https://www.notion.so/My-Page-34a6a710-9f45-80f4-b78d-f783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        assert_eq!(
            resolve_notion_id("https://www.notion.so/34a6a710-9f45-80f4-b78d-f783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        // 纯 UUID 输入（无 URL 外壳）
        assert_eq!(
            resolve_notion_id("34a6a710-9f45-80f4-b78d-f783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        // 带 query 的 UUID 链接
        assert_eq!(
            resolve_notion_id("https://www.notion.so/笔记-34a6a710-9f45-80f4-b78d-f783f9905cd4?v=1#block"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
    }

    #[test]
    fn resolve_notion_id_keeps_unknown_input() {
        // 认不出 ID 时原样返回（让用户看到自己填的东西，而不是被改写成半截）
        assert_eq!(resolve_notion_id("我的笔记"), "我的笔记");
        assert_eq!(resolve_notion_id("https://www.notion.so/alice"), "https://www.notion.so/alice");
    }

    #[test]
    fn resolve_notion_id_prefers_last_path_segment() {
        // 团队空间链接里有两段 32 位 hex（workspace + page）：
        // 必须取「最后一段」里的页面 ID，全域扫描会取到 workspace_id
        assert_eq!(
            resolve_notion_id("https://www.notion.so/11111111222222223333333344444444/Notes-34a6a7109f4580f4b78df783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        assert_eq!(
            resolve_notion_id("https://www.notion.so/11111111222222223333333344444444/34a6a710-9f45-80f4-b78d-f783f9905cd4"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
    }

    #[test]
    fn resolve_notion_id_handles_id_followed_by_slug() {
        // ID 在前、slug 在后（团队空间常见形态）：不能因为后面还有字符就流产
        assert_eq!(
            resolve_notion_id("https://www.notion.so/34a6a710-9f45-80f4-b78d-f783f9905cd4-My-Page"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        // slug 恰好是纯 hex 词（deadbeef）时，仍要取前面那个 32 位 ID，不能拼成垃圾串
        assert_eq!(
            resolve_notion_id("https://www.notion.so/34a6a710-9f45-80f4-b78d-f783f9905cd4-deadbeef"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
    }

    #[test]
    fn resolve_notion_id_handles_trailing_slash() {
        // 复制链接常带尾斜杠，不能因此取到空段而原样返回整条 URL
        assert_eq!(
            resolve_notion_id("https://www.notion.so/34a6a7109f4580f4b78df783f9905cd4/"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
        assert_eq!(
            resolve_notion_id("https://www.notion.so/Notes-34a6a7109f4580f4b78df783f9905cd4/?v=2"),
            "34a6a7109f4580f4b78df783f9905cd4"
        );
    }

    #[test]
    fn resolve_notion_id_rejects_overlong_hex() {
        // 超过 32 位的连续 hex 不是 Notion ID，不能截前 32 位当 ID
        let overlong = "https://www.notion.so/deadbeefdeadbeefdeadbeefdeadbeefdead";
        assert_eq!(resolve_notion_id(overlong), overlong, "36 位 hex 不应被截断成 32 位");
    }
}
