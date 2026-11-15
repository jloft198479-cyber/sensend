# Sensend 待办事项

> 本文件记录已发现但暂未处理的问题与改进点，便于后续统一处理。
> 新发现的问题追加到对应分类末尾即可，处理完成后移至「已处理」归档区。

---

## 待处理

### 表格功能前端入口缺失

- **发现时间**：2026-07-31（v0.3.0 发布后用户反馈）
- **现象**：用户下载 v0.3.0 测试，看不到表格支持
- **根因**：前端没有任何"插入表格"的入口，但编辑器内核已注册 TableKit，粘贴表格能正常解析

**现状（半成品）**：

| 层级 | 文件 | 状态 |
|------|------|------|
| 编辑器内核 | `src/composables/useEditor.ts#L171` | ✅ 已注册 TableKit |
| 粘贴富文本 HTML | `src/composables/useEditor.ts` | ✅ 可解析表格 |
| 粘贴纯文本 GFM 表格 | `src/composables/useEditor.ts` | ✅ 已修复（07a7be3：正则补表格分隔行） |
| 表格显示样式 | `src/styles/editor.css` | ✅ 已修复（07a7be3：边框/表头/选中高亮） |
| 格式化操作 | `src/composables/useEditorFormat.ts` | ❌ 无 insertTable |
| 工具栏按钮 | `src/App.vue#L176-L211` | ❌ 无表格按钮 |
| 快捷键 | `src/composables/useHotkey.ts` | ❌ 无表格快捷键 |
| 后端适配器 markdown/notion/flowus/local | `src-tauri/src/adapters/*.rs` | ✅ 转换逻辑齐全 |
| 后端适配器 lark | `src-tauri/src/adapters/lark.rs#L418-L436` | ⚠️ 飞书无原生表格，降级为 `\|` 分隔文本 |

**修复方案（最小改动）**：

1. `useEditorFormat.ts` 加 `insertTable` 方法：`editor.chain().focus().insertTable({ rows: 3, cols: 3 }).run()`
2. `App.vue` BubbleMenu 加"插入表格"按钮

**关联说明**：编辑器内核已注册 TableKit，粘贴链路已通（HTML 表格和 GFM 表格均可解析），仅剩手动创建入口。

---

### 前端空检查正则对带空格的 @实例名失效

- **发现时间**：2026-08-18（第三方复查 ir.rs 尾部剥除逻辑时发现）
- **现象**：实例名含空格（如 `我的 Notion`）时，发送前空检查的正则 `/(?<!\S)@\S+/g` 只剥到第一个空格，残留 ` Notion` 使检查误判为有内容而放行；后端 stripMentions 后全文只剩空白 → 空 IR → 适配器兜底发出一条空内容并提示成功（"成功的空发送"）
- **位置**：`src/composables/usePlatform.ts#L76`
- **修复方向**：空检查改用与 stripMentions 同源的判定（遍历 JSON 剔除 mention 节点后再判空），而非文本正则
- **严重度**：低（需实例名含空格且正文只有该 mention 才触发），暂不修，记此备查

---

### 平台发送健壮性（2026-08-19 从 EXPERIENCE §四 并入，集中维护）

以下为各平台已识别但暂不修复的发送健壮性问题，统一在此排队，不在其他文档重复维护。

| 平台 | 问题 | 严重度 | 状态 | 出处 |
|------|------|:---:|------|------|
| Notion | 429 限流无重试（读 Retry-After 后重试，最多 3 次） | P3 | ❌ 未修复 | NOTION-FORMAT-SPEC.md §6.1 P3 |
| 飞书 | 429/99991400 限流无重试 | P1 | ❌ 未修复 | FEISHU-GUIDE.md §5.6 P1 |
| 飞书 | 引用未用 quote_container 嵌套（段落间失去引用归属） | P1 | ❌ 未修复 | FEISHU-GUIDE.md §5.6 P1 |
| 飞书 | text_run 无长度上限检查 | P1 | ❌ 未修复 | FEISHU-GUIDE.md §5.6 P1 |
| 飞书 | 嵌套列表拍平丢层级 | P2 | ❌ 未修复 | FEISHU-GUIDE.md §5.6 P2 |
| 飞书 | 代码块 wrap 未设置（长代码行不自动换行） | P2 | ❌ 未修复 | FEISHU-GUIDE.md §5.6 P2 |

