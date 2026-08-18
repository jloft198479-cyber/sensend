# 飞书对接指南（Sensend）

> 给"明天就忘"的自己：飞书这套机制不需要背，知道往哪查就行。本文按「逻辑 → 配置 → 排查 → 格式规范」四段写，重点把最容易晕的**权限模型**和**格式映射**讲透。
> 适用场景：sensend 通过「企业自建应用」把笔记追加（Append）到指定的飞书文档 / 知识库页面。
> 本文已并入原 `FEISHU-FORMAT-SPEC.md`（格式规范调研，2026-08-18）全部内容。

---

## 〇、三句话先记住

1. sensend 用的是「应用身份」（App ID + App Secret），不碰你的飞书账号密码。
2. 能不能写一份文档，取决于 **三个条件同时满足**：API 权限（Scope）✅ + 权限已发布 ✅ + 应用被加为文档/知识库协作者 ✅。
3. **读和写是两套独立权限**，可能出现「能读不能写」——这就是 2026-08-17 排查到的真实问题。

---

## 一、飞书的整套逻辑（大白话版）

### 1.1 身份：到底是谁在访问？

- App ID / App Secret 相当于「公司门禁卡 + 密码」。
- 用它们换来的 `tenant_access_token` 相当于「临时通行证」，**2 小时有效**。
- 通行证代表「应用自己」，不是某个员工。所以 sensend 干的每件事，飞书都理解为「这个应用在操作」。

一次完整流程（对应 [lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L499-L543)）：

```
App ID + App Secret
        │  ① 换通行证
        ▼
tenant_access_token（2 小时，进程内缓存）
        │  ② 拿通行证去开门
        ▼
读：解析 wiki 链接 → 找到文档 → 读取文档
写：把笔记转成 block → 追加到文档末尾
```

### 1.2 权限 = 三扇门，全开才能写

这是飞书最容易晕的地方：**光有 token 没用，要一层一层把门打开**。

| 门 | 在哪设置 | 管什么 | 最容易踩的坑 |
|---|---|---|---|
| ① **API 权限 Scope** | 开放平台 → 权限管理 | 这个应用「能不能调用某个接口」（比如能不能写文档） | 加了权限但**没发布版本**，线上不生效 |
| ② **版本发布** | 开放平台 → 版本管理与发布 | 让 ① 的权限真正生效 | 改完权限忘了发版 = 白改 |
| ③ **资源协作** | 飞书客户端，把应用加进文档/知识库协作者 | 这个应用「能不能碰到那份**具体**文档」 | 有权限但没加协作者，连看都看不了 |

打个比方：

- ① = 你**会不会开车**（驾照）
- ② = 驾照有没有**年审**（过期了照扣）
- ③ = 你有没有**这辆车的钥匙**（驾照再全，没钥匙也开不走）

三样缺一不可。

### 1.3 为什么「能读不能写」？（2026-08-17 踩的坑）

- 读文档只需要权限 A；写文档（追加块）需要权限 A + 写权限 B。
- 2026-08-17 实测：**读正常，写返回 `403 / code=1770032 / forBidden`**，连最普通的 text 块都被拒。
- 说明该应用只有读权限，没有写权限（或写了权限但没发布）。
- **注意**：sensend 配置窗口的「测试连接」按钮只做了**读取**验证（拿 token → 解析 wiki → 读文档，见 [lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L499-L514)），**从不测写入**。所以「测试连接显示正常」≠「能写入」。真正的写入验证 = 实际发一条内容。

### 1.4 几个名词别搞混

| 名词 | 是什么 | sensend 里对应 |
|---|---|---|
| App ID | 应用身份证号 | 配置里的 **token** 字段 |
| App Secret | 应用密码 | 配置里的 **token2** 字段 |
| tenant_access_token | 应用身份的临时通行证（2h） | 每次 API 调用都用它 |
| node_token | 知识库页面的「地址编号」 | 从 wiki 链接里取（URL 最后一段） |
| document_id / obj_token | 真正文档的「身份证号」 | 写入文档时用 |
| Block | 文档里一段一段的内容块 | sensend 把笔记转成 block 再追加 |

