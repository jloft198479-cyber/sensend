//! IR（中间表示）：TipTap JSON → 平台格式的唯一中间层。
//!
//! 解析（TipTap→IR）只在这里发生一次；markdown/notion/flowus/lark
//! 四个映射层各自把 IR 渲染成平台格式，不再各自遍历 TipTap JSON。

use serde_json::Value;

// ── 行内节点 ──

/// 行内样式标记
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    Bold,
    Italic,
    Strike,
    Underline,
    Code,
    Link(String),
}

/// 行内节点
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text { text: String, marks: Vec<Mark> },
    /// hardBreak：显式换行
    Break,
    /// mention 提及（@人名），label 为显示文本
    Mention(String),
}

// ── 块级节点 ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListKind {
    Bullet,
    Ordered,
    Task,
}

/// 列表项：首段 + 额外段落 + 嵌套子列表
#[derive(Debug, Clone, PartialEq)]
pub struct ListItem {
    /// 首段行内内容（紧跟列表标记）
    pub inlines: Vec<Inline>,
    /// 列表项内的额外段落
    pub extra_paras: Vec<Vec<Inline>>,
    /// 嵌套子列表（保持结构，供支持嵌套的平台使用）
    pub children: Vec<Block>,
    /// taskItem 的勾选状态；非待办列表为 None
    pub checked: Option<bool>,
}

/// 表格：每行每格均为行内内容（保留格式，修复单元格内联丢失）
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub rows: Vec<Vec<Vec<Inline>>>,
    /// TipTap attrs.col_count（可能缺省为 0），仅 markdown 补齐列时使用
    pub col_count_hint: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading { level: u64, inlines: Vec<Inline> },
    List { kind: ListKind, items: Vec<ListItem> },
    CodeBlock { language: String, code: String },
    /// 引用：每段一组行内内容
    BlockQuote(Vec<Vec<Inline>>),
    Table(Table),
    HorizontalRule,
}

// ── 解析：TipTap JSON → IR（唯一遍历点）──

/// 解析 TipTap 文档 JSON 为 IR 块序列
pub fn parse(doc: &Value) -> Vec<Block> {
    let mut blocks = Vec::new();
    if let Some(children) = doc.get("content").and_then(|c| c.as_array()) {
        for node in children {
            blocks.extend(parse_block(node));
        }
    }
    blocks
}

/// 解析单个块级节点；未知类型递归提取子节点（内容不丢）
fn parse_block(node: &Value) -> Vec<Block> {
    let t = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match t {
        "paragraph" => vec![Block::Paragraph(parse_inlines(node))],
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(1);
            vec![Block::Heading { level, inlines: parse_inlines(node) }]
        }
        "bulletList" => vec![parse_list(node, ListKind::Bullet)],
        "orderedList" => vec![parse_list(node, ListKind::Ordered)],
        "taskList" => vec![parse_list(node, ListKind::Task)],
        "codeBlock" => {
            let language = node
                .get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("")
                .to_string();
            let code = collect_raw_text(node);
            vec![Block::CodeBlock { language, code }]
        }
        "blockquote" => {
            let mut paras: Vec<Vec<Inline>> = Vec::new();
            if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
                for child in children {
                    paras.push(parse_inlines(child));
                }
            }
            if paras.is_empty() {
                paras.push(parse_inlines(node));
            }
            vec![Block::BlockQuote(paras)]
        }
        "table" => vec![Block::Table(parse_table(node))],
        "horizontalRule" => vec![Block::HorizontalRule],
        _ => {
            // 兜底：递归提取子块（与 markdown 旧行为一致，内容不丢）
            let mut blocks = Vec::new();
            if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
                for child in children {
                    blocks.extend(parse_block(child));
                }
            }
            blocks
        }
    }
}

/// 解析列表（bulletList / orderedList / taskList）
fn parse_list(node: &Value, kind: ListKind) -> Block {
    let mut items = Vec::new();
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for item in children {
            let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let expected = match kind {
                ListKind::Task => "taskItem",
                _ => "listItem",
            };
            if item_type != expected {
                continue;
            }
            items.push(parse_list_item(item, kind));
        }
    }
    Block::List { kind, items }
}

