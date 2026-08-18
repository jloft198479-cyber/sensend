# Notion 平台格式规范与适配调研

> 调研目标：基于 Notion API 的格式展示要求和规范，对比 Sensend 当前的 TipTap→Notion 转换链路，找出差异和缺失项，为后续优化提供依据。
> 调研时间：2026-08-18
> 版本基线：Sensend v0.4.0
> Notion API 版本：2022-06-28（Sensend 代码中 `NOTION_VERSION` 常量）

---

## 〇、结论先行

1. **核心格式覆盖完整**：Sensend 当前支持的全部 TipTap 节点类型（段落、标题、列表、代码块、引用、表格、分割线）均有 Notion block 对应，行内样式（粗体/斜体/删除线/下划线/行内码/链接）也全部映射到 Notion annotations。
2. **三个潜在缺陷**：①rich_text 文本超 2000 字符未分段 — **✅ 已修复（d2d8909）**；②无 429 限流重试机制 — ❌ 未修复；③表格单元格只取 rich_text 首元素，多段格式可能丢失 — **✅ 已修复（d2d8909）**。
3. **Notion 有大量能力 Sensend 尚未利用**：callout（提示框）、toggle（折叠块）、bookmark（书签）、颜色标注、可折叠标题、行内公式、真正 @提及——这些是未来增强方向，但不影响当前基本功能。
4. **代码块语言映射需对齐**：TipTap 的 language 标识与 Notion 的 60+ 语言枚举存在命名差异（如 `typescript` vs `typescript`、`plaintext` vs `plain text`），当前代码用原始值直传，部分语言可能不被 Notion 识别。

---

## 一、Notion API Block 类型体系

### 1.1 完整 Block 类型清单

Notion API（2022-06-28 版本）支持以下 block 类型：

| Block 类型 | 说明 | 支持 rich_text | 支持 children | Sensend 是否使用 |
|-----------|------|:---:|:---:|:---:|
| `paragraph` | 段落 | ✅ | ✅ | ✅ |
| `heading_1` | 一级标题 | ✅ | ✅（is_toggleable） | ✅ |
| `heading_2` | 二级标题 | ✅ | ✅（is_toggleable） | ✅ |
| `heading_3` | 三级标题 | ✅ | ✅（is_toggleable） | ✅ |
| `heading_4` | 四级标题（较新） | ✅ | ✅（is_toggleable） | ❌ IR 无此级别 |
| `bulleted_list_item` | 无序列表项 | ✅ | ✅ | ✅ |
| `numbered_list_item` | 有序列表项 | ✅ | ✅ | ✅ |
| `to_do` | 待办事项 | ✅ | ✅ | ✅ |
| `code` | 代码块 | ✅ | ❌ | ✅ |
| `quote` | 引用块 | ✅ | ✅ | ✅ |
| `table` | 表格 | ❌ | ✅（table_row） | ✅ |
| `table_row` | 表格行 | ❌ | ❌ | ✅（作为 table children） |
| `divider` | 分割线 | ❌ | ❌ | ✅ |
| `callout` | 提示框 | ✅ | ✅ | ❌ |
| `toggle` | 折叠块 | ✅ | ✅ | ❌ |
| `bookmark` | 书签 | ❌（caption） | ❌ | ❌ |
| `embed` | 嵌入网页 | ❌ | ❌ | ❌ |
| `image` | 图片 | ❌（caption） | ❌ | ❌ |
| `video` | 视频 | ❌（caption） | ❌ | ❌ |
| `audio` | 音频 | ❌（caption） | ❌ | ❌ |
| `file` | 文件 | ❌（caption） | ❌ | ❌ |
| `pdf` | PDF | ❌（caption） | ❌ | ❌ |
| `equation` | 公式块 | ❌ | ❌ | ❌ |
| `column_list` | 多栏容器 | ❌ | ✅（column） | ❌ |
| `column` | 栏 | ❌ | ✅ | ❌ |
| `table_of_contents` | 目录 | ❌ | ❌ | ❌ |
| `breadcrumb` | 面包屑 | ❌ | ❌ | ❌ |
| `synced_block` | 同步块 | ❌ | ✅ | ❌ |
| `template` | 模板按钮 | ❌ | ✅ | ❌ |
| `link_preview` | 链接预览 | ❌ | ❌ | ❌ |
| `transcription` | 会议记录（2026-03-11 起改名） | ❌ | ✅ | ❌ |
| `child_page` | 子页面 | ❌ | ❌ | ❌ |
| `child_database` | 子数据库 | ❌ | ❌ | ❌ |
| `unsupported` | 不支持的类型 | ❌ | ❌ | — |

