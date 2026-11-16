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

/// 提取文档标题全量文本（不截断到 18 字，用于平台页面标题与去重比较；
/// 仍设一个宽松上限，避免把整篇正文塞进平台标题）
pub fn extract_title_full(content: &Value) -> String {
    if let Some(children) = content.get("content").and_then(|c| c.as_array()) {
        // 优先取第一个 heading
        for node in children {
            if node.get("type").and_then(|t| t.as_str()) == Some("heading") {
                if let Some(text) = extract_plain_text(node) {
                    if !text.trim().is_empty() {
                        return clamp_title(text.trim());
                    }
                }
            }
        }
        // 兜底：取第一个非空块的「首段」文本。
        // [m-01] 原来对任意节点无差别递归 extract_plain_text——首块若是列表/表格，
        // 会把整篇内容拼成一整行当标题（还可能超过平台标题长度上限导致发送失败）。
        for node in children {
            if let Some(text) = first_block_text(node) {
                if !text.trim().is_empty() {
                    return clamp_title(text.trim());
                }
            }
        }
    }
    "Sensend 笔记".to_string()
}

/// 标题字符上限：平台页面标题不该是整篇正文。
/// 取 2000（Notion 标题属性上限）而不是更小的值——`extract_title_full` 同时被
/// notion/flowus 用来和「首块 heading 全文」比较去重，两侧一旦截断长度不一致，
/// 页面标题和正文里的 H1 就会同时出现（去重失效）。
const MAX_TITLE_CHARS: usize = 2000;

fn clamp_title(s: &str) -> String {
    if s.chars().count() <= MAX_TITLE_CHARS {
        s.to_string()
    } else {
        s.chars().take(MAX_TITLE_CHARS).collect()
    }
}

