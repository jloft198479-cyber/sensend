// 飞书适配器 - 仅支持追加内容到已有文档
// 注意：飞书 tenant_access_token 只能访问已添加应用的文档

use super::{PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::{json, Value};

const FEISHU_BASE: &str = "https://open.feishu.cn/open-apis";

// ── 飞书 block_type 常量 ──
const BLOCK_TEXT: i64 = 2;
const BLOCK_HEADING1: i64 = 3;
const BLOCK_BULLET: i64 = 12;
const BLOCK_ORDERED: i64 = 13;
const BLOCK_CODE: i64 = 14;
const BLOCK_QUOTE: i64 = 15;
const BLOCK_DIVIDER: i64 = 22;

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

        body.get("tenant_access_token")
            .and_then(|t| t.as_str())
            .map(|t| t.to_string())
            .ok_or_else(|| "飞书认证响应中缺少 tenant_access_token".into())
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

/// 从 TipTap 节点提取纯文本
fn extract_text_from_node(node: &Value) -> String {
    let mut text = String::new();
    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if child_type == "text" {
                if let Some(t) = child.get("text").and_then(|t| t.as_str()) {
                    text.push_str(t);
                }
            } else {
                text.push_str(&extract_text_from_node(child));
            }
        }
    }
    text
}

/// TipTap inline marks → 飞书 text_element
fn marks_to_text_elements(node: &Value) -> Vec<Value> {
    let mut elements = Vec::new();

    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if child_type == "text" {
                let text = child.get("text").and_then(|t| t.as_str()).unwrap_or("");
                if text.is_empty() {
                    continue;
                }

                let mut text_run = serde_json::Map::new();
                text_run.insert("content".into(), json!(text));

                let mut style = serde_json::Map::new();
                if let Some(marks) = child.get("marks").and_then(|m| m.as_array()) {
                    for mark in marks {
                        let mark_type = mark.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match mark_type {
                            "bold" => { style.insert("bold".into(), json!(true)); }
                            "italic" => { style.insert("italic".into(), json!(true)); }
                            "strike" => { style.insert("strikethrough".into(), json!(true)); }
                            "code" => { style.insert("inline_code".into(), json!(true)); }
                            "link" => {
                                let href = mark.get("attrs")
                                    .and_then(|a| a.get("href"))
                                    .and_then(|h| h.as_str())
                                    .unwrap_or("");
                                if !href.is_empty() {
                                    style.insert("link".into(), json!({ "url": href }));
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if !style.is_empty() {
                    text_run.insert("text_element_style".into(), Value::Object(style));
                }

                elements.push(json!({ "text_run": Value::Object(text_run) }));
            } else if child_type == "hardBreak" {
                elements.push(json!({ "text_run": { "content": "\n" } }));
            } else {
                elements.extend(marks_to_text_elements(child));
            }
        }
    }

    elements
}

/// TipTap 节点 → 飞书 block
fn node_to_lark_block(node: &Value) -> Option<Value> {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    let text_elements = marks_to_text_elements(node);

    match node_type {
        "paragraph" => {
            Some(json!({
                "block_type": BLOCK_TEXT,
                "text": {
                    "elements": if text_elements.is_empty() {
                        vec![json!({ "text_run": { "content": "" } })]
                    } else {
                        text_elements
                    },
                    "style": {}
                }
            }))
        }
        "heading" => {
            let level = node.get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_i64())
                .unwrap_or(1);
            let block_type = BLOCK_HEADING1 + level - 1;
            let bt = if block_type > 11 { 11 } else { block_type };
            let heading_level = if level > 9 { 9 } else { level };
            let elements = if text_elements.is_empty() {
                vec![json!({ "text_run": { "content": "" } })]
            } else {
                text_elements
            };
            let heading_key = format!("heading{}", heading_level);
            let mut block = json!({ "block_type": bt });
            block.as_object_mut().unwrap().insert(heading_key, json!({
                "elements": elements,
                "style": {}
            }));
            Some(block)
        }
        "blockquote" => {
            if text_elements.is_empty() {
                return None;
            }
            Some(json!({
                "block_type": BLOCK_QUOTE,
                "quote": {
                    "elements": text_elements,
                    "style": {}
                }
            }))
        }
        "codeBlock" => {
            let code_text = extract_text_from_node(node);
            Some(json!({
                "block_type": BLOCK_CODE,
                "code": {
                    "elements": [{ "text_run": { "content": code_text } }],
                    "style": { "language": 1 }
                }
            }))
        }
        "horizontalRule" => {
            Some(json!({
                "block_type": BLOCK_DIVIDER,
                "divider": {}
            }))
        }
        _ => None,
    }
}

/// 从 listItem 节点提取 text elements
fn extract_list_item_elements(item: &Value) -> Vec<Value> {
    if let Some(children) = item.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if child.get("type").and_then(|t| t.as_str()) == Some("paragraph") {
                return marks_to_text_elements(child);
            }
        }
    }
    marks_to_text_elements(item)
}

/// TipTap JSON → 飞书 block 数组
fn tiptap_to_lark_blocks(content: &Value) -> Vec<Value> {
    let mut blocks = Vec::new();

    if let Some(doc) = content.get("content") {
        for node in doc.as_array().unwrap_or(&vec![]) {
            let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match node_type {
                "bulletList" => {
                    if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                        for item in items {
                            if item.get("type").and_then(|t| t.as_str()) == Some("listItem") {
                                let elements = extract_list_item_elements(item);
                                if elements.is_empty() { continue; }
                                blocks.push(json!({
                                    "block_type": BLOCK_BULLET,
                                    "bullet": { "elements": elements, "style": {} }
                                }));
                            }
                        }
                    }
                }
                "orderedList" => {
                    if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                        for item in items {
                            if item.get("type").and_then(|t| t.as_str()) == Some("listItem") {
                                let elements = extract_list_item_elements(item);
                                if elements.is_empty() { continue; }
                                blocks.push(json!({
                                    "block_type": BLOCK_ORDERED,
                                    "ordered": { "elements": elements, "style": {} }
                                }));
                            }
                        }
                    }
                }
                _ => {
                    if let Some(block) = node_to_lark_block(node) {
                        blocks.push(block);
                    }
                }
            }
        }
    }

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