### 1.2 Heading 的 is_toggleable 属性

Notion 的 heading_1/2/3/4 支持一个 `is_toggleable` 布尔属性。设为 `true` 时，标题变成可折叠的 toggle heading——点击可以展开/收起子内容。

```json
{
  "type": "heading_1",
  "heading_1": {
    "rich_text": [...],
    "color": "default",
    "is_toggleable": true
  }
}
```

Sensend 当前未设置此属性（默认 `false`），所有标题都是普通标题。

### 1.3 Callout（提示框）

Callout 是 Notion 中非常常用的 block，类似带图标的引用块：

```json
{
  "type": "callout",
  "callout": {
    "rich_text": [...],
    "icon": { "emoji": "💡" },
    "color": "default"
  }
}
```

Sensend 的 TipTap 编辑器没有 callout 节点，IR 中也没有对应类型。如果未来要支持，需要：
- 前端：TipTap 扩展新增 callout 节点
- IR：新增 `Block::Callout { inlines, icon, color }`
- Notion adapter：映射为 callout block

### 1.4 Code Block 语言枚举

Notion 代码块支持 60+ 种语言，完整列表：

```
abap, arduino, bash, basic, c, clojure, coffeescript, c++, c#, css, dart,
diff, docker, elixir, elm, erlang, flow, fortran, f#, gherkin, glsl, go,
graphql, groovy, haskell, html, java, javascript, json, julia, kotlin,
latex, less, lisp, livescript, lua, makefile, markdown, markup, matlab,
mermaid, nix, objective-c, ocaml, pascal, perl, php, plain text, powershell,
prolog, protobuf, python, r, reason, ruby, rust, sass, scala, scheme, scss,
shell, sql, swift, typescript, vb.net, verilog, vhdl, visual basic,
webassembly, xml, yaml, java/c/c++/c#
```

Sensend 当前代码（`notion.rs:123`）直接把 TipTap 的 language 属性值传给 Notion，如果为空则填 `"plain text"`。TipTap 的 language 值来自代码块的高亮库（通常是小写），大部分与 Notion 枚举一致，但有几个需注意：

| TipTap 常见值 | Notion 枚举值 | 是否匹配 |
|-------------|-------------|:---:|
| `typescript` | `typescript` | ✅ |
| `javascript` | `javascript` | ✅ |
| `python` | `python` | ✅ |
| `rust` | `rust` | ✅ |
| `json` | `json` | ✅ |
| `bash` / `shell` | `bash` / `shell` | ✅ |
| `html` | `html` | ✅ |
| `css` | `css` | ✅ |
| `sql` | `sql` | ✅ |
| `markdown` | `markdown` | ✅ |
| `plaintext` / `text` / `txt` | `plain text` | ⚠️ 需映射 |
| `c++` / `cpp` | `c++` | ⚠️ 需确认 |
| `c#` / `csharp` | `c#` | ⚠️ 需确认 |
| `go` | `go` | ✅ |
| `java` | `java` | ✅ |
| `yaml` | `yaml` | ✅ |

---

## 二、Notion Rich Text 规范

### 2.1 Rich Text 对象结构

Notion 的行内内容统一用 rich_text 数组表示。每个 rich_text 对象有三种类型：

| 类型 | 说明 | Sensend 是否使用 |
|------|------|:---:|
| `text` | 普通文本 | ✅ 全部用此类型 |
| `mention` | @提及（页面/用户/数据库/日期/链接预览） | ❌ 降级为纯文本 |
| `equation` | 行内 LaTeX 公式 | ❌ 不支持 |

### 2.2 Annotation 对象（行内样式）

Notion 的 annotations 支持以下字段：

| 属性 | 类型 | 说明 | Sensend 是否映射 |
|------|------|------|:---:|
| `bold` | boolean | 粗体 | ✅ `Mark::Bold` |
| `italic` | boolean | 斜体 | ✅ `Mark::Italic` |
| `strikethrough` | boolean | 删除线 | ✅ `Mark::Strike` |
| `underline` | boolean | 下划线 | ✅ `Mark::Underline` |
| `code` | boolean | 行内代码 | ✅ `Mark::Code` |
| `color` | string (enum) | 文字颜色 | ❌ 未使用 |