---

### 外部借鉴 · wandao 万能导（2026-09-02 评估，待展开）

- **来源**：`G:\skills\wandao-main`（开源项目源码，**不是** WorkBuddy 技能，无 SKILL.md）
- **它是什么**：多平台知识库批量导入导出搬迁工具（飞书 / 语雀 / Notion / 印象笔记等），Tauri 2 + HTML/JS + Python sidecar，1.4.x
- **与 sensend 关系**：方向相反 —— 它是「批量搬迁」，我们是「单条速记分发」。仅局部可借鉴，不可整体照抄

**待办清单**（按价值排序，均未启动）：

| # | 待办 | 落点 | 严重度 | 参考实现 |
|---|------|------|:---:|---------|
| 1 | 平台凭证明文存储 → 加密 | `src-tauri/src/commands/platform.rs`：`token_memory`、`platform_instances` 经 tauri-plugin-store **明文写入** `config.json`（Notion token、飞书 App ID/Secret 全在里面） | P1 | wandao `src-tauri/src/security.rs` 的 `protect_bytes` / `unprotect_bytes`（Windows 用户级 DPAPI / macOS 钥匙串，含旧密文兼容迁移） |
| 2 | 缺应用内自动更新 | `src-tauri/tauri.conf.json`（无 `plugins.updater`）、`src-tauri/Cargo.toml`（无 tauri-plugin-updater）；当前用户只能手动下载 exe | P2 | wandao `tauri.conf.json` 的 `plugins.updater`：minisign 公钥 + `latest.json` 端点 + 强制签名校验 |
| 3 | CSP 过松 | `src-tauri/tauri.conf.json` 的 `app.security.csp`：`script-src` 含 `unsafe-eval`、`connect-src` 对全网 `https:` 放行、缺 `object-src` / `base-uri` / `form-action` | P2 | wandao 同字段：无 unsafe-eval、补齐三项限制、另加 `freezePrototype: true` |
| 4 | 配置写入非原子 | `note.json` 800ms 防抖直写，崩溃可能留半截文件 | P3 | wandao `security.rs` 的 `write_json_atomic` / `write_private_atomic`（临时文件 + rename） |
| 5 | 本地文件名边界未覆盖 | `src-tauri/src/adapters/local.rs#L13` `safe_title` 仅替换 `\/:*?"<>\|` | P3 | wandao `security.rs::safe_relative_path`：长度上限（Windows 260）、结尾空格/点、保留名（con/nul/aux/com1 等） |

**明确不做**（记此防下次重提）：

| 项 | 原因 |
|---|---|
| 插件中心 / Provider manifest / 任务中心 / checkpoint 断点续跑 / 增量跳过 / 内嵌 Python runtime | 属「批量搬迁」的重型机制，套在单条速记上即过度工程 |

> ⚠️ **AGPL-3.0 红线（长期有效，待办处理完也不删除）**：wandao 为 AGPL-3.0，sensend 为 MIT。直接复制其 Rust 源码会触发协议传染，法律上要求 sensend 整体以 AGPL 开源。**只能借鉴思路、自行重新实现，不得搬运文件。**

---

## 已处理（归档）

### 格式 P1 三连修（2026-08-19 提交 d09a413）

| 项目 | 状态 |
|------|------|
| FlowUs 表格单元格多 rich_text 丢失（`rt[0]` → `row_cells.extend(rt)`） | ✅ 已修复 |
| Notion 代码块语言映射（`NOTION_LANG_MAP`，40 个别名、大小写不敏感、兜底纯文本） | ✅ 已修复 |
| Notion 嵌套列表深度截断（上限 2 层，第 3 层起子项文本并入父项） | ✅ 已修复 |
