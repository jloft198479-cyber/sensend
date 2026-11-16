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

**待办清单**：四项已全部完成（#1 随 v0.5.1 上线，#2/#3/#4 于 2026-11-16 完成），明细移至「已处理」归档区。

> 注：wandao 参考源 `G:\skills\wandao-main` 已不在原位置（2026-11-16 实测），其思路记录仍有效；如需再对照实现须先重新定位源码。

**明确不做**（记此防下次重提）：

| 项 | 原因 |
|---|---|
| 插件中心 / Provider manifest / 任务中心 / checkpoint 断点续跑 / 增量跳过 / 内嵌 Python runtime | 属「批量搬迁」的重型机制，套在单条速记上即过度工程 |

> ⚠️ **AGPL-3.0 红线（长期有效，待办处理完也不删除）**：wandao 为 AGPL-3.0，sensend 为 MIT。直接复制其 Rust 源码会触发协议传染，法律上要求 sensend 整体以 AGPL 开源。**只能借鉴思路、自行重新实现，不得搬运文件。**

---

## 已处理（归档）

### 平台凭证 DPAPI 加密（2026-11-16 修复，出处：wandao 借鉴项 #1）

- **现象**：`config.json` 中 `platform_instances` 的 token/token2 与 `token_memory` 全部**明文存储**（Notion token、飞书 App Secret 可直接读）
- **修复**：新增 `src-tauri/src/security.rs`——Windows DPAPI 用户级加密 + 应用熵，密文 `enc1:` 前缀 + base64 标记
- **兼容**：读取兼容旧明文（无前缀原样返回）；启动时 `migrate_credentials_at_startup` 自动迁移残留明文，迁移前备份 `config.json` → `config.json.bak`
- **边界**：加解密只发生在 platform.rs 存/读 store 处，业务层与前端零改动；仅借鉴 wandao 思路自行实现（规避 AGPL）
- **诚实边界**：DPAPI + 熵可挡「拷文件到别的机器」与「同机程序空熵盲扫」，**挡不住**以同账户运行且专门逆向本应用的恶意程序

### 格式 P1 三连修（2026-08-19 提交 d09a413）

| 项目 | 状态 |
|------|------|
| FlowUs 表格单元格多 rich_text 丢失（`rt[0]` → `row_cells.extend(rt)`） | ✅ 已修复 |
| Notion 代码块语言映射（`NOTION_LANG_MAP`，40 个别名、大小写不敏感、兜底纯文本） | ✅ 已修复 |
| Notion 嵌套列表深度截断（上限 2 层，第 3 层起子项文本并入父项） | ✅ 已修复 |

### wandao 借鉴项 #1–#4 全部完成（2026-11-16）

| # | 项目 | 实现情况 |
|---|------|----------|
| 1 | 应用内自动更新 | v0.5.1 上线：tauri-plugin-updater + minisign 签名校验 + 双端点（useUpdater + commands/updater.rs） |
| 2 | CSP 收紧 | `script-src` 去 `unsafe-eval`；`connect-src` 去全网 `https:`（前端零直连，全走 IPC）；补 `object-src 'none'` / `base-uri` / `form-action` |
| 3 | 配置原子写 | `note.rs` tmp + sync_all + rename（tokio 异步）；另加 inFlightSave 并发守卫防交错写 |
| 4 | 本地文件名边界 | `local.rs::sanitize_title`：控制字符、首尾空白、保留设备名（stem 精确比对，如 con/NUL.backup/com1.md）前缀 `_`，带单元测试 |

> 均只借鉴思路自行实现（规避 AGPL）。

### 复查报告漏网小修批次（2026-11-16 修复，共 12 项）

- WEBVIEW2 附加参数改逐项合并：环境变量部分重叠时 `--js-flags` 等不再丢失（lib.rs）
- 平台「写入方式」显隐改由后端 `PlatformTypeInfo.supported_modes` 下发，删前端 `'local'/'lark'` 写死；切换/载入时 `publish_mode` 归一到支持档位
- 字体管理器预览不再套自定义字体（多 CJK 大字体并发解析会卡开列表；实际字形在编辑器内选择查看）
- `resolve_notion_id` 先剥 `?query`/`#锚点`：块分享链接不再误取锚点当页面 ID（带测试）
- IR 层修复 blockquote 内嵌列表项拼一行：各项以 hardBreak 分隔，四平台共用（带测试）
- Notion 超深有序列表溢出文本按位置编号（不再全部 "1. "）
- markdown 代码块围栏：代码含 ``` 行时加长围栏；闭合围栏前补换行（修复围栏粘在末行代码后）
- `initTheme` 两个 IPC 并行（省一次串行往返；主题仍在挂载前生效防闪色）
- 快捷键默认值收敛到后端 `get_hotkeys` 单一来源（前端不再各存一份）
- 接入 `tauri-plugin-log`：release 落 `%LOCALAPPDATA%\com.jloft.sensend\logs\sensend.log`（512KB 滚动）+ dev stdout；`publish_note` 成功/失败 info/warn 留痕（不含内容与凭证）——结束线上零日志状态
- CSP 收紧见 wandao #2；本地文件名边界见 wandao #4
