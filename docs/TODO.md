# Sensend 待办事项

> 本文件记录已发现但暂未处理的问题与改进点，便于后续统一处理。
> 新发现的问题追加到对应分类末尾即可，处理完成后移至「已处理」归档区。

---

## 待处理

### 表格功能前端入口缺失

- **发现时间**：2026-07-31（v0.3.0 发布后用户反馈）
- **现象**：用户下载 v0.3.0 测试，看不到表格支持
- **根因**：前端没有任何"插入表格"的入口，但 README 宣传支持表格

**现状（半成品）**：

| 层级 | 文件 | 状态 |
|------|------|------|
| 编辑器内核 | `src/composables/useEditor.ts#L132` | ✅ 已注册 TableKit |
| 粘贴富文本 HTML | `src/composables/useEditor.ts` | ✅ 可解析表格 |
| 粘贴纯文本 GFM 表格 | `src/composables/useEditor.ts` | ✅ 已修复（07a7be3：正则补表格分隔行） |
| 表格显示样式 | `src/styles/editor.css` | ✅ 已修复（07a7be3：边框/表头/选中高亮） |
| 格式化操作 | `src/composables/useEditorFormat.ts` | ❌ 无 insertTable |
| 工具栏按钮 | `src/App.vue#L168-L200` | ❌ 无表格按钮 |
| 快捷键 | `src/composables/useHotkey.ts` | ❌ 无表格快捷键 |
| 后端适配器 markdown/notion/flowus/local | `src-tauri/src/adapters/*.rs` | ✅ 转换逻辑齐全 |
| 后端适配器 lark | `src-tauri/src/adapters/lark.rs#L483-L506` | ⚠️ 飞书无原生表格，降级为 `|` 分隔文本 |

**修复方案（最小改动）**：

1. `useEditorFormat.ts` 加 `insertTable` 方法：`editor.chain().focus().insertTable({ rows: 3, cols: 3 }).run()`
2. `App.vue` BubbleMenu 加"插入表格"按钮

**关联矛盾**：`README.md` 第 10、17 行已宣传"支持表格"，粘贴链路已通，仅剩手动创建入口。

---

### 前端空检查正则对带空格的 @实例名失效

- **发现时间**：2026-08-18（第三方复查 ir.rs 尾部剥除逻辑时发现）
- **现象**：实例名含空格（如 `我的 Notion`）时，发送前空检查的正则 `/(?<!\S)@\S+/g` 只剥到第一个空格，残留 ` Notion` 使检查误判为有内容而放行；后端 stripMentions 后全文只剩空白 → 空 IR → 适配器兜底发出一条空内容并提示成功（"成功的空发送"）
- **位置**：`src/composables/usePlatform.ts#L78`
- **修复方向**：空检查改用与 stripMentions 同源的判定（遍历 JSON 剔除 mention 节点后再判空），而非文本正则
- **严重度**：低（需实例名含空格且正文只有该 mention 才触发），暂不修，记此备查

---

## 已处理（归档）

*（暂无）*