wiki 链接解析流程（[lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L220-L258)）：

```
粘贴的 wiki 链接: https://xxx.feishu.cn/wiki/GydowmeXUiXRWIk3vFicLtHVngd
                          │  提取最后一段
                          ▼
node_token = GydowmeXUiXRWIk3vFicLtHVngd
                          │  调 get_node 解析
                          ▼
obj_token = QNxAdoxSaom98ExNim2cCG0Wn9c  ← 这就是文档的 document_id，用它追加内容
```

### 1.5 为什么 sensend 只能「追加」不能「覆盖/编辑」

- 用应用身份（tenant_access_token）写文档，飞书只允许做「在文档末尾加块」这类温和操作。
- 这是飞书的限制，不是 bug。所以 sensend 的飞书只支持 `page`（追加）模式。

### 1.6 内容长什么样？（block_type 映射，以代码为准）

飞书文档 = 一个根块 + 无数子块。sensend 把富文本笔记转成这些块（常量定义在 [lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L13-L21)）：

| block_type | 含义 | sensend 从哪来 |
|---|---|---|
| 2 | 文本 | 段落 |
| 3~11 | 标题（heading1~9） | 标题 |
| 12 | 无序列表 bullet | 项目符号列表 |
| 13 | 有序列表 ordered | 编号列表 |
| 14 | 代码 code | 代码块 |
| 15 | 引用 quote | 引用 |
| 17 | 待办 todo（style.done 控制勾选） | 任务列表 |
| 22 | 分割线 divider | 分隔线 |

> 注：历史文档 CODE-WIKI.md 曾把 code 写成 17，**实际 code=14、todo=17**（以 lark.rs 代码为准）。

---

## 二、如何有效配置（照抄即可）

### 2.1 飞书开放平台侧（一次配置，长期有效）

1. 打开 https://open.feishu.cn → 创建 **企业自建应用**，填个名字（如 "Agent整理笔记"）。
2. 左侧「凭证与基础信息」→ 记下 **App ID** 和 **App Secret**（Secret 只完整显示一次，立即备份）。
3. 左侧「权限管理」→ 搜索并开通以下**租户权限**：
   - `docx:document`（创建及编辑新版文档）— 文档读写，写文档的必须
   - `wiki:wiki`（查看、编辑和管理知识库）— 因为目标是知识库页面，解析链接需要它
4. **关键一步**：左侧「版本管理与发布」→ 创建版本 → 申请发布。
   - 个人自建应用通常免审 / 自动通过；企业应用需管理员审核。
   - **不发布 = 上面开的权限全部无效**。
5. 给应用「分钥匙」（资源协作）：
   - 目标是知识库页面 → 进知识库 → 设置 → 成员与权限 → 添加成员 → 选「机器人」→ 搜你的应用 → 授权 **可编辑**（至少编辑权限，才能写入）。
   - 目标是普通云文档 → 文档右上角「...」→ 更多 → 添加文档应用 → 搜应用 → 授予**可编辑**。

### 2.2 Sensend 侧

