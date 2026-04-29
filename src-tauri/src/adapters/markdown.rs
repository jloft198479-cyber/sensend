//! 公共 Markdown 转换模块
//! 将 TipTap JSON 树转换为 Markdown 文本，供 local.rs / flowus.rs 等适配器复用。

use serde_json::Value;

/// TipTap JSON → Markdown 文本
pub fn tiptap_to_markdown(tree: &Value) -> String {
    let mut md = String::new();
    if let Some(children) = tree.get("content").and_then(|c| c.as_array()) {
        for node in children {
            render_node(node, &mut md, 0);
        }
    }
    let trimmed = md.trim_end().to_string();
    if trimmed.is_empty() {
        "(空笔记)".to_string()
    } else {
        trimmed
    }
}

/// 提取文档标题（取第一个非空段落前 18 字）
pub fn extract_title(content: &Value) -> String {
    if let Some(children) = content.get("content").and_then(|c| c.as_array()) {
        // 优先取第一个 heading
        for node in children {
            if node.get("type").and_then(|t| t.as_str()) == Some("heading") {
                if let Some(text) = extract_plain_text(node) {
                    if !text.is_empty() {
                        return text.chars().take(18).collect();
                    }
                }
            }
        }
        // 兜底：取第一个非空段落
        for node in children {
            if let Some(text) = extract_plain_text(node) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return trimmed.chars().take(18).collect();
                }
            }
        }
    }
    "Sensend 笔记".to_string()
}

/// 提取纯文本（忽略格式，用于标题等场景）
pub fn extract_plain_text(node: &Value) -> Option<String> {
    let mut text = String::new();
    if let Some(t) = node.get("text").and_then(|t| t.as_str()) {
        text.push_str(t);
    }
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            if let Some(t) = extract_plain_text(child) {
                text.push_str(&t);
            }
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

// ── 内部实现 ──

fn render_node(node: &Value, out: &mut String, list_depth: usize) {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match node_type {
        "paragraph" => {
            render_inline(node, out);
            out.push('\n');
            // 列表项内的段落不双换行；顶层段落双换行
            if list_depth == 0 {
                out.push('\n');
            }
        }
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(1);
            out.push_str(&"#".repeat(level as usize));
            out.push(' ');
            render_inline(node, out);
            out.push_str("\n\n");
        }
        "bulletList" => render_list(node, out, list_depth, ListKind::Bullet),
        "orderedList" => render_list(node, out, list_depth, ListKind::Ordered),
        "codeBlock" => {
            let lang = node
                .get("attrs")
                .and_then(|a| a.get("language").and_then(|l| l.as_str()))
                .unwrap_or("");
            out.push_str(&format!("```{}\n", lang));
            render_text(node, out);
            out.push_str("```\n\n");
        }
        "blockquote" => {
            if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
                for child in children {
                    for line in node_to_inline_text(child).lines() {
                        out.push_str(&format!("> {}\n", line));
                    }
                }
            }
            out.push('\n');
        }
        "horizontalRule" => {
            out.push_str("---\n\n");
        }
        "hardBreak" => {
            out.push_str("  \n");
        }
        _ => {
            // 兜底：递归处理子节点
            if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
                for child in children {
                    render_node(child, out, list_depth);
                }
            }
        }
    }
}

/// 列表类型
enum ListKind {
    Bullet,
    Ordered,
}

/// 渲染列表（含嵌套）
fn render_list(node: &Value, out: &mut String, list_depth: usize, kind: ListKind) {
    let indent = "  ".repeat(list_depth);
    if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
        for (i, item) in items.iter().enumerate() {
            let marker = match kind {
                ListKind::Bullet => "- ".to_string(),
                ListKind::Ordered => format!("{}. ", i + 1),
            };
            out.push_str(&indent);
            out.push_str(&marker);

            if let Some(children) = item.get("content").and_then(|c| c.as_array()) {
                for (ci, child) in children.iter().enumerate() {
                    let child_type = child.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    // 嵌套列表：递归，深度 +1
                    if child_type == "bulletList" || child_type == "orderedList" {
                        out.push('\n');
                        render_list(
                            child,
                            out,
                            list_depth + 1,
                            if child_type == "bulletList" {
                                ListKind::Bullet
                            } else {
                                ListKind::Ordered
                            },
                        );
                    } else if ci == 0 {
                        // 列表项第一个子节点（通常是 paragraph）：内联输出紧跟 marker
                        render_inline(child, out);
                        out.push('\n');
                    } else {
                        // 列表项内后续段落：换行 + 缩进对齐
                        out.push_str(&indent);
                        out.push_str("  ");
                        render_inline(child, out);
                        out.push('\n');
                    }
                }
            }
        }
    }
    // 顶层列表后空一行，嵌套列表不额外空行
    if list_depth == 0 {
        out.push('\n');
    }
}

/// 提取内联文本（含 mark 格式：粗体、斜体、删除线、代码、链接）
fn render_inline(node: &Value, out: &mut String) {
    if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
        let mut s = text.to_string();
        if let Some(marks) = node.get("marks").and_then(|m| m.as_array()) {
            for mark in marks {
                let mark_type = mark.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match mark_type {
                    "bold" => {
                        s = format!("**{}**", s);
                    }
                    "italic" => {
                        s = format!("*{}*", s);
                    }
                    "strike" => {
                        s = format!("~~{}~~", s);
                    }
                    "code" => {
                        s = format!("`{}`", s);
                    }
                    "link" => {
                        if let Some(href) = mark
                            .get("attrs")
                            .and_then(|a| a.get("href"))
                            .and_then(|h| h.as_str())
                        {
                            return out.push_str(&format!("[{}]({})", s, href));
                        }
                    }
                    _ => {}
                }
            }
        }
        out.push_str(&s);
    } else if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            // mention 节点输出为 @名称
            if child.get("type").and_then(|t| t.as_str()) == Some("mention") {
                if let Some(label) = child
                    .get("attrs")
                    .and_then(|a| a.get("label"))
                    .and_then(|l| l.as_str())
                {
                    out.push_str(&format!("@{}", label));
                }
            } else {
                render_inline(child, out);
            }
        }
    }
}

/// 提取纯文本（忽略所有格式）
fn render_text(node: &Value, out: &mut String) {
    if let Some(text) = node.get("text").and_then(|t| t.as_str()) {
        out.push_str(text);
    }
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            render_text(child, out);
        }
    }
}

/// 节点 → 纯文本（用于 blockquote 等需要逐行引用的场景）
fn node_to_inline_text(node: &Value) -> String {
    let mut s = String::new();
    render_inline(node, &mut s);
    s
}