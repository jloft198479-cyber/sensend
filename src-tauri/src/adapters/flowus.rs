use super::{markdown, resolve_target_id, PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.flowus.cn/v1";

pub struct FlowUsAdapter;

/// 目标类型
enum TargetType {
    Database { db_id: String },
    Page,
}

impl FlowUsAdapter {
    pub fn new() -> Self { 
        log::info!("[FlowUs] REST API 版本 - 2026-04-29");
        Self 
    }

    /// 发送 HTTP 请求
    async fn request(
        &self,
        method: &str,
        path: &str,
        token: &str,
        body: Option<Value>,
    ) -> Result<Value, String> {
        let client = super::http_client();
        let url = format!("{}{}", API_BASE, path);
        
        log::debug!("[FlowUs] {} {}", method, url);
        
        let mut req = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => return Err(format!("不支持的 HTTP 方法: {}", method)),
        };
        
        req = req.header("Authorization", format!("Bearer {}", token));
        
        if let Some(b) = body {
            req = req.json(&b);
            log::debug!("[FlowUs] 请求体: {}", serde_json::to_string(&b).unwrap_or_default());
        }
        
        let res = req.send().await.map_err(|e| format!("请求失败: {}", e))?;
        
        let status = res.status();
        let body: Value = res.json().await.unwrap_or_default();
        
        log::debug!("[FlowUs] 响应状态: {}", status);
        log::debug!("[FlowUs] 响应体: {}", serde_json::to_string(&body).unwrap_or_default());
        
        if !status.is_success() {
            let default_msg = format!("HTTP 错误 ({})", status);
            let msg = body.get("message").and_then(|m| m.as_str())
                .unwrap_or(&default_msg);
            return Err(format!("FlowUs API: {}", msg));
        }
        
        Ok(body)
    }

    // ── FlowUs 标准注解 ──
    fn default_annotations() -> Value {
        json!({
            "bold": false,
            "italic": false,
            "strikethrough": false,
            "underline": false,
            "code": false,
            "color": "default"
        })
    }

    /// 生成 title property
    fn make_title_property(&self, title: &str) -> Value {
        json!({
            "title": {
                "type": "title",
                "title": [{
                    "type": "text",
                    "text": { "content": title, "link": null },
                    "annotations": Self::default_annotations()
                }]
            }
        })
    }

    // ── TipTap JSON → FlowUs Blocks 转换 ──

    /// 将 TipTap JSON 文档树转换为 FlowUs block 数组
    fn tiptap_to_blocks(&self, tree: &Value) -> Vec<Value> {
        let mut blocks = Vec::new();
        if let Some(content) = tree.get("content").and_then(|c| c.as_array()) {
            for node in content {
                blocks.extend(self.convert_node(node));
            }
        }
        if blocks.is_empty() {
            blocks.push(json!({
                "type": "paragraph",
                "data": { "rich_text": [{"type":"text","text":{"content":"","link":null}}] }
            }));
        }
        blocks
    }

    /// 转换单个 TipTap 节点为 FlowUs block(s)
    fn convert_node(&self, node: &Value) -> Vec<Value> {
        let mut out = Vec::new();
        let t = match node.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return out,
        };
        match t {
            "paragraph" => {
                out.push(json!({
                    "type": "paragraph",
                    "data": {
                        "rich_text": self.convert_text(node),
                        "text_color": "default",
                        "background_color": "default"
                    }
                }));
            }
            "heading" => {
                let level = node.get("attrs").and_then(|a| a.get("level")).and_then(|l| l.as_u64()).unwrap_or(1);
                let ht = if level == 1 { "heading_1" } else if level == 2 { "heading_2" } else { "heading_3" };
                out.push(json!({
                    "type": ht,
                    "data": {
                        "rich_text": self.convert_text(node),
                        "text_color": "default",
                        "background_color": "default"
                    }
                }));
            }
            "bulletList" => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        out.push(json!({
                            "type": "bulleted_list_item",
                            "data": {
                                "rich_text": self.convert_text(item),
                                "text_color": "default",
                                "background_color": "default"
                            }
                        }));
                    }
                }
            }
            "orderedList" => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        out.push(json!({
                            "type": "numbered_list_item",
                            "data": {
                                "rich_text": self.convert_text(item),
                                "text_color": "default",
                                "background_color": "default"
                            }
                        }));
                    }
                }
            }
            "blockquote" => {
                if let Some(paragraphs) = node.get("content").and_then(|c| c.as_array()) {
                    for para in paragraphs {
                        out.push(json!({
                            "type": "quote",
                            "data": {
                                "rich_text": self.convert_text(para),
                                "text_color": "default",
                                "background_color": "default"
                            }
                        }));
                    }
                } else {
                    out.push(json!({
                        "type": "quote",
                        "data": {
                            "rich_text": self.convert_text(node),
                            "text_color": "default",
                            "background_color": "default"
                        }
                    }));
                }
            }
            "codeBlock" => {
                let lang = node.get("attrs")
                    .and_then(|a| a.get("language").and_then(|l| l.as_str()))
                    .unwrap_or("plain text");
                out.push(json!({
                    "type": "code",
                    "data": {
                        "rich_text": self.convert_text(node),
                        "language": lang
                    }
                }));
            }
            "horizontalRule" => {
                out.push(json!({
                    "type": "divider",
                    "data": {}
                }));
            }
            _ => {}
        }
        out
    }

    /// 将节点的子内容转换为 FlowUs rich_text 数组
    fn convert_text(&self, node: &Value) -> Vec<Value> {
        let mut rt = Vec::new();
        if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
            for child in content {
                rt.extend(self.collect_text_nodes(child));
            }
        }
        if rt.is_empty() { 
            rt.push(json!({"type":"text","text":{"content":"","link":null}})); 
        }
        rt
    }

    /// 递归收集文本节点，生成 FlowUs rich_text 元素（含 annotations）
    fn collect_text_nodes(&self, node: &Value) -> Vec<Value> {
        let t = match node.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Vec::new(),
        };

        if t != "text" {
            let mut result = Vec::new();
            if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    result.extend(self.collect_text_nodes(child));
                }
            }
            return result;
        }

        // 处理 text 节点
        let text = node.get("text").and_then(|t| t.as_str()).unwrap_or("");
        let mut anno = serde_json::Map::new();
        let mut link_url: Option<String> = None;
        if let Some(marks) = node.get("marks").and_then(|m| m.as_array()) {
            for mark in marks {
                match mark.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                    "bold" => { anno.insert("bold".into(), json!(true)); }
                    "italic" => { anno.insert("italic".into(), json!(true)); }
                    "strike" => { anno.insert("strikethrough".into(), json!(true)); }
                    "code" => { anno.insert("code".into(), json!(true)); }
                    "link" => {
                        if let Some(href) = mark.get("attrs").and_then(|a| a.get("href")).and_then(|h| h.as_str()) {
                            link_url = Some(href.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        // 构造 FlowUs rich_text 元素
        let mut text_obj = serde_json::Map::new();
        text_obj.insert("content".into(), json!(text));
        if let Some(ref url) = link_url {
            text_obj.insert("link".into(), json!({ "url": url }));
        } else {
            text_obj.insert("link".into(), Value::Null);
        }

        // 合并用户 marks 和默认值
        let mut full_anno = serde_json::Map::new();
        full_anno.insert("bold".into(), json!(anno.contains_key("bold")));
        full_anno.insert("italic".into(), json!(anno.contains_key("italic")));
        full_anno.insert("strikethrough".into(), json!(anno.contains_key("strikethrough")));
        full_anno.insert("underline".into(), json!(false));
        full_anno.insert("code".into(), json!(anno.contains_key("code")));
        full_anno.insert("color".into(), json!("default"));

        let mut obj = serde_json::Map::new();
        obj.insert("type".into(), json!("text"));
        obj.insert("text".into(), Value::Object(text_obj));
        obj.insert("annotations".into(), Value::Object(full_anno));
        obj.insert("plain_text".into(), json!(text));
        obj.insert("href".into(), if link_url.is_some() { json!(link_url) } else { Value::Null });

        vec![Value::Object(obj)]
    }

    // ── 目标类型判断 ──

    /// 判断目标 ID 是数据库还是页面
    async fn resolve_target(
        &self,
        token: &str,
        target_id: &str,
    ) -> Result<TargetType, String> {
        // 获取块的子块
        let result = self.request(
            "GET",
            &format!("/blocks/{}/children?page_size=100", target_id),
            token,
            None,
        ).await;
        
        match result {
            Ok(children) => {
                // 检查是否有 child_database
                if let Some(results) = children.get("results").and_then(|r| r.as_array()) {
                    for block in results {
                        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if block_type == "child_database" {
                            let db_id = block.get("id").and_then(|id| id.as_str())
                                .unwrap_or(target_id);
                            return Ok(TargetType::Database { db_id: db_id.to_string() });
                        }
                    }
                }
                Ok(TargetType::Page)
            }
            Err(_) => {
                // 如果获取子块失败，假设是普通页面
                Ok(TargetType::Page)
            }
        }
    }

    /// 创建页面
    async fn create_page(
        &self,
        token: &str,
        parent_id: &str,
        title: &str,
        is_database: bool,
    ) -> Result<(String, String), String> {
        let parent = if is_database {
            json!({ "database_id": parent_id })
        } else {
            json!({ "page_id": parent_id })
        };

        let body = json!({
            "parent": parent,
            "properties": self.make_title_property(title)
        });

        let result = self.request("POST", "/pages", token, Some(body)).await?;
        
        let page_id = result.get("id").and_then(|id| id.as_str())
            .ok_or("创建页面失败：未返回页面 ID")?
            .to_string();
        
        let page_url = result.get("url")
            .and_then(|u| u.as_str())
            .map(|u| u.to_string())
            .unwrap_or_else(|| format!("https://flowus.cn/docs/{}", page_id));

        Ok((page_id, page_url))
    }

    /// 追加内容块到页面
    async fn append_children(
        &self,
        token: &str,
        page_id: &str,
        blocks: Vec<Value>,
    ) -> Result<(), String> {
        // 分批追加（每批最多 100 个）
        for chunk in blocks.chunks(100) {
            let body = json!({ "children": chunk });
            self.request(
                "PATCH",
                &format!("/blocks/{}/children", page_id),
                token,
                Some(body),
            ).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl PlatformAdapter for FlowUsAdapter {

    async fn probe_type(&self, instance: &PlatformInstance) -> Result<String, String> {
        let target_id = resolve_target_id("flowus", &instance.target_id);
        let target = self.resolve_target(&instance.token, &target_id).await?;
        match target {
            TargetType::Database { .. } => Ok("database".to_string()),
            TargetType::Page => Ok("page".to_string()),
        }
    }

    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String> {
        let target_id = resolve_target_id("flowus", &instance.target_id);
        log::info!("[FlowUs] 测试连接, target_id={}", &target_id);
        
        // 获取页面信息
        match self.request("GET", &format!("/pages/{}", target_id), &instance.token, None).await {
            Ok(result) => {
                log::info!("[FlowUs] 测试连接成功");
                log::debug!("[FlowUs] 页面信息: {}", result);
                Ok(())
            }
            Err(e) => {
                log::warn!("[FlowUs] 测试连接失败: {}", e);
                Err(e)
            }
        }
    }

    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        let target_id = resolve_target_id("flowus", &instance.target_id);
        
        // 提取标题
        let title = markdown::extract_title(content);
        
        // 判断目标类型，获取正确的父级 ID
        let target = self.resolve_target(&instance.token, &target_id).await?;
        let (parent_id, is_database) = match target {
            TargetType::Database { db_id } => (db_id, true),
            TargetType::Page => (target_id, false),
        };
        
        // 创建页面
        let (page_id, page_url) = self.create_page(
            &instance.token,
            &parent_id,
            &title,
            is_database,
        ).await?;
        
        log::info!("[FlowUs] 创建页面成功: {}", page_id);
        
        // 转换内容为 blocks
        let mut blocks = self.tiptap_to_blocks(content);
        
        // 如果第一个 block 是 heading 且文本与 title 相同，跳过
        if let Some(first) = blocks.first() {
            let is_heading = first.get("type")
                .and_then(|t| t.as_str())
                .map(|t| t.starts_with("heading_"))
                .unwrap_or(false);
            if is_heading {
                let first_text = first.get("data")
                    .and_then(|d| d.get("rich_text"))
                    .and_then(|rt| rt.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|t| t.get("text").and_then(|t| t.get("content")).and_then(|c| c.as_str()))
                        .collect::<String>())
                    .unwrap_or_default();
                if first_text == title {
                    blocks.remove(0);
                }
            }
        }
        
        // 追加内容到新页面
        if !blocks.is_empty() {
            self.append_children(&instance.token, &page_id, blocks).await?;
            log::info!("[FlowUs] 追加内容成功");
        }

        Ok(PublishResult {
            success: true,
            message: "发送成功".into(),
            url: Some(page_url),
        })
    }

    async fn append_blocks(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        let target_id = resolve_target_id("flowus", &instance.target_id);
        
        // 构建追加 blocks：分隔线 + 正文
        let mut children: Vec<Value> = Vec::new();

        // 分隔线
        children.push(json!({
            "type": "divider",
            "data": {}
        }));

        // 正文
        children.extend(self.tiptap_to_blocks(content));

        // 追加
        self.append_children(&instance.token, &target_id, children).await?;
        
        log::info!("[FlowUs] 追加内容成功");

        Ok(PublishResult {
            success: true,
            message: "追加成功".into(),
            url: Some(format!("https://flowus.cn/docs/{}", target_id)),
        })
    }
}