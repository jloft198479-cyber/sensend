// 飞书适配器 - 仅支持追加内容到已有文档
// 注意：飞书 tenant_access_token 只能访问已添加应用的文档

use super::{PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const FEISHU_BASE: &str = "https://open.feishu.cn/open-apis";

// ── 飞书 block_type 常量 ──
const BLOCK_TEXT: i64 = 2;
const BLOCK_HEADING1: i64 = 3;
const BLOCK_BULLET: i64 = 12;
const BLOCK_ORDERED: i64 = 13;
const BLOCK_CODE: i64 = 14;
const BLOCK_QUOTE: i64 = 15;
const BLOCK_DIVIDER: i64 = 22;

/// 飞书 tenant_access_token 进程内缓存：key=app_id，有效期 2h（提前 5min 刷新）
struct CachedToken {
    token: String,
    expire_at: Instant,
}

static TOKEN_CACHE: OnceLock<Mutex<HashMap<String, CachedToken>>> = OnceLock::new();

pub struct LarkAdapter;

impl LarkAdapter {
    pub fn new() -> Self {
        log::info!("[Lark] REST API 版本 - 2026-04-29");
        Self
    }

    fn get_credentials(instance: &PlatformInstance) -> Result<(String, String), String> {
        let app_id = instance.token.trim();
        let app_secret = instance.token2.trim();
        
        if app_id.is_empty() || app_secret.is_empty() {
            return Err("请填写飞书 App ID 和 App Secret".into());
        }
        Ok((app_id.to_string(), app_secret.to_string()))
    }

    async fn get_tenant_token(client: &reqwest::Client, app_id: &String, app_secret: &String) -> Result<String, String> {
        // 1. 查缓存（临界区不含 .await，不会阻塞 async 线程池）
        {
            let cache = TOKEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
            if let Ok(g) = cache.lock() {
                if let Some(c) = g.get(app_id) {
                    if c.expire_at > Instant::now() {
                        return Ok(c.token.clone());
                    }
                }
            }
        }

        // 2. 缓存未命中或已过期 → 重新获取
        let res = client
            .post(format!("{}/auth/v3/tenant_access_token/internal", FEISHU_BASE))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&json!({
                "app_id": app_id,
                "app_secret": app_secret,
            }))
            .send()
            .await
            .map_err(|e| format!("飞书认证请求失败: {}", e))?;

        let status = res.status();
        let body: Value = res.json().await.unwrap_or_default();

        if let Some(code) = body.get("code").and_then(|c| c.as_i64()) {
            if code != 0 {
                let msg = body.get("msg").and_then(|m| m.as_str()).unwrap_or("未知错误");
                return Err(format!("飞书认证失败 (code={}): {}", code, msg));
            }
        }

        if !status.is_success() {
            return Err(format!("飞书认证 HTTP 错误 ({})", status));
        }

        let token = body.get("tenant_access_token")
            .and_then(|t| t.as_str())
            .map(|t| t.to_string())
            .ok_or_else(|| "飞书认证响应中缺少 tenant_access_token".to_string())?;

        // 3. 写入缓存（提前 5 分钟刷新，避免边界失效；失败不写缓存）
        {
            let cache = TOKEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
            if let Ok(mut g) = cache.lock() {
                g.insert(app_id.clone(), CachedToken {
                    token: token.clone(),
                    expire_at: Instant::now() + Duration::from_secs(2 * 3600 - 300),
                });
            }
        }

        Ok(token)
    }

    async fn request(
        &self,
        client: &reqwest::Client,
        method: &str,
        path: &str,
        token: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let url = format!("{}{}", FEISHU_BASE, path);

        log::debug!("[Lark] {} {}", method, url);

        let mut req = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            _ => return Err(format!("不支持的 HTTP 方法: {}", method)),
        };

        req = req
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json; charset=utf-8");

        if let Some(b) = body {
            req = req.json(&b);
            log::debug!("[Lark] 请求体: {}", serde_json::to_string(&b).unwrap_or_default());
        }

        let res = req.send().await.map_err(|e| format!("请求失败: {}", e))?;

        let status = res.status();
        let body: Value = res.json().await.unwrap_or_default();

        log::debug!("[Lark] 响应状态: {}", status);

        if let Some(code) = body.get("code").and_then(|c| c.as_i64()) {
            if code != 0 {
                let msg = body.get("msg").and_then(|m| m.as_str()).unwrap_or("未知错误");
                return Err(format!("飞书 API 错误 (code={}): {}", code, msg));
            }
        }

        if !status.is_success() {
            return Err(format!("HTTP 错误 ({})", status));
        }

        Ok(body)
    }

    pub fn resolve_lark_id(raw: &str) -> String {
        let trimmed = raw.trim();

        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            return trimmed.to_string();
        }

        let without_query = trimmed.split('?').next().unwrap_or(trimmed);
        let without_trailing = without_query.trim_end_matches('/');
        let segments: Vec<&str> = without_trailing.split('/').filter(|s| !s.is_empty()).collect();

        if let Some(&last) = segments.last() {
            return last.to_string();
        }

        trimmed.to_string()
    }

    fn is_wiki_url(raw: &str) -> bool {
        raw.trim().to_lowercase().contains("/wiki/")
    }

    async fn resolve_document_id(
        &self,
        client: &reqwest::Client,
        token: &str,
        raw_target: &str,
    ) -> Result<String, String> {
        let extracted = Self::resolve_lark_id(raw_target);

        if !Self::is_wiki_url(raw_target) {
            return Ok(extracted);
        }

        log::debug!("[Lark] 检测到 wiki URL，解析 node_token={} → document_id", extracted);

        let body = self.request(
            client,
            "GET",
            &format!("/wiki/v2/spaces/get_node?token={}", extracted),
            token,
            None,
        ).await?;

        let node = body
            .get("data")
            .and_then(|d| d.get("node"))
            .ok_or("飞书 wiki 节点响应中缺少 node")?;

        let obj_type = node.get("obj_type").and_then(|t| t.as_str()).unwrap_or("");

        if obj_type != "docx" && obj_type != "doc" {
            return Err(format!("wiki 中嵌入的类型 '{}' 暂不支持，请使用文档链接", obj_type));
        }

        let obj_token = node.get("obj_token").and_then(|t| t.as_str())
            .ok_or("飞书 wiki 节点响应中缺少 obj_token")?;

        log::debug!("[Lark] wiki 解析成功: {} → {}", extracted, obj_token);
        Ok(obj_token.to_string())
    }

    async fn append_blocks(
        &self,
        client: &reqwest::Client,
        token: &str,
        document_id: &str,
        blocks: Vec<Value>,
    ) -> Result<(), String> {
        for (chunk_idx, chunk) in blocks.chunks(50).enumerate() {
            log::debug!("[Lark] append chunk {} 共 {} 个 block", chunk_idx, chunk.len());

            self.request(
                client,
                "POST",
                &format!("/docx/v1/documents/{}/blocks/{}/children", document_id, document_id),
                token,
                Some(json!({ "children": chunk })),
            ).await?;
        }

        Ok(())
    }

    async fn get_file_url(
        &self,
        client: &reqwest::Client,
        token: &str,
        document_id: &str,
    ) -> Result<String, String> {
        let body = self.request(
            client,
            "GET",
            &format!("/drive/v1/files/{}?type=docx", document_id),
            token,
            None,
        ).await?;

        body.get("data")
            .and_then(|d| d.get("file"))
            .and_then(|f| f.get("url"))
            .and_then(|u| u.as_str())
            .map(|u| u.to_string())
            .ok_or_else(|| "飞书文档链接响应中缺少 url 字段".into())
    }
}