**颜色枚举（19 种）**：
`default`, `gray`, `brown`, `orange`, `yellow`, `green`, `blue`, `purple`, `pink`, `red`, `gray_background`, `brown_background`, `orange_background`, `yellow_background`, `green_background`, `blue_background`, `purple_background`, `pink_background`, `red_background`

Sensend 的 TipTap 编辑器没有颜色标注功能，IR 中也没有 color mark，因此当前不映射。如未来需要支持，需在 IR 的 `Mark` enum 新增 `Color(String)` 变体。

### 2.3 Mention（@提及）

Notion 的 mention rich text 类型支持提及以下对象：

| mention 子类型 | 说明 | plain_text 示例 |
|-------------|------|-------------|
| `page` | 提及另一个 Notion 页面 | 页面标题 |
| `user` | 提及用户 | "@用户名" |
| `database` | 提及数据库 | 数据库标题 |
| `date` | 提及日期 | "2022-12-16" |
| `link_preview` | 链接预览提及 | URL |
| `template_mention` | 模板占位符 | "@Today" / "@Me" |

Sensend 当前的处理（`notion.rs:562-564`）：把 `Inline::Mention(label)` 降级为纯文本 `"@{label}"`，即生成一个普通 text rich_text 对象。这不会丢失内容，但失去了 Notion mention 的链接能力——在 Notion 中点击不会跳转到被提及的页面/用户。

### 2.4 行内公式（Equation）

Notion 支持行内 LaTeX 公式：

```json
{
  "type": "equation",
  "equation": { "expression": "E = mc^2" }
}
```

Sensend 的 TipTap 编辑器没有公式节点，IR 中也没有对应类型，当前不支持。

---

## 三、Sensend 当前转换链路分析

### 3.1 转换链路总览

```
TipTap JSON（前端编辑器输出）
    │
    ▼
ir::parse()（ir.rs:76）           ← 唯一遍历点，TipTap → IR
    │
    ▼
IR Block 序列                     ← 平台无关的中间表示
    │
    ├─→ notion.rs: map_block()    ← IR → Notion blocks
    ├─→ markdown.rs: render_block() ← IR → Markdown 文本
    ├─→ flowus.rs                  ← IR → FlowUs blocks
    └─→ lark.rs                    ← IR → 飞书 blocks
```

### 3.2 IR 数据模型（ir.rs）

**行内节点**：

```rust
pub enum Mark {
    Bold, Italic, Strike, Underline, Code, Link(String),
}

pub enum Inline {
    Text { text: String, marks: Vec<Mark> },
    Break,           // hardBreak → 换行
    Mention(String), // @提及，label 为显示文本
}
```

**块级节点**：

```rust
pub enum Block {
    Paragraph(Vec<Inline>),
    Heading { level: u64, inlines: Vec<Inline> },
    List { kind: ListKind, items: Vec<ListItem> },
    CodeBlock { language: String, code: String },
    BlockQuote(Vec<Vec<Inline>>),
    Table(Table),
    HorizontalRule,
}
```

### 3.3 Notion 适配器的映射实现（notion.rs）

**Block 级映射**（`map_block()` 函数，L94-182）：

| IR Block | Notion block | 代码位置 | 备注 |
|----------|-------------|---------|------|
| `Paragraph(inlines)` | `paragraph` | L98-104 | rich_text 数组 |
| `Heading { level: 1 }` | `heading_1` | L105-116 | level ≥ 3 统一映射为 heading_3 |
| `Heading { level: 2 }` | `heading_2` | L105-116 | |
| `Heading { level: 3+ }` | `heading_3` | L105-116 | heading_4 未使用 |
| `List { Bullet }` | `bulleted_list_item` | L117-121 | 逐项映射 |
| `List { Ordered }` | `numbered_list_item` | L117-121 | 逐项映射 |
| `List { Task }` | `to_do` | L117-121 | 带 checked 属性 |
| `CodeBlock` | `code` | L122-132 | language 直传，空则 "plain text" |
| `BlockQuote` | `quote` | L133-141 | 多段 → 多个 quote block |
| `Table` | `table` + `table_row` | L142-172 | has_column_header/has_row_header 均为 false |
| `HorizontalRule` | `divider` | L173-179 | |