/// 取单个块的「首段」纯文本：下钻到第一个有文字的段落/单元格就停，
/// 不把整块内容（整个列表、整张表格）拼接成一行
fn first_block_text(node: &Value) -> Option<String> {
    match node.get("type").and_then(|t| t.as_str()).unwrap_or("") {
        "paragraph" | "heading" => extract_plain_text(node),
        _ => node
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|children| children.iter().find_map(first_block_text)),
    }
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
#[derive(Clone, Copy)]
enum BreakMode {
    /// 段落内：渲染为 Markdown 硬换行 "  \n"
    Line,
    /// 表格单元格内：GFM 表格一行就是一条记录，真换行会把表格打断，
    /// 用 `<br>` 保留换行（[m-02] 原来直接丢弃，单元格内的换行静默消失）
    Inline,
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
            // 围栏取「代码行最长 ` 连串 + 1」与 3 的较大值：代码含 ``` 行时，3 连反引号会被提前闭合（GFM 规则）
            // 闭合围栏允许最多 3 个空格缩进（GFM 规则），统计时先跳过缩进再数反引号；
            // 4 空格及以上是代码内容，不构成闭合围栏
            let longest = code
                .lines()
                .map(|l| {
                    let indent = l.chars().take_while(|&c| c == ' ').count();
                    if indent > 3 {
                        return 0;
                    }
                    l.chars().skip(indent).take_while(|&c| c == '`').count()
                })
                .max()
                .unwrap_or(0);
            let fence = "`".repeat(3.max(longest + 1));
            out.push_str(&fence);
            out.push_str(language);
            out.push('\n');
            out.push_str(code);
            // 代码末尾无换行时补一个：GFM 要求闭合围栏独占一行，否则粘在末行代码后面不生效
            if !code.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&fence);
            out.push_str("\n\n");
        }
        Block::BlockQuote(paras) => {
            for para in paras {
                let mut line = String::new();
                // 引用内保留硬换行（Line）：IR 用 Break 分隔引用内嵌列表项，Suppress 会把分隔吞掉
                render_inlines(para, &mut line, BreakMode::Line);
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
            render_inlines(cell, &mut cell_text, BreakMode::Inline);
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
        // 先转义反斜杠再转义竖线：原文含字面 \| 时若只转义 |，产出 \\| 会被 GFM 当列分隔符
        out.push_str(&format!(" {} |", cell.replace('\\', "\\\\").replace('|', "\\|")));
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
            out.push_str(&format!(" {} |", cell.replace('\\', "\\\\").replace('|', "\\|")));
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
                match break_mode {
                    BreakMode::Line => out.push_str("  \n"),
                    BreakMode::Inline => out.push_str("<br>"),
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

    // ── 表格竖线转义（单元格含 | 时列不得错位）──

    #[test]
    fn table_cell_pipe_escaped() {
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "table",
                "content": [{
                    "type": "tableRow",
                    "content": [
                        { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "a|b" }] }] },
                        { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "c" }] }] }
                    ]
                }]
            }]
        });
        let md = tiptap_to_markdown(&doc);
        assert!(md.contains("a\\|b"), "单元格内的 | 必须转义为 \\|: {}", md);
        assert!(!md.contains("a|b"), "未转义的 | 会造成列错位: {}", md);
    }

    #[test]
    fn table_cell_backslash_pipe_escaped() {
        // 角案：原文含字面反斜杠+竖线（a\|b）。须先转义 \ 再转义 |，
        // 否则产出 \\| 的偶数反斜杠让 GFM 把 | 当列分隔符，仍然错位
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "table",
                "content": [{
                    "type": "tableRow",
                    "content": [
                        { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "a\\|b" }] }] }
                    ]
                }]
            }]
        });
        let md = tiptap_to_markdown(&doc);
        assert!(md.contains("a\\\\\\|b"), "应输出 \\\\ + \\|（字面反斜杠+字面竖线）: {}", md);
    }

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

    // ── [m-01] 标题兜底不能把整篇内容拼成标题 ──

    #[test]
    fn title_from_list_uses_first_item_only() {
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "bulletList",
                "content": [
                    { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "第一项" }] }] },
                    { "type": "listItem", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "第二项" }] }] }
                ]
            }]
        });
        assert_eq!(
            extract_title_full(&doc), "第一项",
            "首块是列表时标题只取首项，不能把整个列表拼成一行"
        );
    }

    #[test]
    fn title_from_table_uses_first_cell_only() {
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "table",
                "content": [{
                    "type": "tableRow",
                    "content": [
                        { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "列A" }] }] },
                        { "type": "tableCell", "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "列B" }] }] }
                    ]
                }]
            }]
        });
        assert_eq!(extract_title_full(&doc), "列A", "首块是表格时只取第一个单元格");
    }

    #[test]
    fn title_clamped_for_huge_first_paragraph() {
        // 极端长首段（整篇一段）不能原样当标题，否则可能超平台标题上限。
        // 上限取 2000（Notion 标题属性上限），与去重比较用的长度一致
        let long = "超".repeat(2500);
        let doc = json!({ "type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": long }] }
        ]});
        assert_eq!(extract_title_full(&doc).chars().count(), 2000);
        // 未超上限时不截断（去重比较依赖全长）
        let short = "超".repeat(1800);
        let doc2 = json!({ "type": "doc", "content": [
            { "type": "paragraph", "content": [{ "type": "text", "text": short }] }
        ]});
        assert_eq!(extract_title_full(&doc2).chars().count(), 1800);
    }

    // ── [m-02] 表格单元格内的换行 ──

    #[test]
    fn table_cell_hardbreak_becomes_br() {
        // 单元格内两段（M-10 产出的 Break）+ 段内 hardBreak（m-02）
        let doc = json!({
            "type": "doc",
            "content": [{
                "type": "table",
                "content": [{
                    "type": "tableRow",
                    "content": [{
                        "type": "tableCell",
                        "content": [
                            { "type": "paragraph", "content": [
                                { "type": "text", "text": "第一行" },
                                { "type": "hardBreak" },
                                { "type": "text", "text": "第二行" }
                            ]},
                            { "type": "paragraph", "content": [{ "type": "text", "text": "第三行" }] }
                        ]
                    }]
                }]
            }]
        });
        let md = tiptap_to_markdown(&doc);
        assert!(
            md.contains("第一行<br>第二行<br>第三行"),
            "单元格内换行应渲染为 <br>（既不能丢，也不能输出真换行打断表格）: {}",
            md
        );
        // 表格仍是表头 + 1 数据行 = 2 行，没有被打断
        assert!(md.matches('|').count() >= 4, "表格列结构应完整: {}", md);
    }
}