/// 解析列表项：首段 + 额外段落 + 嵌套子列表
fn parse_list_item(item: &Value, kind: ListKind) -> ListItem {
    let mut first: Option<Vec<Inline>> = None;
    let mut extra_paras: Vec<Vec<Inline>> = Vec::new();
    let mut children: Vec<Block> = Vec::new();

    if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
        for child in content {
            match child.get("type").and_then(|t| t.as_str()).unwrap_or("") {
                "paragraph" => {
                    let inlines = parse_inlines(child);
                    if first.is_none() {
                        first = Some(inlines);
                    } else {
                        extra_paras.push(inlines);
                    }
                }
                "bulletList" => children.push(parse_list(child, ListKind::Bullet)),
                "orderedList" => children.push(parse_list(child, ListKind::Ordered)),
                "taskList" => children.push(parse_list(child, ListKind::Task)),
                _ => {
                    // 其他子节点（如 hardBreak 顶层出现）：并入首段
                    let inlines = parse_inlines(child);
                    if inlines.is_empty() {
                        continue;
                    }
                    if first.is_none() {
                        first = Some(inlines);
                    } else {
                        extra_paras.push(inlines);
                    }
                }
            }
        }
    }

    // listItem 无子内容时（非标准结构），整体按行内提取兜底
    let inlines = first.unwrap_or_else(|| parse_inlines(item));

    let checked = if kind == ListKind::Task {
        Some(
            item.get("attrs")
                .and_then(|a| a.get("checked"))
                .and_then(|c| c.as_bool())
                .unwrap_or(false),
        )
    } else {
        None
    };

    ListItem { inlines, extra_paras, children, checked }
}

/// 解析表格：每格保留行内内容
fn parse_table(node: &Value) -> Table {
    let col_count_hint = node
        .get("attrs")
        .and_then(|a| a.get("col_count"))
        .and_then(|c| c.as_u64())
        .unwrap_or(0) as usize;

    let mut rows: Vec<Vec<Vec<Inline>>> = Vec::new();
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for row in children {
            if row.get("type").and_then(|t| t.as_str()) != Some("tableRow") {
                continue;
            }
            let mut cells = Vec::new();
            if let Some(cells_arr) = row.get("content").and_then(|c| c.as_array()) {
                for cell in cells_arr {
                    cells.push(parse_inlines(cell));
                }
            }
            rows.push(cells);
        }
    }
    Table { rows, col_count_hint }
}

/// 解析节点的行内内容（text/hardBreak/mention；其他递归）
fn parse_inlines(node: &Value) -> Vec<Inline> {
    let mut out = Vec::new();
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            collect_inline(child, &mut out);
        }
    }
    out
}

/// 递归收集行内节点
fn collect_inline(node: &Value, out: &mut Vec<Inline>) {
    match node.get("type").and_then(|t| t.as_str()).unwrap_or("") {
        "text" => {
            let text = node.get("text").and_then(|t| t.as_str()).unwrap_or("");
            if text.is_empty() {
                return;
            }
            let mut marks = Vec::new();
            if let Some(ms) = node.get("marks").and_then(|m| m.as_array()) {
                for mark in ms {
                    let mt = mark.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    let m = match mt {
                        "bold" => Some(Mark::Bold),
                        "italic" => Some(Mark::Italic),
                        "strike" => Some(Mark::Strike),
                        "underline" => Some(Mark::Underline),
                        "code" => Some(Mark::Code),
                        "link" => mark
                            .get("attrs")
                            .and_then(|a| a.get("href"))
                            .and_then(|h| h.as_str())
                            .map(|h| Mark::Link(h.to_string())),
                        _ => None,
                    };
                    if let Some(m) = m {
                        marks.push(m);
                    }
                }
            }
            out.push(Inline::Text { text: text.to_string(), marks });
        }
        "hardBreak" => out.push(Inline::Break),
        "mention" => {
            let label = node
                .get("attrs")
                .and_then(|a| a.get("label"))
                .and_then(|l| l.as_str())
                .unwrap_or("");
            if !label.is_empty() {
                out.push(Inline::Mention(label.to_string()));
            }
        }
        _ => {
            if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
                for child in children {
                    collect_inline(child, out);
                }
            }
        }
    }
}

