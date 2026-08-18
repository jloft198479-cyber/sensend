# 飞书平台格式规范与适配调研

> 调研目标：基于飞书开放平台文档 API 的格式展示要求和规范，对比 Sensend 当前的 TipTap→飞书转换链路，找出差异和缺失项，为后续优化提供依据。
> 调研时间：2026-08-18
> 版本基线：Sensend v0.4.0
> 飞书 API 基线：`open.feishu.cn/open-apis/docx/v1`（代码中 `FEISHU_BASE` 常量）
> 飞书 API 文档：https://open.feishu.cn/document/server-docs/docs/docs/docx-v1/document-block/create

---

## 〇、结论先行

1. **核心格式覆盖完整**：Sensend 当前支持的 TipTap 节点类型（段落、标题1-9、无序/有序/待办列表、代码块、引用、分割线）和行内样式（粗体/斜体/删除线/下划线/行内码/链接）全部映射到飞书 block，且代码块语言映射表覆盖 34 个别名、75 种飞书枚举语言中的常用 30+ 种。
2. **四个潜在缺陷**（lark.rs 截至 2026-08-18）：①无 429 限流重试（飞书单文档 3 并发/s，超限返回 429，未修）；②~~表格降级为文本~~ **已修复**（原生 Table Block 31 + descendant API）；③引用未使用 quote_container（34）嵌套结构（未修）；④无 text_run 长度上限检查（未修）。
3. **飞书有大量能力 Sensend 尚未利用**：callout（高亮块）、折叠标题/列表、文本对齐、行内颜色（7 种文字色 + 15 种背景色）、@用户/@文档、行内公式（KaTeX）、日期提醒、分栏、内嵌网页、图片——这些是未来增强方向。
4. **飞书 vs Notion 差异显著**：飞书支持 9 级标题（Notion 仅 3 级）、原生表格通过嵌套块 API（Notion 直接支持）、块级背景色 14 种（Notion 无）、行内颜色 22 种（Notion 仅 19 种 color 枚举且语义不同）；但飞书无 toggle（Notion 有 toggle 块）、无 bookmark。

---

## 一、飞书 API Block 类型体系

### 1.1 完整 Block 类型清单

飞书 docx API（`/docx/v1/documents/:document_id/blocks/:block_id/children`）支持以下 block 类型：

| block_type | 类型名 | 支持 API 创建 | 支持 rich_text | Sensend 是否使用 |
|:---:|---|:---:|:---:|:---:|
| 1 | Page（页面根块） | ❌ | ✅ | —（根块，不可创建） |
| 2 | Text（文本段落） | ✅ | ✅ | ✅ `BLOCK_TEXT` |
| 3 | Heading 1（一级标题） | ✅ | ✅ | ✅ `BLOCK_HEADING1` |
| 4 | Heading 2 | ✅ | ✅ | ✅ |
| 5 | Heading 3 | ✅ | ✅ | ✅ |
| 6 | Heading 4 | ✅ | ✅ | ✅ |
| 7 | Heading 5 | ✅ | ✅ | ✅ |
| 8 | Heading 6 | ✅ | ✅ | ✅ |
| 9 | Heading 7 | ✅ | ✅ | ✅ |
| 10 | Heading 8 | ✅ | ✅ | ✅ |
| 11 | Heading 9 | ✅ | ✅ | ✅ |
| 12 | Bullet（无序列表） | ✅ | ✅ | ✅ `BLOCK_BULLET` |
| 13 | Ordered（有序列表） | ✅ | ✅ | ✅ `BLOCK_ORDERED` |
| 14 | Code（代码块） | ✅ | ✅ | ✅ `BLOCK_CODE` |
| 15 | Quote（引用） | ✅ | ✅ | ✅ `BLOCK_QUOTE` |
| 17 | Todo（待办） | ✅ | ✅ | ✅ `BLOCK_TODO` |
| 18 | Bitable（多维表格） | 开发中 | — | ❌ |
| 19 | Callout（高亮块） | 开发中 | — | ❌ |
| 20 | Chat Card（群名片） | 开发中 | — | ❌ |
| 21 | Diagram（UML 图） | ❌ | — | ❌ |
| 22 | Divider（分割线） | ✅ | — | ✅ `BLOCK_DIVIDER` |
| 23 | File（文件） | 开发中 | — | ❌ |
| 24 | Grid（分栏） | 开发中 | — | ❌ |
| 25 | Grid Column（分栏列） | ❌ | — | ❌ |
| 26 | Iframe（内嵌网页） | 开发中 | — | ❌ |
| 27 | Image（图片） | 开发中 | — | ❌ |
| 28 | ISV（三方小组件） | ❌ | — | ❌ |
| 29 | Mindnote（思维笔记） | 开发中 | — | ❌ |
| 30 | Sheet（电子表格） | ✅（单次≤5） | — | ❌ |
| 31 | Table（表格） | ✅（需嵌套块 API） | — | ✅（已用 descendant API 原生创建） |
| 32 | Table Cell（单元格） | ✅（需嵌套块 API） | ✅ | ✅（表格单元格内容已随表格创建） |
| 33 | View（视图） | ✅ | — | ❌ |
| 34 | Quote Container（引用容器） | ✅ | — | ❌ |
| 35 | Task（任务） | 开发中 | — | ❌ |
| 36-39 | OKR 系列 | 开发中 | — | ❌ |
| 40 | Add-Ons（文档小组件） | 开发中 | — | ❌ |
| 41 | Jira Issue | 开发中 | — | ❌ |
| 42 | Wiki Catalog | 开发中 | — | ❌ |
| 43 | Board（画板） | 开发中 | — | ❌ |
| 44-47 | Agenda 系列 | 开发中 | — | ❌ |
| 48 | Link Preview | 开发中 | — | ❌ |
| 49-50 | Synced Block（同步块） | ❌（只读） | — | ❌ |
| 51 | Sub Page List | 开发中 | — | ❌ |
| 52 | AI Template | ❌（只读） | — | ❌ |
| 999 | Undefined（未支持） | ❌ | — | ❌ |

