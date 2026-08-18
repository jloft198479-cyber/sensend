//! 公共 Markdown 转换模块
//! IR → Markdown 文本，供 local.rs 等适配器复用。
//! TipTap 解析统一走 ir::parse（唯一遍历点）。

use super::ir::{self, Block, Inline, ListKind, Mark, Table};
use serde_json::Value;

/// TipTap JSON → Markdown 文本
pub fn tiptap_to_markdown(tree: &Value) -> String {
    let blocks = ir::parse(tree);
    let mut md = String::new();
    for block in &blocks {
        render_block(block, &mut md, 0);
    }
    let trimmed = md.trim_end().to_string();
    if trimmed.is_empty() {
        "(空笔记)".to_string()
    } else {
        trimmed
    }
}

/// 提取文档标题（取第一个非空段落前 18 字，用于本地文件名）
pub fn extract_title(content: &Value) -> String {
    let full = extract_title_full(content);
    full.chars().take(18).collect()
}

/// 提取文档标题全量文本（不截断，用于平台页面标题与去重比较）
pub fn extract_title_full(content: &Value) -> String {
    if let Some(children) = content.get("content").and_then(|c| c.as_array()) {
        // 优先取第一个 heading
        for node in children {
            if node.get("type").and_then(|t| t.as_str()) == Some("heading") {
                if let Some(text) = extract_plain_text(node) {
                    if !text.is_empty() {
                        return text;
                    }
                }
            }
        }
        // 兜底：取第一个非空段落
        for node in children {
            if let Some(text) = extract_plain_text(node) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
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

// ── IR → Markdown 渲染 ──

/// hardBreak 的渲染方式（不同上下文行为不同）
enum BreakMode {
    /// 段落内：渲染为 Markdown 硬换行 "  \n"
    Line,
    /// 表格单元格/引用内：丢弃（与历史行为一致）
    Suppress,
}

fn render_block(block: &Block, out: &mut String, list_depth: usize) {
    match block {
        Block::Paragraph(inlines) => {
            render_inlines(inlines, out, BreakMode::Line);
            out.push('\n');
            // 列表项内的段落不双换行；顶层段落双换行
            if list_depth == 0 {
                out.push('\n');
            }
        }
        Block::Heading { level, inlines } => {
            out.push_str(&"#".repeat(*level as usize));
            out.push(' ');
            render_inlines(inlines, out, BreakMode::Line);
            out.push_str("\n\n");
        }
        Block::List { kind, items } => render_list(items, *kind, out, list_depth),
        Block::CodeBlock { language, code } => {
            out.push_str(&format!("```{}\n", language));
            out.push_str(code);
            out.push_str("```\n\n");
        }
        Block::BlockQuote(paras) => {
            for para in paras {
                let mut line = String::new();
                render_inlines(para, &mut line, BreakMode::Suppress);
                for l in line.lines() {
                    out.push_str(&format!("> {}\n", l));
                }
            }
            out.push('\n');
        }
        Block::Table(table) => render_table(table, out),
        Block::HorizontalRule => {
            out.push_str("---\n\n");
        }
    }
}

/// 渲染列表（含嵌套）
fn render_list(items: &[ir::ListItem], kind: ListKind, out: &mut String, list_depth: usize) {
    let indent = "  ".repeat(list_depth);
    for (i, item) in items.iter().enumerate() {
        let marker = match kind {
            ListKind::Bullet => "- ".to_string(),
            ListKind::Ordered => format!("{}. ", i + 1),
            ListKind::Task => {
                if item.checked.unwrap_or(false) {
                    "- [x] ".to_string()
                } else {
                    "- [ ] ".to_string()
                }
            }
        };
        out.push_str(&indent);
        out.push_str(&marker);
        render_inlines(&item.inlines, out, BreakMode::Line);
        out.push('\n');

        // 列表项内后续段落：缩进对齐
        for para in &item.extra_paras {
            out.push_str(&indent);
            out.push_str("  ");
            render_inlines(para, out, BreakMode::Line);
            out.push('\n');
        }

        // 嵌套子列表：递归，深度 +1
        for child in &item.children {
            if let Block::List { kind: sub_kind, items: sub_items } = child {
                out.push('\n');
                render_list(sub_items, *sub_kind, out, list_depth + 1);
            }
        }
    }
    // 顶层列表后空一行，嵌套列表不额外空行
    if list_depth == 0 {
        out.push('\n');
    }
}

/// 渲染表格（GFM table 语法），按最大列数补齐空单元格
fn render_table(table: &Table, out: &mut String) {
    if table.rows.is_empty() {
        return;
    }
    let mut max_cols = table.col_count_hint;
    for cells in &table.rows {
        if cells.len() > max_cols {
            max_cols = cells.len();
        }
    }
    if max_cols == 0 {
        return;
    }

    // 提取所有行的单元格文本
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    for cells in &table.rows {
        let mut row: Vec<String> = Vec::new();
        for cell in cells {
            let mut cell_text = String::new();
            render_inlines(cell, &mut cell_text, BreakMode::Suppress);
            row.push(cell_text.trim().to_string());
        }
        while row.len() < max_cols {
            row.push(String::new());
        }
        table_rows.push(row);
    }

    // 第一行作为表头
    let header = &table_rows[0];
    out.push('|');
    for cell in header {
        out.push_str(&format!(" {} |", cell));
    }
    out.push('\n');

    // 分隔行
    out.push('|');
    for _ in 0..max_cols {
        out.push_str(" --- |");
    }
    out.push('\n');

    // 数据行
    for cells in table_rows.iter().skip(1) {
        out.push('|');
        for cell in cells {
            out.push_str(&format!(" {} |", cell));
        }
        out.push('\n');
    }
    out.push('\n');
}

/// 渲染行内内容（含 mark 格式：粗体、斜体、删除线、行内码、链接）
fn render_inlines(inlines: &[Inline], out: &mut String, break_mode: BreakMode) {
    for inline in inlines {
        match inline {
            Inline::Break => {
                if let BreakMode::Line = break_mode {
                    out.push_str("  \n");
                }
            }
            Inline::Mention(label) => {
                out.push_str(&format!("@{}", label));
            }
            Inline::Text { text, marks } => {
                let mut s = text.clone();
                let mut consumed = false;
                for mark in marks {
                    match mark {
                        Mark::Bold => s = format!("**{}**", s),
                        Mark::Italic => s = format!("*{}*", s),
                        Mark::Strike => s = format!("~~{}~~", s),
                        Mark::Code => s = format!("`{}`", s),
                        // Markdown 无原生下划线语法，用 <u> HTML 兜底
                        Mark::Underline => s = format!("<u>{}</u>", s),
                        Mark::Link(href) => {
                            // 链接输出后即完成（与历史行为一致：link 后的 mark 忽略）
                            out.push_str(&format!("[{}]({})", s, href));
                            consumed = true;
                        }
                    }
                    if consumed {
                        break;
                    }
                }
                if !consumed {
                    out.push_str(&s);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers;
    use serde_json::json;

    macro_rules! golden_tests {
        ($($name:ident),* $(,)?) => {
            $(
                #[test]
                fn $name() {
                    let fixture = test_helpers::load_fixture(stringify!($name));
                    let output = tiptap_to_markdown(&fixture);
                    test_helpers::assert_or_update_golden("markdown", stringify!($name), "md", &output);
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

    // ── extract_title（#7 超长标题）──

    #[test]
    fn title_full_text_not_truncated() {
        // 标题 23 个字（超过 18 字截断线）
        let long = "这是一条专门用来测试超长标题去重逻辑的标题文字";
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 1 }, "content": [{ "type": "text", "text": long }] },
                { "type": "paragraph", "content": [{ "type": "text", "text": "正文" }] }
            ]
        });
        assert_eq!(extract_title_full(&doc), long);
        assert_eq!(extract_title(&doc), long.chars().take(18).collect::<String>());
    }

    #[test]
    fn title_heading_priority_and_fallback() {
        let heading_first = json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "content": [{ "type": "text", "text": "首段" }] },
                { "type": "heading", "attrs": { "level": 2 }, "content": [{ "type": "text", "text": "标题段" }] }
            ]
        });
        assert_eq!(extract_title_full(&heading_first), "标题段");

        let para_only = json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "  开头空白段落  " }] }]
        });
        assert_eq!(extract_title_full(&para_only), "开头空白段落");

        let empty = json!({ "type": "doc", "content": [] });
        assert_eq!(extract_title_full(&empty), "Sensend 笔记");
    }
}