**行内映射**（`map_rich_text()` 函数，L526-568）：

| IR Inline | Notion rich_text | 代码位置 | 备注 |
|-----------|-----------------|---------|------|
| `Text { Bold }` | annotations.bold = true | L536 | |
| `Text { Italic }` | annotations.italic = true | L537 | |
| `Text { Strike }` | annotations.strikethrough = true | L538 | |
| `Text { Underline }` | annotations.underline = true | L539 | |
| `Text { Code }` | annotations.code = true | L540 | |
| `Text { Link(href) }` | text.link = { url: href } | L541, L547-549 | |
| `Break` | text "\n" | L559-561 | 换行符作为独立 text 对象 |
| `Mention(label)` | text "@{label}" | L562-564 | 降级为纯文本 |

**列表项嵌套**（`map_list_item()` 函数，L186-229）：
- 嵌套子列表映射为 `children` 数组
- 代码注释说明 Notion API 单请求 children 仅支持两层嵌套
- 列表项内多段落用 `\n` 分隔合并到同一个 rich_text

**100-block 批处理**（`create_page()` 函数，L350-355, `append_children()` 函数，L393-409）：
- 创建页面时 children 最多 100 个，超出部分追加
- 追加也按 100 个一批分块

### 3.4 表格单元格的处理

当前代码（L146-161）：

```rust
for cell in row {
    let rt = map_rich_text(cell);
    row_cells.push(if rt.is_empty() {
        json!({"type": "text", "text": {"content": ""}})
    } else {
        rt[0].clone()  // ⚠️ 只取第一个 rich_text 元素
    });
}
```

**问题**：`rt[0].clone()` 只取了 rich_text 数组的第一个元素。如果一个单元格内的文本有混合样式（如 "**粗体**普通*斜体*"），`map_rich_text` 会返回多个 rich_text 对象，但这里只保留了第一个，其余丢失。

Notion table_row 的单元格实际上接受 rich_text 数组，正确的做法应该是把整个 `rt` 数组作为单元格值传入，而非只取 `[0]`。

---

## 四、Notion API 工程约束

### 4.1 请求大小限制

| 限制项 | 上限 | Sensend 当前处理 | 风险 |
|--------|------|-----------------|------|
| 单次 children blocks | 100 个 | ✅ 已分批（L350, L399） | 无 |
| rich_text `text.content` | 2000 字符 | ❌ 未分段 | 🔴 长文本发送会被 Notion 拒绝 |
| rich_text `text.link.url` | 2000 字符 | ❌ 未校验 | 🟡 长链接极少见 |
| rich_text `equation.expression` | 1000 字符 | — | 不支持公式 |
| rich_text 数组长度 | 100 个元素 | ❌ 未校验 | 🟡 超长段落可能超限 |
| payload 总大小 | 500KB | ❌ 未校验 | 🟡 极端情况可能超限 |
| payload block 总数 | 1000 个 | ✅ 已分批 | 无 |
| 请求超时 | 60 秒 | ✅ HTTP 客户端 15s 超时 | 无 |

### 4.2 Rate Limit

| 限制 | 说明 | Sensend 当前处理 |
|------|------|-----------------|
| 每连接 3 req/s | 平均 3 请求/秒，允许突发 | ❌ 无限流控制 |
| 工作区级共享限制 | 按 workspace plan 缩放 | ❌ 无法控制 |
| 429 响应 | 返回 `Retry-After` 头 | ❌ 未实现重试 |
| 529 响应 | 服务过载 | ❌ 未实现重试 |

Sensend 是手动触发发送（非批量），正常使用频率很低，限流风险不大。但如果用户连续快速发送多条笔记，可能触发 429。当前代码遇到 429 会直接报错给用户，不会自动重试。

### 4.3 Notion API 版本

Sensend 代码中使用 `Notion-Version: 2022-06-28`（`notion.rs:6`），这是当前稳定的 API 版本。注意 Notion 文档中提到 `transcription` 类型在 `2026-03-11` 版本起改名为 `meeting notes`，但 2022-06-28 版本仍使用旧名。

---

## 五、格式映射对照表（TipTap → IR → Notion）

### 5.1 块级映射

