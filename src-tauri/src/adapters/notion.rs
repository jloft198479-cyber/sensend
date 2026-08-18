use super::{markdown, resolve_target_id, PlatformAdapter, PlatformInstance, PublishResult};
use async_trait::async_trait;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

// ── Notion 代码块语言映射（common aliases → Notion 官方 language 枚举值）──
// 未知语言回落 "plain text"（官方枚举成员，表示无高亮纯文本）
const NOTION_LANG_MAP: &[(&str, &str)] = &[
    ("plaintext", "plain text"), ("text", "plain text"), ("txt", "plain text"),
    ("bash", "bash"), ("sh", "shell"), ("shell", "shell"), ("zsh", "shell"), ("console", "shell"),
    ("csharp", "c#"), ("cs", "c#"), ("c#", "c#"),
    ("cpp", "c++"), ("c++", "c++"), ("cxx", "c++"),
    ("c", "c"),
    ("css", "css"), ("scss", "css"), ("less", "css"),
    ("dockerfile", "docker"),
    ("go", "go"), ("golang", "go"),
    ("html", "html"), ("xml", "xml"), ("svg", "xml"),
    ("json", "json"),
    ("java", "java"),
    ("javascript", "javascript"), ("js", "javascript"),
    ("jsx", "javascript"),
    ("kotlin", "kotlin"), ("kt", "kotlin"),
    ("lua", "lua"),
    ("markdown", "markdown"), ("md", "markdown"),
    ("php", "php"),
    ("perl", "perl"),
    ("python", "python"), ("py", "python"),
    ("ruby", "ruby"), ("rb", "ruby"),
    ("rust", "rust"), ("rs", "rust"),
    ("scala", "scala"),
    ("swift", "swift"),
    ("typescript", "typescript"), ("ts", "typescript"), ("tsx", "typescript"),
    ("yaml", "yaml"), ("yml", "yaml"),
    ("sql", "sql"),
    ("toml", "toml"), ("ini", "toml"),
];