**来源**：飞书官方 BlockType 枚举 + Create blocks API 请求体 block_type 字段说明。

### 1.2 Block 级 style（text_style）字段

所有支持 rich_text 的 block（text/heading1-9/bullet/ordered/code/quote/todo）共享一个 `style` 对象（text_style），包含以下字段：

| 字段 | 类型 | 说明 | Sensend 是否使用 |
|---|---|---|:---:|
| `align` | int | 对齐方式：1=左（默认）、2=居中、3=右 | ❌ |
| `done` | boolean | Todo 勾选状态（仅 Todo Block） | ✅ `map_list_item` L464 |
| `folded` | boolean | 折叠状态（支持 Heading1-9 及有 children 的 Text/Bullet/Ordered/Todo） | ❌ |
| `language` | int | 代码块语言枚举 1-75（仅 Code Block） | ✅ `map_language` L57-64 |
| `wrap` | boolean | 代码块自动换行（仅 Code Block） | ❌ |
| `background_color` | string | **块级**背景色，14 种枚举（与行内背景色的 int 枚举不同！） | ❌ |
| `indentation_level` | string | 首行缩进：`NoIndent`（默认）/ `OneLevelIndent`（仅 Text Block） | ❌ |

块级 `background_color` 枚举（string 类型）：

| 值 | 描述 |
|---|---|
| `LightGrayBackground` | 浅灰 |
| `LightRedBackground` | 浅红 |
| `LightOrangeBackground` | 浅橙 |
| `LightYellowBackground` | 浅黄 |
| `LightGreenBackground` | 浅绿 |
| `LightBlueBackground` | 浅蓝 |
| `LightPurpleBackground` | 浅紫 |
| `PaleGrayBackground` | 中灰 |
| `DarkGrayBackground` | 灰 |
| `DarkRedBackground` | 中红 |
| `DarkOrangeBackground` | 中橙 |
| `DarkYellowBackground` | 中黄 |
| `DarkGreenBackground` | 中绿 |
| `DarkBlueBackground` | 中蓝 |
| `DarkPurpleBackground` | 中紫 |

### 1.3 代码块语言枚举（1-75）

飞书 CodeLanguage 枚举共 75 种。Sensend `LARK_LANG_MAP`（lark.rs L25-54）已覆盖以下映射：