/// IR 行内内容 → 飞书 text elements
/// hardBreak → 换行文本；mention → "@标签" 文本（防数据丢失）
fn inlines_to_elements(inlines: &[super::ir::Inline]) -> Vec<Value> {
    use super::ir::{Inline, Mark};
    let mut elements = Vec::new();
    for inline in inlines {
        let (text, marks): (String, &Vec<Mark>) = match inline {
            Inline::Text { text, marks } => (text.clone(), marks),
            Inline::Break => ("\n".to_string(), &Vec::new()),
            Inline::Mention(label) => {
                elements.push(make_text_run(&format!("@{}", label), &[]));
                continue;
            }
        };
        if text.is_empty() {
            continue;
        }
        elements.push(make_text_run(&text, marks));
    }
    elements
}

/// 构造单个 text_run（marks → text_element_style）
fn make_text_run(text: &str, marks: &[super::ir::Mark]) -> Value {
    use super::ir::Mark;
    let mut text_run = serde_json::Map::new();
    text_run.insert("content".into(), json!(text));

    let mut style = serde_json::Map::new();
    for mark in marks {
        match mark {
            Mark::Bold => { style.insert("bold".into(), json!(true)); }
            Mark::Italic => { style.insert("italic".into(), json!(true)); }
            Mark::Strike => { style.insert("strikethrough".into(), json!(true)); }
            Mark::Underline => { style.insert("underline".into(), json!(true)); }
            Mark::Code => { style.insert("inline_code".into(), json!(true)); }
            Mark::Link(href) => {
                if !href.is_empty() {
                    style.insert("link".into(), json!({ "url": href }));
                }
            }
        }
    }
    if !style.is_empty() {
        text_run.insert("text_element_style".into(), Value::Object(style));
    }

    json!({ "text_run": Value::Object(text_run) })
}