/// 编辑器语言字符串 → Notion 官方 language 枚举值，未知回落 "plain text"
fn map_language(lang: &str) -> &str {
    let lower = lang.to_lowercase();
    NOTION_LANG_MAP
        .iter()
        .find(|(k, _)| *k == lower)
        .map(|(_, v)| *v)
        .unwrap_or("plain text")
}

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
            // 错误消息统一带状态码：resolve_target 依赖它区分 404 与网络错误
            return Err(format!("Notion API ({}): {}", status, msg));
        }

        Ok(body)
    }

    // ── IR → Notion Blocks 转换 ──

    fn tiptap_to_blocks(&self, tree: &Value) -> Vec<Value> {
        let blocks: Vec<Value> = super::ir::parse(tree)
            .iter()
            .flat_map(|b| self.map_block(b))
            .collect();
        if blocks.is_empty() {
            return vec![json!({
                "object": "block",
                "type": "paragraph",
                "paragraph": { "rich_text": [{"type":"text","text":{"content":""}}] }
            })];
        }
        blocks
    }

    /// IR 块 → Notion block(s)
    fn map_block(&self, block: &super::ir::Block) -> Vec<Value> {
        use super::ir::Block;
        let mut out = Vec::new();
        match block {
            Block::Paragraph(inlines) => {
                out.push(json!({
                    "object": "block",
                    "type": "paragraph",
                    "paragraph": { "rich_text": map_rich_text(inlines) }
                }));
            }
            Block::Heading { level, inlines } => {
                let ht = match level {
                    1 => "heading_1",
                    2 => "heading_2",
                    _ => "heading_3",
                };
                out.push(json!({
                    "object": "block",
                    "type": ht,
                    ht: { "rich_text": map_rich_text(inlines) }
                }));
            }
            Block::List { kind, items } => {
                for item in items {
                    out.push(self.map_list_item(*kind, item, 1));
                }
            }
            Block::CodeBlock { language, code } => {
                out.push(json!({
                    "object": "block",
                    "type": "code",
                    "code": {
                        "rich_text": [{ "type": "text", "text": { "content": code } }],
                        "language": map_language(language)
                    }
                }));
            }
            Block::BlockQuote(paras) => {
                for para in paras {
                    out.push(json!({
                        "object": "block",
                        "type": "quote",
                        "quote": { "rich_text": map_rich_text(para) }
                    }));
                }
            }
            Block::Table(table) => {
                // 列数取各行最大值
                let col_count = table.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                // 官方结构：table 的 children 必须是 table_row 块数组；
                // 每个 table_row 的 cells 是二维 rich_text 数组——
                // 每格本身是一个 rich_text 数组（可含多段格式），不再只取 rt[0]
                let mut table_row_blocks: Vec<Value> = Vec::new();
                for row in &table.rows {
                    let mut cells: Vec<Value> = Vec::new();
                    for cell in row {
                        let rt = map_rich_text(cell);
                        cells.push(json!(rt));
                    }
                    while cells.len() < col_count {
                        cells.push(json!([{ "type": "text", "text": { "content": "" } }]));
                    }
                    table_row_blocks.push(json!({
                        "object": "block",
                        "type": "table_row",
                        "table_row": { "cells": cells }
                    }));
                }
                out.push(json!({
                    "object": "block",
                    "type": "table",
                    "table": {
                        "table_width": col_count,
                        "has_column_header": false,
                        "has_row_header": false,
                        "children": table_row_blocks
                    }
                }));
            }
            Block::HorizontalRule => {
                out.push(json!({
                    "object": "block",
                    "type": "divider",
                    "divider": {}
                }));
            }
        }
        out
    }

    /// 列表项 → Notion block（嵌套子列表进 children，修复 #2）
    /// depth 从 1 计：Notion 单请求 children 嵌套上限 2 层，第 3 层起子项文本
    /// 以 \n 拼进父项 rich_text（保文本，层级降级），避免 API validation_error
    fn map_list_item(&self, kind: super::ir::ListKind, item: &super::ir::ListItem, depth: usize) -> Value {
        let (block_type, checked) = match kind {
            super::ir::ListKind::Bullet => ("bulleted_list_item", None),
            super::ir::ListKind::Ordered => ("numbered_list_item", None),
            super::ir::ListKind::Task => ("to_do", item.checked),
        };

        // rich_text：首段 + 后续段落（\n 分隔，修复列表项多段落丢失）
        // 空兜底已在 map_rich_text 内部统一处理
        let mut rt = map_rich_text(&item.inlines);
        for para in &item.extra_paras {
            rt.push(json!({"type": "text", "text": {"content": "\n"}}));
            rt.extend(map_rich_text(para));
        }

        let mut data = serde_json::Map::new();
        data.insert("rich_text".into(), json!(rt));
        if let Some(checked) = checked {
            data.insert("checked".into(), json!(checked));
        }

        let can_nest = depth < 2;
        let mut overflow_texts: Vec<String> = Vec::new();

        let children: Vec<Value> = item
            .children
            .iter()
            .filter_map(|c| match c {
                super::ir::Block::List { items: sub_items, kind: sub_kind } => {
                    Some(sub_items.iter().map(|it| {
                        if can_nest {
                            self.map_list_item(*sub_kind, it, depth + 1)
                        } else {
                            // 超过 2 层：收集文本稍后拼进父项（collect_list_texts 复用多段逻辑）
                            collect_list_texts(*sub_kind, it, &mut overflow_texts);
                            Value::Null
                        }
                    }).collect::<Vec<Value>>())
                }
                _ => None,
            })
            .flatten()
            .filter(|v| !v.is_null())
            .collect();
        if !children.is_empty() {
            data.insert("children".into(), json!(children));
        }
        // 溢出文本拼进父项 rich_text（缩进保留层级语义）
        for text in overflow_texts {
            let rt_arr = data.get_mut("rich_text").and_then(|v| v.as_array_mut());
            if let Some(arr) = rt_arr {
                arr.push(json!({"type": "text", "text": {"content": format!("\n  {}", text)}}));
            }
        }

        json!({
            "object": "block",
            "type": block_type,
            block_type: Value::Object(data)
        })
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
    /// children 探测失败时不能静默降级为 Page——弱网下多维表目标会被误判为页面，
    /// 内容作为独立文章追加到容器页末尾（多维表下方），且全程"发送成功"无报错。
    /// 仅当目标确实不存在（404）时才降级 Page，其他错误（超时/401/403）向上传递。
    async fn resolve_target(
        &self,
        token: &str,
        target_id: &str,
    ) -> Result<TargetType, String> {
        // 两个独立探测并行发起：database 直查 + 页面 children 拆查
        // 用 join! 而非 try_join! —— db 查询对页面目标返回 400 是预期内，只需看 children 结果
        let db_url = format!("/databases/{}", target_id);
        let children_url = format!("/blocks/{}/children?page_size=100", target_id);
        let (db_res, children_res) = tokio::join!(
            self.request("GET", &db_url, token, None),
            self.request("GET", &children_url, token, None),
        );

        // 优先判：目标是纯 Database
        if let Ok(body) = db_res {
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

        // 再判：页面内嵌 child_database（children 是类型判断的关键探测，失败必须区分原因）
        match children_res {
            Ok(body) => {
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
                // children 拉取成功但无 child_database → 真普通页面
                Ok(TargetType::Page)
            }
            Err(e) => {
                // 仅当目标不存在（404）时降级为 Page；网络/权限错误必须报错
                if e.contains("404") || e.to_lowercase().contains("not found") {
                    Ok(TargetType::Page)
                } else {
                    Err(format!("无法确认目标类型（网络或权限异常），已阻止发送以防内容写错位置：{}", e))
                }
            }
        }
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

        // Notion 创建页面时 children 最多 100 个，超出部分需要后续追加
        let (initial_blocks, rest_blocks) = if blocks.len() > 100 {
            let rest = blocks[100..].to_vec();
            (blocks[..100].to_vec(), Some(rest))
        } else {
            (blocks, None)
        };

        let body = json!({
            "parent": parent,
            "properties": properties,
            "children": initial_blocks
        });

        let result = self.request("POST", "/pages", token, Some(body)).await?;

        let page_id = result.get("id").and_then(|id| id.as_str())
            .ok_or("创建页面失败：未返回页面 ID")?
            .to_string();

        // 追加剩余的 blocks（页面已创建，追加失败时不能丢失这一信息）
        if let Some(rest) = rest_blocks {
            let rest_count = rest.len();
            if let Err(e) = self.append_children(token, &page_id, rest).await {
                let page_url = result.get("url")
                    .and_then(|u| u.as_str())
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| format!("https://notion.so/{}", page_id));
                return Err(format!(
                    "页面已创建（{}），但部分内容（{} 个块）追加失败：{}\n请手动打开页面检查。",
                    page_url, rest_count, e
                ));
            }
        }

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

        // 提取标题（全量文本，超 18 字去重不再失效）
        let title = markdown::extract_title_full(content);

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

/// 超深子列表 → 纯文本收集（Notion 嵌套上限 2 层，溢出文本拼进父项 rich_text 防丢失）
/// 前缀保留层级语义：无序 - / 有序 1. / 待办 [x]
fn collect_list_texts(kind: super::ir::ListKind, item: &super::ir::ListItem, out: &mut Vec<String>) {
    use super::ir::{Inline, ListKind};

    let text = item.inlines.iter()
        .chain(item.extra_paras.iter().flatten())
        .map(|inline| match inline {
            Inline::Text { text, .. } => text.clone(),
            Inline::Break => "\n".to_string(),
            Inline::Mention(label) => format!("@{}", label),
        })
        .collect::<String>();
    let prefix = match kind {
        ListKind::Bullet => "- ",
        ListKind::Ordered => "1. ",
        ListKind::Task => {
            if item.checked.unwrap_or(false) { "[x] " } else { "[ ] " }
        }
    };
    let line = if text.trim().is_empty() { String::new() } else { format!("{}{}", prefix, text) };
    if !line.is_empty() {
        out.push(line);
    }
    for child in &item.children {
        if let super::ir::Block::List { items: sub_items, kind: sub_kind } = child {
            for sub in sub_items {
                collect_list_texts(*sub_kind, sub, out);
            }
        }
    }
}

/// IR 行内内容 → Notion rich_text 数组
/// hardBreak → 换行文本（修复 #1）；mention → "@标签" 文本（防数据丢失）；
/// text.content 上限 2000 字符（Notion API 限制），超长按字符切片，每片携带相同的 anno/link；
/// 空行内内容兜底返回一个空 text 对象（Notion rich_text 数组不能为空）
fn map_rich_text(inlines: &[super::ir::Inline]) -> Vec<Value> {
    use super::ir::{Inline, Mark};
    let mut rt = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, marks } => {
                let mut anno = serde_json::Map::new();
                let mut link_url: Option<String> = None;
                for mark in marks {
                    match mark {
                        Mark::Bold => { anno.insert("bold".into(), json!(true)); }
                        Mark::Italic => { anno.insert("italic".into(), json!(true)); }
                        Mark::Strike => { anno.insert("strikethrough".into(), json!(true)); }
                        Mark::Underline => { anno.insert("underline".into(), json!(true)); }
                        Mark::Code => { anno.insert("code".into(), json!(true)); }
                        Mark::Link(href) => link_url = Some(href.clone()),
                    }
                }

                // Notion text.content 上限 2000 字符，按 Unicode scalar 切片
                let chars: Vec<char> = text.chars().collect();
                for chunk in chars.chunks(2000) {
                    let content: String = chunk.iter().collect();
                    let mut text_obj = serde_json::Map::new();
                    text_obj.insert("content".into(), json!(content));
                    if let Some(url) = &link_url {
                        text_obj.insert("link".into(), json!({ "url": url }));
                    }
                    let mut obj = serde_json::Map::new();
                    obj.insert("type".into(), json!("text"));
                    obj.insert("text".into(), Value::Object(text_obj));
                    if !anno.is_empty() {
                        obj.insert("annotations".into(), Value::Object(anno.clone()));
                    }
                    rt.push(Value::Object(obj));
                }
            }
            Inline::Break => {
                rt.push(json!({ "type": "text", "text": { "content": "\n" } }));
            }
            Inline::Mention(label) => {
                rt.push(json!({ "type": "text", "text": { "content": format!("@{}", label) } }));
            }
        }
    }
    // 空兜底：空段落/空标题/空列表项也保证 rich_text 数组非空
    if rt.is_empty() {
        rt.push(json!({ "type": "text", "text": { "content": "" } }));
    }
    rt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers;

    fn convert(tree: &Value) -> Vec<Value> {
        NotionAdapter::new().tiptap_to_blocks(tree)
    }

    macro_rules! golden_tests {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let fixture = test_helpers::load_fixture(stringify!($name));
                    let output = convert(&fixture);
                    test_helpers::assert_or_update_golden("notion", stringify!($name), "json", &test_helpers::format_json(&serde_json::Value::Array(output)));
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
    fn fix1_hardbreak_becomes_newline() {
        let fixture = test_helpers::load_fixture("hardbreak");
        let blocks = convert(&fixture);
        let rich = blocks[0]["paragraph"]["rich_text"].as_array().unwrap();
        let content: String = rich.iter()
            .filter_map(|t| t.get("text").and_then(|t| t.get("content")).and_then(|c| c.as_str()))
            .collect();
        assert_eq!(content, "第一行\n第二行\n第三行");
    }

    #[test]
    fn fix2_nested_list_children() {
        let fixture = test_helpers::load_fixture("nested_list");
        let blocks = convert(&fixture);
        let first = &blocks[0]["bulleted_list_item"];
        let children = first["children"].as_array().expect("嵌套子列表应进 children");
        assert_eq!(children.len(), 2, "顶层项 1 应有 2 个嵌套子项");
        // 无嵌套的项不应有 children 字段
        assert!(blocks[1]["bulleted_list_item"].get("children").is_none());
    }

    #[test]
    fn fix5_table_cell_keeps_annotations() {
        let fixture = test_helpers::load_fixture("table_with_inline");
        let blocks = convert(&fixture);
        // 官方结构：table.children → table_row 块 → table_row.cells → 二维 rich_text 数组
        let row0_cells = &blocks[0]["table"]["children"][0]["table_row"]["cells"];
        let cell = &row0_cells[0][0];
        assert_eq!(cell["annotations"]["bold"], json!(true), "表头第一格粗体应保留");
        let row1_cells = &blocks[0]["table"]["children"][1]["table_row"]["cells"];
        let cell2 = &row1_cells[1][0];
        assert_eq!(cell2["annotations"]["italic"], json!(true), "数据行斜体应保留");
    }

    #[test]
    fn fix7_long_title_dedup_full_text() {
        let fixture = test_helpers::load_fixture("long_title");
        let title = markdown::extract_title_full(&fixture);
        assert_eq!(title, "这是一个超过十八个字的标题文字啊");
        // 首块 heading 文本应与全量标题一致（publish 去重条件成立）
        let blocks = convert(&fixture);
        let first_text: String = blocks[0]["heading_1"]["rich_text"].as_array().unwrap().iter()
            .filter_map(|t| t.get("text").and_then(|t| t.get("content")).and_then(|c| c.as_str()))
            .collect();
        assert_eq!(first_text, title);
    }

    #[test]
    fn fix_p1_language_mapping() {
        // 别名 → Notion 官方枚举
        assert_eq!(map_language("plaintext"), "plain text");
        assert_eq!(map_language("text"), "plain text");
        assert_eq!(map_language("cpp"), "c++");
        assert_eq!(map_language("CSharp"), "c#");
        assert_eq!(map_language("js"), "javascript");
        assert_eq!(map_language("ts"), "typescript");
        assert_eq!(map_language("py"), "python");
        assert_eq!(map_language("rs"), "rust");
        assert_eq!(map_language("sh"), "shell");
        // 本身即枚举值的直传
        assert_eq!(map_language("rust"), "rust");
        assert_eq!(map_language("python"), "python");
        // 未知/空 → plain text
        assert_eq!(map_language("brainfuck"), "plain text");
        assert_eq!(map_language(""), "plain text");
    }

    #[test]
    fn fix_p1_list_depth_capped_at_two() {
        // 3 层嵌套：第 3 层文本应拼进第 2 层 rich_text，不再产生孙 children
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                {
                    "type": "bulletList",
                    "content": [
                        {
                            "type": "listItem",
                            "content": [
                                { "type": "paragraph", "content": [{ "type": "text", "text": "L1" }] },
                                {
                                    "type": "bulletList",
                                    "content": [
                                        {
                                            "type": "listItem",
                                            "content": [
                                                { "type": "paragraph", "content": [{ "type": "text", "text": "L2" }] },
                                                {
                                                    "type": "bulletList",
                                                    "content": [
                                                        {
                                                            "type": "listItem",
                                                            "content": [
                                                                { "type": "paragraph", "content": [{ "type": "text", "text": "L3" }] }
                                                            ]
                                                        }
                                                    ]
                                                }
                                            ]
                                        }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            ]
        });
        let blocks = convert(&doc);
        let l1 = &blocks[0];
        let l2 = &l1["bulleted_list_item"]["children"][0];
        let l2_obj = &l2["bulleted_list_item"];
        // L2 的 rich_text 应含 L3 溢出文本
        let l2_text: String = l2_obj["rich_text"].as_array().unwrap().iter()
            .filter_map(|t| t["text"]["content"].as_str())
            .collect();
        assert!(l2_text.contains("L3"), "第 3 层文本应拼进第 2 层: {}", l2_text);
        // L2 不应有孙 children（Notion 上限 2 层）
        let has_children = l2_obj.get("children").is_some();
        assert!(!has_children, "第 2 层不应再有 children");
    }
}