/// 收集节点全部原始文本（codeBlock 用，忽略一切格式）
fn collect_raw_text(node: &Value) -> String {
    let mut s = String::new();
    if let Some(t) = node.get("text").and_then(|t| t.as_str()) {
        s.push_str(t);
    }
    if let Some(children) = node.get("content").and_then(|c| c.as_array()) {
        for child in children {
            s.push_str(&collect_raw_text(child));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::test_helpers;
    use serde_json::json;

    #[test]
    fn ir_parse_nested_list_keeps_structure() {
        let doc = test_helpers::load_fixture("nested_list");
        let blocks = parse(&doc);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            Block::List { kind: ListKind::Bullet, items } => {
                assert_eq!(items.len(), 2);
                // 第 1 项有嵌套子列表
                assert_eq!(items[0].children.len(), 1);
                match &items[0].children[0] {
                    Block::List { kind: ListKind::Bullet, items: sub } => {
                        assert_eq!(sub.len(), 2);
                    }
                    other => panic!("嵌套应为列表，实际: {:?}", other),
                }
                // 第 2 项无嵌套
                assert_eq!(items[1].children.len(), 0);
            }
            other => panic!("应为列表块，实际: {:?}", other),
        }
    }

    #[test]
    fn ir_parse_hardbreak_preserved() {
        let doc = test_helpers::load_fixture("hardbreak");
        let blocks = parse(&doc);
        match &blocks[0] {
            Block::Paragraph(inlines) => {
                assert_eq!(inlines.len(), 5);
                assert_eq!(inlines[1], Inline::Break);
                assert_eq!(inlines[3], Inline::Break);
            }
            other => panic!("应为段落块，实际: {:?}", other),
        }
    }

    #[test]
    fn ir_parse_table_keeps_inline_marks() {
        let doc = test_helpers::load_fixture("table_with_inline");
        let blocks = parse(&doc);
        match &blocks[0] {
            Block::Table(t) => {
                assert_eq!(t.rows.len(), 2);
                // 表头第一格带 bold
                match &t.rows[0][0][0] {
                    Inline::Text { text, marks } => {
                        assert_eq!(text, "列名");
                        assert!(marks.contains(&Mark::Bold));
                    }
                    other => panic!("应为文本节点，实际: {:?}", other),
                }
            }
            other => panic!("应为表格块，实际: {:?}", other),
        }
    }

    #[test]
    fn ir_parse_tasklist_checked() {
        let doc = test_helpers::load_fixture("tasklist");
        let blocks = parse(&doc);
        match &blocks[0] {
            Block::List { kind: ListKind::Task, items } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].checked, Some(true));
                assert_eq!(items[1].checked, Some(false));
            }
            other => panic!("应为待办列表块，实际: {:?}", other),
        }
    }

    #[test]
    fn ir_parse_marks_and_mention() {
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "粗", "marks": [{ "type": "bold" }, { "type": "underline" }] },
                    { "type": "hardBreak" },
                    { "type": "mention", "attrs": { "label": "张三" } }
                ]
            }]
        });
        let blocks = parse(&doc);
        match &blocks[0] {
            Block::Paragraph(inlines) => {
                assert_eq!(inlines[0], Inline::Text {
                    text: "粗".into(),
                    marks: vec![Mark::Bold, Mark::Underline],
                });
                assert_eq!(inlines[1], Inline::Break);
                assert_eq!(inlines[2], Inline::Mention("张三".into()));
            }
            other => panic!("应为段落块，实际: {:?}", other),
        }
    }

    #[test]
    fn ir_parse_list_item_extra_paras() {
        let doc = json!({
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
        let blocks = parse(&doc);
        match &blocks[0] {
            Block::List { items, .. } => {
                assert_eq!(items[0].extra_paras.len(), 1);
            }
            other => panic!("应为列表块，实际: {:?}", other),
        }
    }
}