| 编辑器语言 | 别名 | 飞书枚举值 | 飞书语言名 |
|---|---|:---:|---|
| plaintext / text / (空) | — | 1 | PlainText |
| bash / sh | — | 7 | Bash |
| csharp / cs / c# | — | 8 | CSharp |
| cpp / c++ | — | 9 | C++ |
| c | — | 10 | C |
| css | — | 12 | CSS |
| dockerfile | — | 18 | Dockerfile |
| go / golang | — | 22 | Go |
| html | — | 24 | HTML |
| json | — | 28 | JSON |
| java | — | 29 | Java |
| javascript / js | — | 30 | JavaScript |
| kotlin | — | 32 | Kotlin |
| lua | — | 36 | Lua |
| markdown / md | — | 39 | Markdown |
| php | — | 43 | PHP |
| perl | — | 44 | Perl |
| python / py | — | 49 | Python |
| ruby / rb | — | 52 | Ruby |
| rust / rs | — | 53 | Rust |
| scala | — | 57 | Scala |
| shell / console | — | 60 | Shell |
| swift | — | 61 | Swift |
| typescript / ts | — | 63 | TypeScript |
| yaml / yml | — | 67 | YAML |
| sql | — | 56 | SQL |
| xml | — | 66 | XML |
| toml | — | 75 | TOML |

未覆盖但飞书支持的常见语言：Dart(15)、Delphi(16)、Erlang(19)、Fortran(20)、Groovy(23)、Haskell(27)、Julia(31)、MATLAB(37)、Makefile(38)、Nginx(40)、Objective-C(41)、PowerShell(46)、Prolog(47)、R(50)、Scheme(58)、VBScript(64)、VisualBasic(65)、CMake(68)、Diff(69)、GraphQL(71) 等。未知语言统一回落 1（PlainText），不会报错。

---

## 二、飞书富文本规范

### 2.1 text_element 类型体系

飞书的 `elements` 数组中每个元素是一种 `text_element`，共 7 种类型：

| 类型 | 说明 | 支持 API 创建 | Sensend 是否使用 |
|---|---|:---:|:---:|
| `text_run` | 普通文本 | ✅ | ✅ `make_text_run` L328 |
| `mention_user` | @用户（需 user_id） | ✅ | ❌（降级为 "@标签" 文本） |
| `mention_doc` | @文档（需 doc token + obj_type） | ✅ | ❌ |
| `reminder` | 日期提醒（需时间戳 + 创建者） | ✅ | ❌ |
| `equation` | 行内公式（KaTeX 语法） | ✅ | ❌ |
| `inline_file` | 内联文件 | ❌（只支持删除/移动） | ❌ |
| `inline_block` | 内联块 | ❌（只支持删除/移动） | ❌ |

### 2.2 text_run 结构

```json
{
  "text_run": {
    "content": "文本内容",
    "text_element_style": {
      "bold": true,
      "italic": false,
      "strikethrough": false,
      "underline": true,
      "inline_code": false,
      "text_color": 5,
      "background_color": 3,
      "link": { "url": "https%3A%2F%2Fexample.com" }
    }
  }
}
```

### 2.3 text_element_style 完整字段

| 字段 | 类型 | 说明 | Sensend 是否使用 |
|---|---|---|:---:|
| `bold` | boolean | 粗体 | ✅ L336 |
| `italic` | boolean | 斜体 | ✅ L337 |
| `strikethrough` | boolean | 删除线 | ✅ L338 |
| `underline` | boolean | 下划线 | ✅ L339 |
| `inline_code` | boolean | 行内代码 | ✅ L340 |
| `link` | object `{ url: string }` | 超链接（url 需 url_encode） | ✅ L341-345 |
| `text_color` | int 1-7 | **文字颜色** | ❌ |
| `background_color` | int 1-15 | **行内背景色** | ❌ |
| `comment_ids` | string[] | 评论 ID（创建时不支持传入） | — |

**text_color 枚举**（int 类型，与 Notion 的 color 字段不同）：

| 值 | 颜色 |
|:---:|---|
| 1 | 粉色（Pink） |
| 2 | 橙色（Orange） |
| 3 | 黄色（Yellow） |
| 4 | 绿色（Green） |
| 5 | 蓝色（Blue） |
| 6 | 紫色（Purple） |
| 7 | 灰色（Grey） |