/// 空的 text_run（飞书要求 elements 非空）
fn empty_element() -> Value {
    json!({ "text_run": { "content": "" } })
}

/// IR 块 → 飞书 block(s)
fn map_blocks(blocks: &[super::ir::Block], out: &mut Vec<Value>) {
    use super::ir::Block;
    for block in blocks {
        match block {
            Block::Paragraph(inlines) => {
                let mut elements = inlines_to_elements(inlines);
                if elements.is_empty() {
                    elements.push(empty_element());
                }
                out.push(json!({
                    "block_type": BLOCK_TEXT,
                    "text": { "elements": elements, "style": {} }
                }));
            }
            Block::Heading { level, inlines } => {
                let level = *level as i64;
                let block_type = BLOCK_HEADING1 + level - 1;
                let bt = if block_type > 11 { 11 } else { block_type };
                let heading_level = if level > 9 { 9 } else { level };
                let mut elements = inlines_to_elements(inlines);
                if elements.is_empty() {
                    elements.push(empty_element());
                }
                let heading_key = format!("heading{}", heading_level);
                let mut block = json!({ "block_type": bt });
                block.as_object_mut().unwrap().insert(heading_key, json!({
                    "elements": elements,
                    "style": {}
                }));
                out.push(block);
            }
            Block::List { kind, items } => {
                for item in items {
                    map_list_item(*kind, item, out);
                }
            }
            Block::CodeBlock { code, .. } => {
                out.push(json!({
                    "block_type": BLOCK_CODE,
                    "code": {
                        "elements": [{ "text_run": { "content": code } }],
                        "style": { "language": 1 }
                    }
                }));
            }
            Block::BlockQuote(paras) => {
                for para in paras {
                    let elements = inlines_to_elements(para);
                    if elements.is_empty() {
                        continue;
                    }
                    out.push(json!({
                        "block_type": BLOCK_QUOTE,
                        "quote": { "elements": elements, "style": {} }
                    }));
                }
            }
            Block::Table(table) => {
                // 飞书无原生表格，降级为逐行文本（保留行内格式，修复 #5）
                for row in &table.rows {
                    let mut elements: Vec<Value> = Vec::new();
                    for (ci, cell) in row.iter().enumerate() {
                        if ci > 0 {
                            elements.push(make_text_run(" | ", &[]));
                        }
                        elements.extend(inlines_to_elements(cell));
                    }
                    if elements.is_empty() {
                        continue;
                    }
                    out.push(json!({
                        "block_type": BLOCK_TEXT,
                        "text": { "elements": elements, "style": {} }
                    }));
                }
            }
            Block::HorizontalRule => {
                out.push(json!({
                    "block_type": BLOCK_DIVIDER,
                    "divider": {}
                }));
            }
        }
    }
}

/// 列表项 → 飞书 block（修复 #2 拍平保文本 + #3 多段落合并）
fn map_list_item(kind: super::ir::ListKind, item: &super::ir::ListItem, out: &mut Vec<Value>) {
    // rich_text：首段 + 后续段落（\n 分隔，修复 #3 多段落丢失）
    let mut elements = inlines_to_elements(&item.inlines);
    for para in &item.extra_paras {
        elements.push(make_text_run("\n", &[]));
        elements.extend(inlines_to_elements(para));
    }

    // 待办项：首段文本前缀 [x]/[ ]（保持历史行为）
    if kind == super::ir::ListKind::Task {
        if let Some(e) = elements.first_mut() {
            if let Some(tr) = e.get_mut("text_run").and_then(|tr| tr.as_object_mut()) {
                if let Some(content) = tr.get("content").and_then(|c| c.as_str()).map(|s| s.to_string()) {
                    let prefix = if item.checked.unwrap_or(false) { "[x] " } else { "[ ] " };
                    tr.insert("content".into(), json!(format!("{}{}", prefix, content)));
                }
            }
        }
    }

    if !elements.is_empty() {
        let block_type = match kind {
            super::ir::ListKind::Bullet | super::ir::ListKind::Task => BLOCK_BULLET,
            super::ir::ListKind::Ordered => BLOCK_ORDERED,
        };
        let key = if block_type == BLOCK_BULLET { "bullet" } else { "ordered" };
        let mut block = json!({ "block_type": block_type });
        block.as_object_mut().unwrap().insert(key.to_string(), json!({
            "elements": elements,
            "style": {}
        }));
        out.push(block);
    }

    // 嵌套子列表：飞书拍平为同级块（文本不丢，层级降级）
    for child in &item.children {
        map_blocks(std::slice::from_ref(child), out);
    }
}