| TipTap 节点 | IR Block | Notion Block | 映射状态 | 备注 |
|------------|----------|-------------|:---:|------|
| `paragraph` | `Paragraph` | `paragraph` | ✅ | |
| `heading` (level 1) | `Heading { level: 1 }` | `heading_1` | ✅ | |
| `heading` (level 2) | `Heading { level: 2 }` | `heading_2` | ✅ | |
| `heading` (level 3) | `Heading { level: 3 }` | `heading_3` | ✅ | |
| `heading` (level 4+) | `Heading { level: 4+ }` | `heading_3` | ⚠️ | 降级为 heading_3，Notion 支持 heading_4 但 IR 未映射 |
| `bulletList` | `List { Bullet }` | `bulleted_list_item` | ✅ | |
| `orderedList` | `List { Ordered }` | `numbered_list_item` | ✅ | |
| `taskList` | `List { Task }` | `to_do` | ✅ | |
| `codeBlock` | `CodeBlock` | `code` | ✅ | 语言直传 |
| `blockquote` | `BlockQuote` | `quote` | ✅ | 多段 → 多 quote |
| `table` | `Table` | `table` + `table_row` | ⚠️ | 单元格只取首元素 |
| `horizontalRule` | `HorizontalRule` | `divider` | ✅ | |
| 嵌套 list | `ListItem.children` | `children` | ✅ | 仅一层 |
| 未知节点 | 兜底递归提取 | — | ✅ | 不丢内容 |

### 5.2 行内映射

| TipTap Mark | IR Mark | Notion Annotation | 映射状态 |
|------------|---------|-------------------|:---:|
| `bold` | `Bold` | `bold: true` | ✅ |
| `italic` | `Italic` | `italic: true` | ✅ |
| `strike` | `Strike` | `strikethrough: true` | ✅ |
| `underline` | `Underline` | `underline: true` | ✅ |
| `code` | `Code` | `code: true` | ✅ |
| `link` (attrs.href) | `Link(String)` | `text.link: { url }` | ✅ |
| `hardBreak` | `Break` | text "\n" | ✅ |
| `mention` | `Mention(String)` | text "@label" | ⚠️ 降级为纯文本 |
| — | — | `color` | ❌ 未使用 |
| — | — | `mention`（rich text 类型） | ❌ 未使用 |
| — | — | `equation`（rich text 类型） | ❌ 未使用 |

---

## 六、发现的问题与改进建议

### 6.1 已确认的缺陷

#### 缺陷 P1：表格单元格多 rich_text 丢失 ✅ 已修复（d2d8909）

**位置**：`notion.rs` 表格单元格映射（原 `rt[0].clone()` 已移除）

**问题**：当表格单元格内文本有混合样式时（如"**粗体**+普通+*斜体*"），`map_rich_text` 返回多个 rich_text 对象，但只取了第一个。

**影响**：表格内混合样式的文字会丢失后续段的格式或内容。

**修复方案**：Notion table_row 的单元格接受 rich_text 数组，应传入完整数组：

```rust
// 修复前
row_cells.push(if rt.is_empty() {
    json!({"type": "text", "text": {"content": ""}})
} else {
    rt[0].clone()
});

// 修复后
row_cells.push(if rt.is_empty() {
    json!([{"type": "text", "text": {"content": ""}}])
} else {
    json!(rt)
});
```

#### 缺陷 P2：rich_text 文本超 2000 字符未分段 ✅ 已修复（d2d8909）

**位置**：`map_rich_text()` 函数（L539-565），`chars.chunks(2000)` 切片

**问题**：Notion 限制单个 rich_text 对象的 `text.content` 最多 2000 字符。当用户在 Sensend 中输入超长段落（如粘贴一篇长文），单个 text 对象可能超限，导致 Notion API 返回 `validation_error`。

**影响**：长文本发送失败。

**修复方案**：在 `map_rich_text` 中对超过 2000 字符的文本进行分段：

```rust
Inline::Text { text, marks } => {
    for chunk in text.chars().collect::<Vec<_>>().chunks(2000) {
        let content: String = chunk.iter().collect();
        // 创建 rich_text 对象，携带相同的 annotations 和 link
        rt.push(/* ... */);
    }
}
```

#### 缺陷 P3：无 429 限流重试

**位置**：`request()` 函数（L29-74）

**问题**：Notion API 返回 429 时带 `Retry-After` 头，建议等待后重试。当前代码遇到非 2xx 直接返回错误，不重试。

**影响**：连续快速发送时可能遇到 429 报错，用户体验不佳。

**修复方案**：在 `request()` 中加入 429/529 重试逻辑：