**background_color 枚举**（int 类型，行内级，注意与块级 string 枚举不同）：

| 值 | 颜色 | 深/浅 |
|:---:|---|---|
| 1 | 红 | 浅 |
| 2 | 橙 | 浅 |
| 3 | 黄 | 浅 |
| 4 | 绿 | 浅 |
| 5 | 蓝 | 浅 |
| 6 | 紫 | 浅 |
| 7 | 灰 | 中 |
| 8 | 红 | 深 |
| 9 | 橙 | 深 |
| 10 | 黄 | 深 |
| 11 | 绿 | 深 |
| 12 | 蓝 | 深 |
| 13 | 紫 | 深 |
| 14 | 灰 | 深 |
| 15 | 灰 | 浅 |

### 2.4 mention_user 结构

```json
{
  "mention_user": {
    "user_id": "ou_3bbe8a09c20e89cce9bff989ed840674",
    "text_element_style": { ... }
  }
}
```

`user_id` 类型由查询参数 `user_id_type` 决定（open_id / union_id / user_id）。

### 2.5 mention_doc 结构

```json
{
  "mention_doc": {
    "token": "doxbc873Y7cXD153gXqb76abcef",
    "obj_type": 22,
    "url": "https%3A%2F%2Fxxx.feishu.cn%2Fdocx%2Fxxx",
    "title": "文档标题",
    "text_element_style": { ... },
    "fallback_type": "FallbackToLink"
  }
}
```

`obj_type` 枚举：1=Doc, 3=Sheet, 8=Bitable, 11=MindNote, 12=File, 15=Slide, 16=Wiki, 22=Docx。

`fallback_type`：无权限时降级方式 —— `FallbackToLink`（降级为超链接）/ `FallbackToText`（降级为纯文本）。

### 2.6 equation 结构

```json
{
  "equation": {
    "content": "E=mc^2\n",
    "text_element_style": { ... }
  }
}
```