1. 打开配置窗口 → 新增/编辑「飞书」实例。
2. `App ID` 填入 **token** 字段，`App Secret` 填入 **token2** 字段（对应 [config.json](file:///c:/Users/fzz198479.NOVA/AppData/Roaming/com.jloft.sensend/config.json) 里 lark 实例的 token / token2）。
3. 目标粘贴 wiki 链接：`https://xxx.feishu.cn/wiki/xxxx`。
4. 点「测试连接」——只验证读权限，成功即可保存。
5. **再真发一条内容**，验证写权限（这一步才算闭环）。

### 2.3 验证权限是否生效（可选）

用 PowerShell 直接调飞书 API 自查，最快：

```powershell
# 1) 换 token
$body = @{ app_id = '<你的App ID>'; app_secret = '<你的App Secret>' } | ConvertTo-Json
$r = Invoke-RestMethod -Uri 'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal' -Method Post -ContentType 'application/json; charset=utf-8' -Body $body
$tok = $r.tenant_access_token
$h = @{ Authorization = "Bearer $tok" }

# 2) 读：解析 wiki 链接（拿 node_token）
$node = Invoke-RestMethod -Uri 'https://open.feishu.cn/open-apis/wiki/v2/spaces/get_node?token=<node_token>' -Headers $h
$doc = $node.data.node.obj_token

# 3) 读：读取文档
Invoke-RestMethod -Uri "https://open.feishu.cn/open-apis/docx/v1/documents/$doc" -Headers $h

# 4) 写：追加一个文本块（验证写权限！）
$payload = @{ children = @( @{ block_type = 2; text = @{ elements = @( @{ text_run = @{ content = "权限验证" } } ); style = @{} } } ) } | ConvertTo-Json -Depth 10
Invoke-RestMethod -Uri "https://open.feishu.cn/open-apis/docx/v1/documents/$doc/blocks/$doc/children" -Method Post -Headers $h -ContentType 'application/json; charset=utf-8' -Body $payload
```

第 4 步返回 `code=0` 才算写入权限 OK；返回 `403` 就是权限问题。

---

## 三、关键问题排查表

| 现象 | 大概率原因 | 处理 |
|---|---|---|
| 提示「无权限访问目标，请检查 Token 权限」（本质是 403） | **写入权限缺失**：没开写权限 / 权限没发布 / 应用只有读权限 / 被移出协作者 | 按排查顺序检查下面「三扇门」 |
| 认证失败（code≠0） | App ID / Secret 填错、抄错、带空格 | 重新复制凭证，比对 config.json |
| 报「wiki 中嵌入的类型不支持」 | 链接指向的不是文档（如电子表格、多维表） | 换成普通文档 / 文档型 wiki 页 |
| 发送「成功」但飞书里看不到 | 追加模式，内容加在文档**末尾** | 刷新页面，往下滚动 |
| 换了 Secret 但行为没变 | 进程内 token 缓存 2 小时未过期 | 重启应用（缓存即清） |
| 报频控错误（99991400 / HTTP 429） | 超频率限制：单应用 3 次/秒、单文档 3 次/秒 | 稍等重试；sensend 单次发送请求很少，正常不会触发 |

### 排查顺序口诀

> **先看权限开没开 → 再看发没发版 → 再看协作者加没加 → 最后才怀疑代码。**

展开讲：
1. 权限管理里有没有 `docx:document`（写文档）和 `wiki:wiki`（解析知识库）？
2. 版本管理与发布里，最近一次发版是否在「改权限之后」？
3. 飞书客户端里，目标文档/知识库的协作者列表里有没有这个应用？权限是「可编辑」还是只有「可查看」？
4. 以上都对还不行，再用 2.3 的脚本直接调 API，看具体错误码和 msg。

---

## 四、Sensend 实现对照（给改代码的人）

- 适配器文件：[lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs)
- 调用流程：
  1. `get_tenant_token` — App ID+Secret 换 token，**进程内缓存 2 小时**（提前 5 分钟刷新）
  2. `resolve_document_id` — 识别 wiki 链接 → `get_node` 解析出 document_id
  3. `append_blocks` — 追加块，**每批最多 50 个 block**（`chunks(50)` 分批）；含表格时走 `descendant` 嵌套块 API（单次上限 1000 块）
  4. `get_file_url` — 取文档链接用于跳转
- 错误语义（后端回传给前端）：
  - `飞书认证失败 (code=...)` → token 获取环节出错
  - `飞书 API 错误 (code=...): msg` → 业务接口报错（如 1770032 = 权限不足）
  - `HTTP 错误 (403)` → 接口级 403
- 前端错误翻译在 [usePlatform.ts](file:///f:/fzz-Project/sensend/sensend/src/composables/usePlatform.ts#L66-L72)：401→「Token 过期」、403/forbidden→「无权限访问目标」、429→「请求频繁」、网络→「网络失败」。**注意 403 类信息会被翻译成「无权限」，不代表 token 失效**。

---

## 五、飞书格式规范（并入原 FEISHU-FORMAT-SPEC.md）

> 本节基于飞书开放平台文档 API 的格式展示要求，对照 Sensend 的 TipTap→飞书转换链路，列出差异、缺陷与改进方向。
> 飞书 API 基线：`open.feishu.cn/open-apis/docx/v1`（代码中 `FEISHU_BASE` 常量）。
> 飞书 API 文档：https://open.feishu.cn/document/server-docs/docs/docs/docx-v1/document-block/create

**结论先行**：
1. **核心格式覆盖完整**：Sensend 支持的 TipTap 节点（段落、标题1-9、无序/有序/待办列表、代码块、引用、分割线、表格）和行内样式（粗/斜/删/下划线/行内码/链接）全部映射到飞书 block；代码块语言映射表覆盖 34 个别名、75 种飞书枚举语言中的常用 30+ 种。
2. **已修复**：表格曾降级为文本 → 现已用原生 `table(31)` + `table_cell(32)` 块 + `descendant` API 创建（d09a413）。
3. **未修复缺陷**：无 429 限流重试、引用未用 quote_container、text_run 无长度上限、Mention 降级纯文本、嵌套列表拍平丢层级、代码块 wrap 未设置——见 [§5.6](#56-已知缺陷与改进)。
4. **飞书 vs Notion 差异显著**：飞书支持 9 级标题（Notion 仅 3 级）、原生表格走嵌套块 API、块级背景色 14 种；但飞书无 toggle / bookmark。

### 5.1 Block 类型体系

#### 5.1.1 完整 Block 类型清单

飞书 docx API 支持以下 block 类型（`/docx/v1/documents/:document_id/blocks/:block_id/children`）：

| block_type | 类型名 | 支持 API 创建 | 支持 rich_text | Sensend 是否使用 |
|:---:|---|:---:|:---:|:---:|
| 1 | Page（页面根块） | ❌ | ✅ | —（根块，不可创建） |
| 2 | Text（文本段落） | ✅ | ✅ | ✅ `BLOCK_TEXT` |
| 3~11 | Heading 1~9 | ✅ | ✅ | ✅ `BLOCK_HEADING1`+ |
| 12 | Bullet（无序列表） | ✅ | ✅ | ✅ `BLOCK_BULLET` |
| 13 | Ordered（有序列表） | ✅ | ✅ | ✅ `BLOCK_ORDERED` |
| 14 | Code（代码块） | ✅ | ✅ | ✅ `BLOCK_CODE` |
| 15 | Quote（引用） | ✅ | ✅ | ✅ `BLOCK_QUOTE` |
| 17 | Todo（待办） | ✅ | ✅ | ✅ `BLOCK_TODO` |
| 18-20, 23-29, 35-52 | Bitable/Callout/Diagram/Grid/Iframe/Image/Sheet/表格/View 等 | 多数"开发中" | — | ❌ |
| 22 | Divider（分割线） | ✅ | — | ✅ `BLOCK_DIVIDER` |
| 30 | Sheet（电子表格） | ✅（单次≤5） | — | ❌ |
| 31 | Table（表格） | ✅（需嵌套块 API） | — | ✅（descendant API 原生创建） |
| 32 | Table Cell（单元格） | ✅（需嵌套块 API） | ✅ | ✅（随表格创建） |
| 34 | Quote Container（引用容器） | ✅ | — | ❌ |

> 完整枚举见飞书官方 BlockType 枚举。Callout(19)、分栏(24)、内嵌网页(26)、图片(27) 等 API 创建多为"开发中"，Sensend 未使用。

#### 5.1.2 Block 级 style（text_style）字段

所有支持 rich_text 的 block 共享一个 `style` 对象：

| 字段 | 类型 | 说明 | Sensend 是否使用 |
|---|---|---|:---:|
| `align` | int | 对齐：1=左、2=居中、3=右 | ❌ |
| `done` | boolean | Todo 勾选态 | ✅ `map_list_item` |
| `folded` | boolean | 折叠状态 | ❌ |
| `language` | int | 代码块语言枚举 1-75 | ✅ `map_language` |
| `wrap` | boolean | 代码块自动换行 | ❌ |
| `background_color` | string | 块级背景色（14 种 string 枚举） | ❌ |
| `indentation_level` | string | 首行缩进 | ❌ |

#### 5.1.3 代码块语言枚举（1-75）

`LARK_LANG_MAP`（lark.rs L25-54）已覆盖：plaintext/text/(空)→1 PlainText、bash/sh→7、csharp/cs/c#→8、cpp/c++→9、c→10、css→12、dockerfile→18、go/golang→22、html→24、json→28、java→29、javascript/js→30、kotlin→32、lua→36、markdown/md→39、php→43、perl→44、python/py→49、ruby/rb→52、rust/rs→53、scala→57、shell/console→60、swift→61、typescript/ts→63、yaml/yml→67、sql→56、xml→66、toml→75。

未覆盖但飞书支持的常见语言：Dart(15)、Delphi(16)、Erlang(19)、Fortran(20)、Groovy(23)、Haskell(27)、Julia(31)、MATLAB(37)、Makefile(38)、Nginx(40)、Objective-C(41)、PowerShell(46)、Prolog(47)、R(50)、Scheme(58)、VBScript(64)、VisualBasic(65)、CMake(68)、Diff(69)、GraphQL(71) 等。未知语言统一回落 1（PlainText），不会报错。

### 5.2 飞书富文本规范

#### 5.2.1 text_element 类型体系

`elements` 数组每个元素是一种 `text_element`，共 7 种：

| 类型 | 说明 | 支持 API 创建 | Sensend 是否使用 |
|---|---|:---:|:---:|
| `text_run` | 普通文本 | ✅ | ✅ `make_text_run` |
| `mention_user` | @用户 | ✅ | ❌（降级为 "@标签" 文本） |
| `mention_doc` | @文档 | ✅ | ❌ |
| `reminder` | 日期提醒 | ✅ | ❌ |
| `equation` | 行内公式（KaTeX） | ✅ | ❌ |
| `inline_file` / `inline_block` | 内联文件/块 | ❌（只读） | ❌ |

#### 5.2.2 text_run 结构

```json
{
  "text_run": {
    "content": "文本内容",
    "text_element_style": {
      "bold": true, "italic": false, "strikethrough": false,
      "underline": true, "inline_code": false,
      "text_color": 5, "background_color": 3,
      "link": { "url": "https%3A%2F%2Fexample.com" }
    }
  }
}
```

#### 5.2.3 text_element_style 完整字段

| 字段 | 类型 | 说明 | Sensend 是否使用 |
|---|---|---|:---:|
| `bold` / `italic` / `strikethrough` / `underline` / `inline_code` | boolean | 粗/斜/删/下划线/行内码 | ✅ |
| `link` | object `{ url }`（url 需 url_encode） | 超链接 | ✅ |
| `text_color` | int 1-7 | 文字颜色 | ❌ |
| `background_color` | int 1-15 | 行内背景色（与块级 string 枚举不同） | ❌ |
| `comment_ids` | string[] | 评论 ID（创建时不支持传入） | — |

#### 5.2.4 软换行与硬换行

飞书 `text_run.content` 支持 `\n` 实现软换行（等同 Shift+Enter），但官方注明"软换行在渲染时可能被忽略"。硬换行需创建新的 Text Block。Sensend 列表项多段落使用 `\n` 拼接（软换行，飞书渲染中可能不总生效）。

### 5.3 Sensend 转换链路分析

```
TipTap JSON → ir.rs::parse()（唯一遍历点）→ IR Block 序列 → lark.rs::map_blocks() → 飞书 block 数组 → LarkAdapter::publish()
```

| 函数 | 位置 | 职责 |
|---|---|---|
| `tiptap_to_lark_blocks` | lark.rs | 入口：`ir::parse` + `map_blocks` |
| `map_blocks` | lark.rs | IR Block → 飞书 block JSON 逐类型映射 |
| `map_list_item` | lark.rs | 列表项 → bullet/ordered/todo block |
| `inlines_to_elements` | lark.rs | IR Inline → 飞书 text_element 数组 |
| `make_text_run` | lark.rs | 构造单个 text_run（marks → text_element_style） |
| `map_language` | lark.rs | 编辑器语言 → 飞书 CodeLanguage 枚举 |
| `append_blocks` | lark.rs | 分块（chunks(50)）发送；含表格走 descendant API |
| `get_tenant_token` | lark.rs | 获取 tenant_access_token（缓存 2h，提前 5min 刷新） |
| `resolve_document_id` | lark.rs | wiki URL → document_id 解析 |
| `publish` | lark.rs | 发布主流程：认证→解析→转换→追加→获取链接 |

各 IR Block 类型映射要点：
- **Paragraph**：空段落兜底为 `empty_element()`（飞书要求 elements 非空）；`style` 固定 `{}`。
- **Heading**：支持 1-9 级，超限截断为 9 级；动态构造 `heading1..heading9` key。
- **List**：Task 用原生 todo(17) + `style.done`；嵌套子列表拍平为同级 block（丢层级，见 §5.6 P2）。
- **CodeBlock**：语言映射完整；`style.wrap` 未设置（默认不换行）。
- **BlockQuote**：多段落拆为多个 quote(15) block，未用 quote_container(34)。
- **Table**（已重写 d09a413）：`table(31)` → `table_cell(32)` → `text(2)` 三层，发送层走 descendant API；空单元格填空 text 块（官方要求 cell 至少一个子块）；参差行按首行列数补空；空表格跳过；行内格式保留。
- **HorizontalRule**：→ divider(22)。
- **Inline**：Text+marks → text_run + style；hardBreak → `\n` 软换行；Mention 降级为纯文本 `@标签`。

### 5.4 工程约束（API 限制）

#### 5.4.1 频率限制

| 限制维度 | 上限 | 超限响应 | Sensend 处理 |
|---|---|---|:---:|
| 应用频率 | 3 req/s（单应用） | HTTP 400 + 99991400 | ❌ 无重试 |
| 文档频率 | 3 并发编辑/s（单文档） | HTTP 429 | ❌ 无重试 |
| 获取文档信息 | 5 qps | HTTP 400 + 99991400 | — |

飞书官方建议使用指数退避算法处理限频。

#### 5.4.2 批量创建限制

| 限制项 | 上限 | Sensend 处理 |
|---|---|:---:|
| children 数组长度 | 1-50 个 block/请求 | ✅ `chunks(50)` 分块 |
| Sheet block | 单次≤5 | —（未使用） |
| 表格创建 | 需"创建嵌套块"API（descendant） | ✅ 已使用 |

#### 5.4.3 鉴权机制

| 项 | 说明 |
|---|---|
| Token 类型 | `tenant_access_token`（应用身份） |
| 获取 | POST `/auth/v3/tenant_access_token/internal`，传 app_id + app_secret |
| 有效期 | 2 小时 |
| Sensend 缓存 | 进程内 `OnceLock<Mutex<HashMap>>`，提前 5min 刷新 |
| 请求头 | `Authorization: Bearer {token}` |

#### 5.4.4 wiki 链接解析

飞书知识库 URL 的 token 是 `node_token`，不是 `document_id`。需先调 `/wiki/v2/spaces/get_node?token={node_token}` 拿 `obj_token`（真正的 document_id）和 `obj_type`。Sensend 已实现（`resolve_document_id`），支持 wiki URL 和普通文档 URL 两种输入。

### 5.5 格式映射对照表

#### 5.5.1 TipTap 节点 → 飞书 Block

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

#### 5.5.2 TipTap Marks / Inline → 飞书 text_element

| TipTap Mark | IR Mark | 飞书 text_element_style | 状态 |
|---|---|---|:---:|
| bold | Bold | `bold: true` | ✅ |
| italic | Italic | `italic: true` | ✅ |
| strike | Strike | `strikethrough: true` | ✅ |
| underline | Underline | `underline: true` | ✅ |
| code | Code | `inline_code: true` | ✅ |
| link (href) | Link(String) | `link: { url: href }` | ✅ |
| — | — | `text_color` / `background_color` | ❌ 未利用 |
| hardBreak | Break | `text_run`（content="\n"） | ✅（软换行） |
| mention | Mention(String) | `text_run`（content="@标签"） | ⚠️ 降级为纯文本 |

### 5.6 已知缺陷与改进

> 状态基准：2026-08-18 调研 + 2026-08-19 P1 修复提交（d09a413）后。

| 编号 | 缺陷 | 严重度 | 状态 |
|---|---|---|:---:|
| P1-1 | 无 429/99991400 限流重试（`request()` 直接返回 Err） | P1 | ❌ 未修复 |
| P1-2 | 表格降级为文本 | P1 | ✅ 已修复（原生 table + descendant，d09a413） |
| P1-3 | 引用未用 quote_container(34) 嵌套 | P1 | ❌ 未修复 |
| P1-4 | text_run 无长度上限检查（经验值约 50000 字符/block） | P1 | ❌ 未修复 |
| P2-1 | Mention 降级为纯文本（需 IR 扩展 user_id） | P2 | ❌ 未修复 |
| P2-2 | 嵌套列表拍平丢层级 | P2 | ❌ 未修复 |
| P2-3 | 代码块 wrap 未设置（长行不换行） | P2 | ❌ 未修复 |
| P3 | Callout/折叠/对齐/行内颜色/公式/@提及/分栏/图片等能力未利用 | P3 | ❌ 未来增强 |

### 5.7 与 Notion 平台对比

| 维度 | 飞书 | Notion | Sensend 适配差异 |
|---|---|---|---|
| 标题层级 | 9 级 | 3 级 | 飞书多 6 级，Sensend 全支持 |
| 代码块语言 | 75 种（int 枚举） | 60+ 种（string 枚举） | 两套映射表分别实现 |
| 原生表格 | Table(31) 需嵌套块 API | 直接 children API | 均已支持 |
| 行内颜色 | 7 文字色 + 15 背景色 | 19+19 | 均未利用 |
| 折叠 | folded 属性 | is_toggleable / toggle | 飞书覆盖更多类型 |
| @提及 | mention_user + mention_doc | mention（user/page/db） | 均降级纯文本 |
| 限流 | 3 req/s + 3 并发/s/文档 | 3 req/s | 均无重试 |
| 批量上限 | 50 blocks/请求 | 100 blocks/请求 | 飞书更严格 |
| 鉴权 | app_id+secret → tenant_token | API Key | 飞书多一步 token 获取 |
| 文档模式 | 仅追加 | 追加 + 创建页面 | 飞书不支持创建新文档 |

### 5.8 测试覆盖现状

飞书适配器（lark.rs）测试：

**Golden Tests（快照）**：`simple_paragraph` / `headings` / `nested_list` / `table_with_inline` / `hardbreak` / `tasklist` / `codeblock` / `blockquote` / `long_title` / `underline_link` / `combined`。

**目标断言测试**：
| 测试名 | 验证点 |
|---|---|
| `fix_s3_todo_uses_native_block` | 待办用原生 todo(17) + style.done |
| `fix2_nested_list_flattened_not_dropped` | 嵌套子项拍平输出不丢失 |
| `fix3_list_item_multi_paragraph_kept` | 列表项多段落用 \n 保留 |
| `b2_language_mapping` | 语言映射大小写不敏感 + 未知回落 PlainText |

**未覆盖场景**：429 限流重试（无 mock server）、超长 text_run 分段、quote_container 嵌套引用、mention_user/mention_doc 原生提及、wiki URL 解析（需 mock API）。
