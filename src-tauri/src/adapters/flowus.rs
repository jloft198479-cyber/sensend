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
        let blocks: Vec<Value> = super::ir::parse(tree)
            .iter()
            .flat_map(|b| self.map_block(b))
            .collect();
        if blocks.is_empty() {
            return vec![json!({
                "type": "paragraph",
                "data": { "rich_text": [{"type":"text","text":{"content":"","link":null}}] }
            })];
        }
        blocks
    }

    /// IR 块 → FlowUs block(s)
    fn map_block(&self, block: &super::ir::Block) -> Vec<Value> {
        use super::ir::Block;
        let mut out = Vec::new();
        match block {
            Block::Paragraph(inlines) => {
                out.push(self.text_block("paragraph", inlines));
            }
            Block::Heading { level, inlines } => {
                let ht = match level {
                    1 => "heading_1",
                    2 => "heading_2",
                    _ => "heading_3",
                };
                out.push(self.text_block(ht, inlines));
            }
            Block::List { kind, items } => {
                for item in items {
                    out.push(self.map_list_item(*kind, item));
                }
            }
            Block::CodeBlock { language, code } => {
                let lang = if language.is_empty() { "plain text" } else { language.as_str() };
                out.push(json!({
                    "type": "code",
                    "data": {
                        "rich_text": [make_rich_element(code, &[])],
                        "language": lang
                    }
                }));
            }
            Block::BlockQuote(paras) => {
                for para in paras {
                    out.push(self.text_block("quote", para));
                }
            }
            Block::Table(table) => {
                let col_count = table.rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1);
                for row in &table.rows {
                    let mut row_cells: Vec<Value> = Vec::new();
                    for cell in row {
                        // 单元格保留行内格式（修复 #5）
                        let rt = map_rich_text(cell);
                        row_cells.push(if rt.is_empty() {
                            json!({"type": "text", "text": {"content": "", "link": null}})
                        } else {
                            rt[0].clone()
                        });
                    }
                    while row_cells.len() < col_count {
                        row_cells.push(json!({"type": "text", "text": {"content": "", "link": null }}));
                    }
                    out.push(json!({
                        "type": "table_row",
                        "data": { "cells": row_cells }
                    }));
                }
            }
            Block::HorizontalRule => {
                out.push(json!({
                    "type": "divider",
                    "data": {}
                }));
            }
        }
        out
    }

    /// 带颜色默认值的文本块（paragraph / heading / quote）
    fn text_block(&self, block_type: &str, inlines: &[super::ir::Inline]) -> Value {
        let mut rt = map_rich_text(inlines);
        if rt.is_empty() {
            rt.push(json!({"type":"text","text":{"content":"","link":null}}));
        }
        json!({
            "type": block_type,
            "data": {
                "rich_text": rt,
                "text_color": "default",
                "background_color": "default"
            }
        })
    }

    /// 列表项 → FlowUs block（嵌套子列表进 children，修复 #2；真机验证点）
    fn map_list_item(&self, kind: super::ir::ListKind, item: &super::ir::ListItem) -> Value {
        let block_type = match kind {
            super::ir::ListKind::Bullet => "bulleted_list_item",
            super::ir::ListKind::Ordered => "numbered_list_item",
            super::ir::ListKind::Task => "todo",
        };

        let mut rt = map_rich_text(&item.inlines);
        for para in &item.extra_paras {
            rt.push(json!({"type": "text", "text": {"content": "\n", "link": null}}));
            rt.extend(map_rich_text(para));
        }
        if rt.is_empty() {
            rt.push(json!({"type":"text","text":{"content":"","link":null}}));
        }

        let mut data = serde_json::Map::new();
        data.insert("rich_text".into(), json!(rt));
        if kind == super::ir::ListKind::Task {
            data.insert("checked".into(), json!(item.checked.unwrap_or(false)));
        }
        data.insert("text_color".into(), json!("default"));
        data.insert("background_color".into(), json!("default"));

        // 嵌套子列表（FlowUs 嵌套结构真机验证点；若不支持改为拍平输出）
        let children: Vec<Value> = item
            .children
            .iter()
            .filter_map(|c| match c {
                super::ir::Block::List { items: sub_items, kind: sub_kind } => {
                    Some(sub_items.iter().map(|it| self.map_list_item(*sub_kind, it)).collect::<Vec<Value>>())
                }
                _ => None,
            })
            .flatten()
            .collect();
        if !children.is_empty() {
            data.insert("children".into(), json!(children));
        }

        json!({
            "type": block_type,
            "data": Value::Object(data)
        })
    }

    // ── 目标类型判断 ──

    /// 判断目标 ID 是数据库还是页面
    /// 仅当目标不存在（404）时降级为 Page，其他错误（401/403/网络等）向上传递
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
            Err(e) => {
                // 仅当 404（目标不存在）时降级为 Page
                // 其他错误（401 认证失败、403 无权限、网络错误等）必须向上传递
                if e.contains("404") || e.to_lowercase().contains("not found") {
                    Ok(TargetType::Page)
                } else {
                    Err(e)
                }
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
        let title = markdown::extract_title_full(content);
        
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

/// IR 行内内容 → FlowUs rich_text 数组（annotations 全字段 + plain_text + href）
/// hardBreak → 换行文本（修复 #1）；mention → "@标签" 文本（防数据丢失）
fn map_rich_text(inlines: &[super::ir::Inline]) -> Vec<Value> {
    use super::ir::Inline;
    let mut rt = Vec::new();
    for inline in inlines {
        let (text, marks) = match inline {
            Inline::Text { text, marks } => (text.as_str(), marks),
            Inline::Break => ("\n", &Vec::new()),
            Inline::Mention(label) => {
                rt.push(make_rich_element(&format!("@{}", label), &[]));
                continue;
            }
        };
        rt.push(make_rich_element(text, marks));
    }
    rt
}

/// 构造单个 FlowUs rich_text 元素
fn make_rich_element(text: &str, marks: &[super::ir::Mark]) -> Value {
    use super::ir::Mark;
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

    let mut text_obj = serde_json::Map::new();
    text_obj.insert("content".into(), json!(text));
    match link_url {
        Some(ref url) => text_obj.insert("link".into(), json!({ "url": url })),
        None => text_obj.insert("link".into(), Value::Null),
    };

    let mut full_anno = serde_json::Map::new();
    full_anno.insert("bold".into(), json!(anno.contains_key("bold")));
    full_anno.insert("italic".into(), json!(anno.contains_key("italic")));
    full_anno.insert("strikethrough".into(), json!(anno.contains_key("strikethrough")));
    full_anno.insert("underline".into(), json!(anno.contains_key("underline")));
    full_anno.insert("code".into(), json!(anno.contains_key("code")));
    full_anno.insert("color".into(), json!("default"));

    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!("text"));
    obj.insert("text".into(), Value::Object(text_obj));
    obj.insert("annotations".into(), Value::Object(full_anno));
    obj.insert("plain_text".into(), json!(text));
    obj.insert("href".into(), match link_url {
        Some(url) => json!(url),
        None => Value::Null,
    });
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers;

    fn convert(tree: &Value) -> Vec<Value> {
        FlowUsAdapter::new().tiptap_to_blocks(tree)
    }

    macro_rules! golden_tests {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let fixture = test_helpers::load_fixture(stringify!($name));
                    let output = convert(&fixture);
                    test_helpers::assert_or_update_golden("flowus", stringify!($name), "json", &test_helpers::format_json(&serde_json::Value::Array(output)));
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
        let rich = blocks[0]["data"]["rich_text"].as_array().unwrap();
        let content: String = rich.iter()
            .filter_map(|t| t["text"]["content"].as_str())
            .collect();
        assert_eq!(content, "第一行\n第二行\n第三行");
    }

    #[test]
    fn fix2_nested_list_children() {
        let fixture = test_helpers::load_fixture("nested_list");
        let blocks = convert(&fixture);
        let children = blocks[0]["data"]["children"].as_array().expect("嵌套子列表应进 children");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn fix5_table_cell_keeps_annotations() {
        let fixture = test_helpers::load_fixture("table_with_inline");
        let blocks = convert(&fixture);
        let cell = &blocks[0]["data"]["cells"][0];
        assert_eq!(cell["annotations"]["bold"], json!(true), "表头第一格粗体应保留");
    }
}