`content` 须符合 [KaTeX 语法](https://katex.org/docs/supported.html)。

### 2.7 软换行与硬换行

飞书 text_run 的 `content` 字段支持 `\n` 实现软换行（等同 Shift+Enter），但官方注明"软换行在渲染时可能被忽略，取决于渲染器处理方式"。硬换行需要创建新的 Text Block。

Sensend 当前在列表项多段落中使用 `\n` 分隔（lark.rs L452 `make_text_run("\n", &[])`），属于软换行——在飞书渲染中可能不总是生效。

---

## 三、Sensend 转换链路分析

### 3.1 架构总览

```
TipTap JSON
    │
    ▼
  ir.rs::parse()          ← 唯一遍历点，TipTap→IR
    │
    ▼
  IR Block 序列
    │
    ▼
  lark.rs::map_blocks()   ← IR→飞书 block JSON
    │
    ▼
  飞书 block 数组
    │
    ▼
  LarkAdapter::publish()  ← 分块(50)发送到飞书 API
```

### 3.2 关键函数与行号

| 函数 | 位置 | 职责 |
|---|---|---|
| `tiptap_to_lark_blocks` | lark.rs L486-490 | 入口：调用 `ir::parse` + `map_blocks` |
| `map_blocks` | lark.rs L361-445 | IR Block → 飞书 block JSON 的逐类型映射 |
| `map_list_item` | lark.rs L448-483 | 列表项 → 飞书 bullet/ordered/todo block |
| `inlines_to_elements` | lark.rs L307-325 | IR Inline → 飞书 text_element 数组 |
| `make_text_run` | lark.rs L328-353 | 构造单个 text_run（marks → text_element_style） |
| `map_language` | lark.rs L57-64 | 编辑器语言 → 飞书 CodeLanguage 枚举 |
| `append_blocks` | lark.rs L260-280 | 分块（chunks(50)）发送到飞书 API |
| `get_tenant_token` | lark.rs L92-148 | 获取 tenant_access_token（缓存 2h，提前 5min 刷新） |
| `resolve_document_id` | lark.rs L220-258 | wiki URL → document_id 解析 |
| `publish` | lark.rs L516-542 | 发布主流程：认证→解析→转换→追加→获取链接 |

### 3.3 各 IR Block 类型的映射实现

#### Paragraph（段落）— L365-374
```rust
Block::Paragraph(inlines) => {
    let mut elements = inlines_to_elements(inlines);
    if elements.is_empty() { elements.push(empty_element()); }
    out.push(json!({ "block_type": BLOCK_TEXT, "text": { "elements": elements, "style": {} } }));
}
```
- ✅ 空段落兜底为 `empty_element()`（飞书要求 elements 非空）
- ❌ `style` 固定为 `{}`，未传递 align/folded/background_color

#### Heading（标题）— L375-391
```rust
Block::Heading { level, inlines } => {
    let block_type = BLOCK_HEADING1 + level - 1;  // 3..11
    let bt = if block_type > 11 { 11 } else { block_type };
    let heading_level = if level > 9 { 9 } else { level };
    // ... 构造 heading{N} key
}
```
- ✅ 支持 1-9 级标题，超限截断为 9 级
- ✅ 动态构造 `heading1`..`heading9` key
- ❌ `style` 固定为 `{}`，未传递 align/folded

#### List（列表）— L392-396 + L448-483
```rust
Block::List { kind, items } => {
    for item in items { map_list_item(*kind, item, out); }
}
```
`map_list_item` 内部：
- ✅ 首段 + extra_paras 用 `\n` 拼接为一个 block 的多个 elements
- ✅ Task 类型使用原生 todo block（17）+ `style.done`
- ✅ 嵌套子列表拍平为同级 block（飞书 API 创建 children 需嵌套块接口）
- ❌ 嵌套列表丢失层级缩进（拍平降级）

#### CodeBlock（代码块）— L397-405
```rust
Block::CodeBlock { code, language } => {
    out.push(json!({
        "block_type": BLOCK_CODE,
        "code": {
            "elements": [{ "text_run": { "content": code } }],
            "style": { "language": map_language(language) }
        }
    }));
}
```
- ✅ 语言映射完整（34 别名 → 飞书枚举）
- ❌ `style.wrap` 未设置（默认 false，长行不换行）
- ❌ 代码内容作为单个 text_run，无长度检查

#### BlockQuote（引用）— L406-417
```rust
Block::BlockQuote(paras) => {
    for para in paras {
        let elements = inlines_to_elements(para);
        if elements.is_empty() { continue; }
        out.push(json!({ "block_type": BLOCK_QUOTE, "quote": { "elements": elements, "style": {} } }));
    }
}
```
- ✅ 多段落引用拆为多个 quote block
- ❌ 未使用 quote_container（34）嵌套结构——飞书支持将多个 block 放入 quote_container 实现真正的嵌套引用
- ❌ 空段落直接跳过（`continue`），可能改变引用块内的段落结构

#### Table（表格）— L418-436（已于 2026-08-18 重写）
```rust
Block::Table(table) => {
    // 飞书原生表格：table(31) → table_cell(32) → text(2)，发送层走 descendant API
    // 空单元格填空 text 块（官方要求 cell 必须至少含一个子块）
    ...
}
```
- ✅ **已修复**：原生 Table Block 31 + Table Cell 32，经"创建嵌套块"API（`/docx/v1/documents/:doc_id/blocks/:block_id/descendant`）创建，单次上限 1000 块
- ✅ 行内格式保留（inlines_to_elements 保留 marks）
- ✅ 参差行按首行列数补空，空表格跳过

#### HorizontalRule — L437-442
```rust
Block::HorizontalRule => {
    out.push(json!({ "block_type": BLOCK_DIVIDER, "divider": {} }));
}
```
- ✅ 正确映射

#### Inline 节点 — L307-325
```rust
fn inlines_to_elements(inlines: &[Inline]) -> Vec<Value> {
    for inline in inlines {
        match inline {
            Inline::Text { text, marks } => elements.push(make_text_run(text, marks)),
            Inline::Break => elements.push(make_text_run("\n", &[])),
            Inline::Mention(label) => elements.push(make_text_run(&format!("@{}", label), &[])),
        }
    }
}
```
- ✅ Text + marks → text_run + text_element_style
- ✅ hardBreak → `\n` 软换行
- ❌ Mention 降级为纯文本 `@标签`（飞书支持 mention_user/mention_doc 原生类型）

---

## 四、工程约束（API 限制）

### 4.1 频率限制

| 限制维度 | 上限 | 超限响应 | Sensend 处理 |
|---|---|---|:---:|
| **应用频率** | 3 req/s（单应用） | HTTP 400 + error 99991400 | ❌ 无重试 |
| **文档频率** | 3 并发编辑/s（单文档） | HTTP 429 | ❌ 无重试 |
| 获取文档信息 | 5 qps | HTTP 400 + 99991400 | — |
| 云空间修改类 | 5 qps, 10000次/天 | HTTP 400 + 99991400 | — |

飞书官方建议：使用指数退避算法处理限频。

### 4.2 批量创建限制

| 限制项 | 上限 | Sensend 处理 |
|---|---|:---:|
| children 数组长度 | 1-50 个 block/请求 | ✅ `chunks(50)` 分块发送 |
| Sheet block | 单次≤5 个 | —（未使用） |
| 表格创建 | 需使用"创建嵌套块"API（非普通 children API） | ✅ 已使用（descendant API） |

### 4.3 鉴权机制

| 项 | 说明 |
|---|---|
| Token 类型 | `tenant_access_token`（应用身份） |
| 获取方式 | POST `/auth/v3/tenant_access_token/internal`，传 app_id + app_secret |
| 有效期 | 2 小时 |
| Sensend 缓存 | 进程内 `OnceLock<Mutex<HashMap>>`，提前 5min 刷新（L136-145） |
| 请求头 | `Authorization: Bearer {token}` |

### 4.4 wiki 链接解析

飞书知识库（wiki）URL 中的 token 是 `node_token`，不是 `document_id`。需要先调用 `/wiki/v2/spaces/get_node?token={node_token}` 获取 `obj_token`（真正的 document_id）和 `obj_type`。

Sensend 已实现此逻辑（`resolve_document_id` L220-258），支持 wiki URL 和普通文档 URL 两种输入。

---

## 五、格式映射对照表

### 5.1 TipTap 节点 → 飞书 Block

| TipTap 节点 | IR Block | 飞书 block_type | 飞书 key | 状态 |
|---|---|:---:|---|:---:|
| paragraph | Paragraph | 2 | `text` | ✅ |
| heading (level=1-9) | Heading{level} | 3-11 | `heading1`-`heading9` | ✅ |
| bulletList | List{Bullet} | 12 | `bullet` | ✅ |
| orderedList | List{Ordered} | 13 | `ordered` | ✅ |
| taskList | List{Task} | 17 | `todo` | ✅ |
| codeBlock | CodeBlock | 14 | `code` | ✅ |
| blockquote | BlockQuote | 15 | `quote` | ✅（未用 quote_container） |
| table | Table | 31 + 32 | `table` + `table_cell` | ✅（原生创建） |
| horizontalRule | HorizontalRule | 22 | `divider` | ✅ |

### 5.2 TipTap Marks → 飞书 text_element_style

| TipTap Mark | IR Mark | 飞书 text_element_style | 状态 |
|---|---|---|:---:|
| bold | Bold | `bold: true` | ✅ |
| italic | Italic | `italic: true` | ✅ |
| strike | Strike | `strikethrough: true` | ✅ |
| underline | Underline | `underline: true` | ✅ |
| code | Code | `inline_code: true` | ✅ |
| link (href) | Link(String) | `link: { url: href }` | ✅ |
| — | — | `text_color: 1-7` | ❌ 未利用 |
| — | — | `background_color: 1-15` | ❌ 未利用 |

### 5.3 TipTap Inline → 飞书 text_element

| TipTap Inline | IR Inline | 飞书 element 类型 | 状态 |
|---|---|---|:---:|
| text | Text | `text_run` | ✅ |
| hardBreak | Break | `text_run`（content="\n"） | ✅（软换行） |
| mention | Mention(String) | `text_run`（content="@标签"） | ⚠️ 降级为纯文本 |
| — | — | `mention_user` | ❌ |
| — | — | `mention_doc` | ❌ |
| — | — | `equation` | ❌ |
| — | — | `reminder` | ❌ |

---

## 六、问题与改进建议

### 6.1 P1 缺陷（影响功能正确性）

> **状态**：截至 d2d8909（2026-08-18），以下 4 个 P1 均未修复。lark.rs 在该提交中无变更，飞书侧尚未跟进。

#### P1-1：无 429/99991400 限流重试

**位置**：`request()` L150-196

**问题**：飞书单文档 3 并发/s 超限返回 429，单应用 3 req/s 超限返回 400+99991400。当前 `request()` 收到非 0 code 直接返回 Err，不重试。大文档（>50 blocks 需多批次）容易触发限流。

**建议**：
- 识别 429 和 99991400 错误码
- 指数退避重试（初始 1s，最多 3 次）
- 可复用 Notion 适配器中已有的重试逻辑（如有）

#### P1-2：表格降级为文本（✅ 已修复 2026-08-18）

**位置**：`map_blocks` Table 分支 L418-436

**修复内容**：表格映射为原生 Table Block（31）+ Table Cell（32）+ text（2）三层结构，发送层含表格时自动改走"创建嵌套块"API（`/docx/v1/documents/:doc_id/blocks/:block_id/descendant`），单次上限 1000 块。空单元格按官方要求填空 text 块，参差行按首行列数补空。普通块仍走 children API（50/批），两类请求按文档顺序交错发送。

#### P1-3：引用未使用 quote_container 嵌套

**位置**：`map_blocks` BlockQuote 分支 L406-417

**问题**：飞书有 quote_container（34）支持将多个 block 嵌套在引用容器内，实现真正的引用块。当前代码将引用的每段拆为独立的 quote block（15），段落间失去引用归属关系。

**建议**：
- 先创建 quote_container block（34）
- 再通过嵌套块 API 将各段落作为 quote_container 的 children
- 或简化方案：维持多个 quote block，但在视觉上飞书会自动将连续 quote block 合并显示

#### P1-4：text_run 无长度上限检查

**位置**：`make_text_run` L328-353、`inlines_to_elements` L307-325

**问题**：飞书 API 文档未明确标注 text_run content 的字符上限，但飞书文档系统内部有 block 内容长度限制（经验值约 50000 字符/block）。超长文本可能导致 API 报错或内容截断。

**建议**：
- 对超长 text_run 做分段处理（如每 2000 字符拆分）
- 或在 IR 层面对超长段落做预警

### 6.2 P2 改进（体验优化）

#### P2-1：Mention 降级为纯文本

**位置**：`inlines_to_elements` Mention 分支 L314-317

**问题**：TipTap 的 mention 节点被降级为 `@标签` 纯文本，失去飞书原生 @用户/@文档 的交互能力。

**建议**：
- mention_user 需要 user_id（Open ID），当前 IR Mention 只有 label，缺少 user_id 信息
- 若 TipTap mention 节点的 attrs 中有 user_id，可在 IR 层扩展 Mention 变体携带
- 短期维持降级，长期在 IR 层增加 MentionUser { label, user_id } 变体

#### P2-2：嵌套列表丢失层级

**位置**：`map_list_item` L479-482

**问题**：嵌套子列表拍平为同级 block，丢失缩进层级。飞书 API 创建嵌套 block 需要两步：先创建父 block，再在其下创建 children。

**建议**：
- 使用嵌套块 API 创建层级列表
- 或在拍平的子项前加缩进提示（如 `→ ` 前缀）

#### P2-3：代码块 wrap 未设置

**位置**：`map_blocks` CodeBlock 分支 L397-405

**问题**：`style.wrap` 默认 false，长代码行不自动换行，在飞书文档中需要横向滚动查看。

**建议**：设置 `style.wrap: true`（或作为用户配置项）。

### 6.3 P3 未来增强（能力扩展）

| 飞书能力 | block_type | 说明 | 前置条件 |
|---|:---:|---|---|
| Callout（高亮块） | 19 | 带背景色的提示框，适合注意事项/Tip | API 创建支持"开发中" |
| 折叠标题/列表 | — | `style.folded: true` | TipTap 需支持折叠节点 |
| 文本对齐 | — | `style.align: 1/2/3` | TipTap 需支持 align 属性 |
| 行内颜色 | — | `text_color` / `background_color` | TipTap 需支持颜色标记 |
| 行内公式 | `equation` | KaTeX 语法 | TipTap 需支持行内公式节点 |
| @用户 | `mention_user` | 原生 @提及 | IR 层需扩展 user_id |
| @文档 | `mention_doc` | 文档间引用 | 需要目标文档 token |
| 日期提醒 | `reminder` | 日期事件提醒 | TipTap 需支持提醒节点 |
| 分栏 | 24 | Grid + Grid Column | API 创建支持"开发中" |
| 内嵌网页 | 26 | Iframe Block | API 创建支持"开发中" |
| 图片 | 27 | Image Block | API 创建支持"开发中" |

---

## 七、与 Notion 平台对比

| 维度 | 飞书 | Notion | Sensend 适配差异 |
|---|---|---|---|
| **标题层级** | 9 级（heading1-9） | 3 级（heading_1/2/3） | 飞书多 6 级，Sensend 已全支持 |
| **代码块语言** | 75 种（int 枚举） | 60+ 种（string 枚举） | 飞书用 int，Notion 用 string，Sensend 分别实现了两套映射表 |
| **原生表格** | Table Block 31（需嵌套块 API） | table block（直接 children API） | 飞书需嵌套块 API，Notion 直接支持；两者均已支持 |
| **行内颜色** | text_color(7) + background_color(15) = 22 种 | color(19) + background(19) = 38 种 | 两者都支持，Sensend 均未利用 |
| **块级背景色** | 14 种 string 枚举 | 无独立块级背景色 | 飞书有 callout 式背景色，Sensend 未利用 |
| **折叠** | folded（标题/列表/文本） | is_toggleable（标题） | 飞书覆盖更多块类型 |
| **@提及** | mention_user + mention_doc | mention（user/page/database） | 飞书区分用户和文档，Notion 统一 mention |
| **行内公式** | equation（KaTeX） | equation（KaTeX） | 两者一致，Sensend 均未利用 |
| **高亮块** | callout（19，开发中） | callout | 飞书 API 创建尚在开发中 |
| **折叠块** | 无独立 toggle block | toggle block | Notion 有，飞书用 folded 属性替代 |
| **书签** | 无 bookmark block | bookmark block | Notion 有，飞书无 |
| **引用嵌套** | quote_container（34） | toggle/quote 可嵌套 | 飞书有专门容器，Sensend 未使用 |
| **限流** | 3 req/s + 3 并发/s/文档 | 3 req/s | 两者类似，Sensend 飞书无重试，Notion 也无重试 |
| **批量上限** | 50 blocks/请求 | 100 blocks/请求 | 飞书更严格 |
| **鉴权** | app_id+app_secret → tenant_token | API Key（Bearer） | 飞书多一步 token 获取 |
| **文档模式** | 仅追加（append） | 追加 + 创建页面 | 飞书不支持创建新文档（只能追加到已有文档） |

---

## 八、测试覆盖现状

Sensend 飞书适配器已有以下测试（lark.rs L550-660）：

### Golden Tests（快照测试）
| 测试名 | 覆盖场景 |
|---|---|
| `simple_paragraph` | 基础段落 |
| `headings` | 多级标题 |
| `nested_list` | 嵌套列表（拍平验证） |
| `table_with_inline` | 表格（原生 table 块 + descendant API） |
| `hardbreak` | 软换行 |
| `tasklist` | 待办列表 |
| `codeblock` | 代码块 |
| `blockquote` | 引用 |
| `long_title` | 长标题 |
| `underline_link` | 下划线+链接组合 |
| `combined` | 综合场景 |

### 目标断言测试
| 测试名 | 验证点 |
|---|---|
| `fix_s3_todo_uses_native_block` | 待办使用原生 todo block(17) + style.done |
| `fix2_nested_list_flattened_not_dropped` | 嵌套子项拍平输出不丢失 |
| `fix3_list_item_multi_paragraph_kept` | 列表项多段落用 \n 保留 |
| `b2_language_mapping` | 语言映射大小写不敏感 + 未知回落 PlainText |

### 未覆盖的测试场景
- 429 限流重试（无 mock server）
- 超长 text_run 分段
- ~~表格原生 Table Block 创建（未实现）~~（✅ 2026-08-18 已实现）
- quote_container 嵌套引用（未实现）
- mention_user/mention_doc 原生提及（未实现）
- wiki URL 解析（需 mock API）
