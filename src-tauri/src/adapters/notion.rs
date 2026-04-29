use super::{markdown, resolve_target_id, PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

pub struct NotionAdapter;

/// 从 Notion 数据库 schema 中提取的列名信息
struct DatabaseSchema {
    title_prop: String,
    date_prop: Option<String>,
}

/// 目标类型：数据库或普通页面
enum TargetType {
    Database { db_id: String, schema: DatabaseSchema },
    Page,
}

impl NotionAdapter {
    pub fn new() -> Self {
        log::info!("[Notion] REST API 版本 - 2026-04-29");
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

        log::debug!("[Notion] {} {}", method, url);

        let mut req = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PATCH" => client.patch(&url),
            "DELETE" => client.delete(&url),
            _ => return Err(format!("不支持的 HTTP 方法: {}", method)),
        };

        req = req
            .header("Authorization", format!("Bearer {}", token))
            .header("Notion-Version", NOTION_VERSION);

        if let Some(b) = body {
            req = req.json(&b);
            log::debug!("[Notion] 请求体: {}", serde_json::to_string(&b).unwrap_or_default());
        }

        let res = req.send().await.map_err(|e| format!("请求失败: {}", e))?;

        let status = res.status();
        let body: Value = res.json().await.unwrap_or_default();

        log::debug!("[Notion] 响应状态: {}", status);
        log::debug!("[Notion] 响应体: {}", serde_json::to_string(&body).unwrap_or_default());

        if !status.is_success() {
            let default_msg = format!("HTTP 错误 ({})", status);
            let msg = body.get("message").and_then(|m| m.as_str())
                .unwrap_or(&default_msg);
            return Err(format!("Notion API: {}", msg));
        }

        Ok(body)
    }

    // ── TipTap JSON → Notion Blocks 转换 ──

    fn tiptap_to_blocks(&self, tree: &Value) -> Vec<Value> {
        let mut blocks = Vec::new();
        if let Some(content) = tree.get("content").and_then(|c| c.as_array()) {
            for node in content {
                blocks.extend(self.convert_node(node));
            }
        }
        if blocks.is_empty() {
            blocks.push(json!({
                "object": "block",
                "type": "paragraph",
                "paragraph": { "rich_text": [{"type":"text","text":{"content":""}}] }
            }));
        }
        blocks
    }

    fn convert_node(&self, node: &Value) -> Vec<Value> {
        let mut out = Vec::new();
        let t = match node.get("type").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return out,
        };
        match t {
            "paragraph" => {
                out.push(json!({
                    "object": "block",
                    "type": "paragraph",
                    "paragraph": { "rich_text": self.convert_text(node) }
                }));
            }
            "heading" => {
                let level = node.get("attrs").and_then(|a| a.get("level")).and_then(|l| l.as_u64()).unwrap_or(1);
                let ht = if level == 1 { "heading_1" } else if level == 2 { "heading_2" } else { "heading_3" };
                out.push(json!({
                    "object": "block",
                    "type": ht,
                    ht: { "rich_text": self.convert_text(node) }
                }));
            }
            "bulletList" => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        out.push(json!({
                            "object": "block",
                            "type": "bulleted_list_item",
                            "bulleted_list_item": { "rich_text": self.convert_text(item) }
                        }));
                    }
                }
            }
            "orderedList" => {
                if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                    for item in items {
                        out.push(json!({
                            "object": "block",
                            "type": "numbered_list_item",
                            "numbered_list_item": { "rich_text": self.convert_text(item) }
                        }));
                    }
                }
            }
            "blockquote" => {
                if let Some(paragraphs) = node.get("content").and_then(|c| c.as_array()) {
                    for para in paragraphs {
                        out.push(json!({
                            "object": "block",
                            "type": "quote",
                            "quote": { "rich_text": self.convert_text(para) }
                        }));
                    }
                } else {
                    out.push(json!({
                        "object": "block",
                        "type": "quote",
                        "quote": { "rich_text": self.convert_text(node) }
                    }));
                }
            }
            "codeBlock" => {
                let lang = node.get("attrs")
                    .and_then(|a| a.get("language").and_then(|l| l.as_str()))
                    .unwrap_or("plain text");
                out.push(json!({
                    "object": "block",
                    "type": "code",
                    "code": {
                        "rich_text": self.convert_text(node),
                        "language": lang
                    }
                }));
            }
            "horizontalRule" => {
                out.push(json!({
                    "object": "block",
                    "type": "divider",
                    "divider": {}
                }));
            }
            _ => {}
        }
        out
    }

    fn convert_text(&self, node: &Value) -> Vec<Value> {
        let mut rt = Vec::new();
        if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
            for child in content {
                rt.extend(self.collect_text_nodes(child));
            }
        }
        if rt.is_empty() {
            rt.push(json!({"type":"text","text":{"content":""}}));
        }
        rt
    }

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

        let mut text_obj = serde_json::Map::new();
        text_obj.insert("content".into(), json!(text));
        if let Some(url) = link_url {
            text_obj.insert("link".into(), json!({ "url": url }));
        }

        let mut obj = serde_json::Map::new();
        obj.insert("type".into(), json!("text"));
        obj.insert("text".into(), Value::Object(text_obj));
        if !anno.is_empty() {
            obj.insert("annotations".into(), Value::Object(anno));
        }

        vec![Value::Object(obj)]
    }

    // ── 数据库 Schema 提取 ──

    fn extract_schema_from_properties(properties: &serde_json::Map<String, Value>) -> Result<DatabaseSchema, String> {
        let mut title_prop = String::new();
        let mut date_prop: Option<String> = None;

        for (name, prop) in properties {
            let prop_type = prop.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if prop_type == "title" && title_prop.is_empty() {
                title_prop = name.clone();
            }
            if prop_type == "date" && date_prop.is_none() {
                date_prop = Some(name.clone());
            }
        }

        if title_prop.is_empty() {
            return Err("数据库中找不到 title 类型的列".into());
        }

        Ok(DatabaseSchema { title_prop, date_prop })
    }

    // ── 目标类型判断 ──

    /// 三步试探法：确定 target_id 的类型
    async fn resolve_target(
        &self,
        token: &str,
        target_id: &str,
    ) -> Result<TargetType, String> {
        // 第一步：直接测是不是纯 Database
        let result = self.request("GET", &format!("/databases/{}", target_id), token, None).await;

        if let Ok(body) = result {
            if body.get("object").and_then(|o| o.as_str()) == Some("database") {
                if let Some(props) = body.get("properties").and_then(|p| p.as_object()) {
                    let schema = Self::extract_schema_from_properties(props)?;
                    return Ok(TargetType::Database { db_id: target_id.to_string(), schema });
                }
                return Ok(TargetType::Database {
                    db_id: target_id.to_string(),
                    schema: DatabaseSchema { title_prop: "Name".to_string(), date_prop: None },
                });
            }
        }

        // 第二步：拆开页面看有没有 child_database
        let children = self.request(
            "GET",
            &format!("/blocks/{}/children?page_size=100", target_id),
            token,
            None,
        ).await;

        if let Ok(body) = children {
            if let Some(arr) = body.get("results").and_then(|r| r.as_array()) {
                for block in arr {
                    if block.get("type").and_then(|t| t.as_str()) == Some("child_database") {
                        let db_id = block.get("id").and_then(|id| id.as_str())
                            .ok_or("child_database 缺少 id")?
                            .to_string();

                        // 获取数据库 schema
                        let schema_body = self.request("GET", &format!("/databases/{}", db_id), token, None).await?;
                        let properties = schema_body.get("properties")
                            .and_then(|p| p.as_object())
                            .ok_or("数据库 schema 中找不到 properties")?;

                        let schema = Self::extract_schema_from_properties(properties)?;
                        return Ok(TargetType::Database { db_id, schema });
                    }
                }
            }
        }

        // 第三步：兜底为普通页面
        Ok(TargetType::Page)
    }

    // ── 创建页面 ──

    async fn create_page(
        &self,
        token: &str,
        parent_id: &str,
        title: &str,
        blocks: Vec<Value>,
        is_database: bool,
        schema: Option<&DatabaseSchema>,
    ) -> Result<(String, String), String> {
        let properties = if is_database {
            let schema = schema.ok_or("数据库模式缺失")?;
            let mut props = serde_json::Map::new();
            props.insert(schema.title_prop.clone(), json!({
                "title": [{ "text": { "content": title } }]
            }));
            if let Some(date_name) = &schema.date_prop {
                let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
                props.insert(date_name.clone(), json!({
                    "date": { "start": now }
                }));
            }
            props
        } else {
            let mut props = serde_json::Map::new();
            props.insert("title".into(), json!({
                "title": [{ "text": { "content": title } }]
            }));
            props
        };

        let parent = if is_database {
            json!({ "database_id": parent_id })
        } else {
            json!({ "page_id": parent_id })
        };

        let body = json!({
            "parent": parent,
            "properties": properties,
            "children": blocks
        });

        let result = self.request("POST", "/pages", token, Some(body)).await?;

        let page_id = result.get("id").and_then(|id| id.as_str())
            .ok_or("创建页面失败：未返回页面 ID")?
            .to_string();

        let page_url = result.get("url")
            .and_then(|u| u.as_str())
            .map(|u| u.to_string())
            .unwrap_or_else(|| format!("https://notion.so/{}", page_id));

        Ok((page_id, page_url))
    }

    /// 追加内容块到页面
    async fn append_children(
        &self,
        token: &str,
        page_id: &str,
        blocks: Vec<Value>,
    ) -> Result<(), String> {
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
impl PlatformAdapter for NotionAdapter {

    async fn probe_type(&self, instance: &PlatformInstance) -> Result<String, String> {
        let target_id = resolve_target_id("notion", &instance.target_id);
        let target = self.resolve_target(&instance.token, &target_id).await?;
        match target {
            TargetType::Database { .. } => Ok("database".to_string()),
            TargetType::Page => Ok("page".to_string()),
        }
    }

    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String> {
        log::info!("[Notion] 测试连接");
        match self.request("GET", "/users/me", &instance.token, None).await {
            Ok(_) => {
                log::info!("[Notion] 测试连接成功");
                Ok(())
            }
            Err(e) => {
                log::warn!("[Notion] 测试连接失败: {}", e);
                Err(e)
            }
        }
    }

    async fn publish(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        let target_id = resolve_target_id("notion", &instance.target_id);

        // 提取标题
        let title = markdown::extract_title(content);

        // 转换内容为 blocks
        let mut blocks = self.tiptap_to_blocks(content);

        // 如果第一个 block 是 heading 且文本与 title 相同，跳过
        if let Some(first) = blocks.first() {
            let is_heading = first.get("type")
                .and_then(|t| t.as_str())
                .map(|t| t.starts_with("heading_"))
                .unwrap_or(false);
            if is_heading {
                let heading_type = first.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let first_text = first.get(heading_type)
                    .and_then(|h| h.get("rich_text"))
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

        // 判断目标类型
        let target = self.resolve_target(&instance.token, &target_id).await?;

        let (parent_id, is_database, schema) = match &target {
            TargetType::Database { db_id, schema } => (db_id.clone(), true, Some(schema)),
            TargetType::Page => (target_id.clone(), false, None),
        };

        // 创建页面
        let (page_id, page_url) = self.create_page(
            &instance.token,
            &parent_id,
            &title,
            blocks,
            is_database,
            schema,
        ).await?;

        log::info!("[Notion] 创建页面成功: {}", page_id);

        Ok(PublishResult {
            success: true,
            message: "发送成功".into(),
            url: Some(page_url),
        })
    }

    async fn append_blocks(&self, content: &Value, instance: &PlatformInstance) -> Result<PublishResult, String> {
        let target_id = resolve_target_id("notion", &instance.target_id);

        // 构建追加 blocks：分隔线 + 正文
        let mut children: Vec<Value> = Vec::new();

        // 分隔线
        children.push(json!({
            "object": "block",
            "type": "divider",
            "divider": {}
        }));

        // 正文
        children.extend(self.tiptap_to_blocks(content));

        // 追加
        self.append_children(&instance.token, &target_id, children).await?;

        log::info!("[Notion] 追加内容成功");

        Ok(PublishResult {
            success: true,
            message: "追加成功".into(),
            url: Some(format!("https://notion.so/{}", target_id)),
        })
    }
}