```rust
// 遇到 429 时读取 Retry-After，等待后重试
if status == 429 || status == 529 {
    let retry_after = res.headers().get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1);
    tokio::time::sleep(Duration::from_secs(retry_after)).await;
    // 重试（最多 3 次）
}
```

### 6.2 代码块语言映射对齐

**位置**：`notion.rs:123`

**问题**：TipTap 的 language 值直传给 Notion，部分值可能不匹配。最常见的不匹配是 `plaintext` / `text` / `txt` → Notion 要求 `plain text`（有空格）。

**修复方案**：建立映射表，类似飞书的 `LARK_LANG_MAP`（`lark.rs` 中已有）：

```rust
fn map_notion_lang(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "" | "text" | "txt" | "plaintext" => "plain text",
        "cpp" => "c++",
        "csharp" => "c#",
        "ts" => "typescript",
        "js" => "javascript",
        "py" => "python",
        "sh" | "shell" => "bash",
        // 其他直接返回原值（大部分已匹配）
        other => other,
    }
}
```

### 6.3 未来增强方向（非缺陷，按需实施）

| 增强项 | 价值 | 实现复杂度 | 说明 |
|--------|------|:---:|------|
| Callout 提示框 | 高 | 中 | 需前端+IR+adapter 三层联动 |
| 可折叠标题 | 中 | 低 | heading 加 `is_toggleable: true` |
| 表格表头行 | 中 | 低 | `has_column_header: true` |
| 真正 @提及 | 中 | 高 | 需实现 Notion mention rich text 类型 |
| 块级颜色 | 低 | 中 | 需前端颜色选择器 + IR 扩展 |
| 行内公式 | 低 | 高 | 需 TipTap 公式扩展 + IR 扩展 |
| 书签 block | 低 | 低 | URL → bookmark block |
| 目录 block | 低 | 低 | 直接插入 `table_of_contents` |

### 6.4 优先级建议

```
P1（应尽快修复）：
  ├── ✅ 表格单元格多 rich_text 丢失      ← 已修复（d2d8909）
  ├── ✅ rich_text 2000 字符分段          ← 已修复（d2d8909）
  └── ❌ 代码块语言映射表                 ← 未修复，小改动收益明确

P2（建议实施）：
  ├── 429 限流重试                       ← 提升健壮性
  └── 表格表头行识别                     ← 展示效果提升

P3（按需实施）：
  ├── Callout 提示框                     ← 产品级增强
  ├── 可折叠标题                         ← 小改动
  └── 其他增强项                         ← 视用户需求
```

---

## 七、测试覆盖现状

Notion adapter 当前有 11 个 golden test（快照测试）和 4 个目标断言测试：

| 测试名 | 覆盖场景 |
|--------|---------|
| `simple_paragraph` | 基本段落 |
| `headings` | 多级标题 |
| `nested_list` | 嵌套列表 |
| `table_with_inline` | 表格含行内格式 |
| `hardbreak` | 段内换行 |
| `tasklist` | 待办列表 |
| `codeblock` | 代码块 |
| `blockquote` | 引用块 |
| `long_title` | 超长标题去重 |
| `underline_link` | 下划线+链接 |
| `combined` | 综合场景 |
| `fix1_hardbreak_becomes_newline` | 换行→\n 断言 |
| `fix2_nested_list_children` | 嵌套→children 断言 |
| `fix5_table_cell_keeps_annotations` | 表格格式断言 |
| `fix7_long_title_dedup_full_text` | 标题去重断言 |

**测试缺口**：
- ❌ 超长文本（2000+ 字符）的 rich_text 分段（代码已修复，缺专项测试）
- ❌ 429 限流场景
- ❌ 表格单元格多段混合样式（golden 快照已更新，缺独立回归测试）
- ❌ 代码块语言映射（`plaintext` → `plain text`）
- ❌ 100+ blocks 的分批追加

---

## 八、附录：Notion API 参考链接

- [Block 类型完整参考](https://developers.notion.com/reference/block)
- [Rich Text 规范](https://developers.notion.com/reference/rich-text)
- [请求限制](https://developers.notion.com/reference/request-limits)
- [Append Block Children](https://developers.notion.com/reference/patch-block-children)
- [Create Page](https://developers.notion.com/reference/post-page)
- [版本变更日志](https://developers.notion.com/reference/versioning)