/// TipTap JSON → 飞书 block 数组
pub(crate) fn tiptap_to_lark_blocks(content: &Value) -> Vec<Value> {
    let mut blocks = Vec::new();
    map_blocks(&super::ir::parse(content), &mut blocks);
    blocks
}

#[async_trait]
impl PlatformAdapter for LarkAdapter {

    async fn probe_type(&self, _instance: &PlatformInstance) -> Result<String, String> {
        Ok("page".to_string())
    }

    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String> {
        let (app_id, app_secret) = Self::get_credentials(instance)?;
        let client = super::http_client();

        log::info!("[Lark] 测试连接, app_id={}...", &app_id[..8.min(app_id.len())]);

        let token = Self::get_tenant_token(&client, &app_id, &app_secret).await?;
        log::info!("[Lark] tenant_access_token 获取成功");

        let document_id = self.resolve_document_id(&client, &token, &instance.target_id).await?;

        self.request(&client, "GET", &format!("/docx/v1/documents/{}", document_id), &token, None).await?;
        log::info!("[Lark] 文档访问成功");

        Ok(())
    }

    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        let (app_id, app_secret) = Self::get_credentials(instance)?;
        let client = super::http_client();

        let token = Self::get_tenant_token(&client, &app_id, &app_secret).await?;
        log::info!("[Lark] tenant_access_token 获取成功");

        let document_id = self.resolve_document_id(&client, &token, &instance.target_id).await?;
        log::info!("[Lark] 目标文档: {}", document_id);

        let mut blocks = vec![json!({
            "block_type": BLOCK_DIVIDER,
            "divider": {}
        })];
        blocks.extend(tiptap_to_lark_blocks(content));

        self.append_blocks(&client, &token, &document_id, blocks).await?;

        let url = self.get_file_url(&client, &token, &document_id).await
            .unwrap_or_else(|_| instance.target_id.trim().to_string());

        Ok(PublishResult {
            success: true,
            message: "追加成功".into(),
            url: Some(url),
        })
    }

    async fn append_blocks(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        // 飞书只有追加模式，publish 和 append_blocks 行为一致
        self.publish(content, instance).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers;

    fn convert(tree: &Value) -> Vec<Value> {
        tiptap_to_lark_blocks(tree)
    }

    macro_rules! golden_tests {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let fixture = test_helpers::load_fixture(stringify!($name));
                    let output = convert(&fixture);
                    test_helpers::assert_or_update_golden("lark", stringify!($name), "json", &test_helpers::format_json(&serde_json::Value::Array(output)));
                }
            )*
        };
    }

    golden_tests!(
        simple_paragraph,
        headings,
        nested_list,
        table_with_inline,
        hardbreak,
        tasklist,
        codeblock,
        blockquote,
        long_title,
        underline_link,
        combined
    );

    // ── 目标断言：缺陷修复不被快照更新悄悄退化 ──

    #[test]
    fn fix2_nested_list_flattened_not_dropped() {
        let fixture = test_helpers::load_fixture("nested_list");
        let blocks = convert(&fixture);
        // 现状缺陷是子项完全丢失；修复后拍平输出：顶层 2 项 + 子项 2 项 = 4 个 bullet
        let bullets: Vec<&Value> = blocks.iter()
            .filter(|b| b["block_type"] == json!(BLOCK_BULLET))
            .collect();
        assert_eq!(bullets.len(), 4, "嵌套子项应拍平输出而不是丢弃");
        let texts: Vec<String> = bullets.iter().map(|b| {
            b["bullet"]["elements"][0]["text_run"]["content"].as_str().unwrap_or("").to_string()
        }).collect();
        assert!(texts.contains(&"子项 1.1".to_string()), "子项文本不丢: {:?}", texts);
        assert!(texts.contains(&"子项 1.2".to_string()));
    }

    #[test]
    fn fix3_list_item_multi_paragraph_kept() {
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "bulletList",
                "content": [{
                    "type": "listItem",
                    "content": [
                        { "type": "paragraph", "content": [{ "type": "text", "text": "首段" }] },
                        { "type": "paragraph", "content": [{ "type": "text", "text": "次段" }] }
                    ]
                }]
            }]
        });
        let blocks = convert(&doc);
        let elements = blocks[0]["bullet"]["elements"].as_array().unwrap();
        let content: String = elements.iter()
            .filter_map(|e| e["text_run"]["content"].as_str())
            .collect();
        assert_eq!(content, "首段\n次段", "列表项多段落应保留（\\n 分隔）");
    }
}