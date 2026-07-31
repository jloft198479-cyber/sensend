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
| 粘贴富文本 HTML | `src/composables/useEditor.ts#L261-L262` | ✅ 可解析表格 |
| 粘贴纯文本 GFM 表格 | `src/composables/useEditor.ts#L272` | ❌ 正则不含 `\|`，不识别 |
| 格式化操作 | `src/composables/useEditorFormat.ts` | ❌ 无 insertTable |
| 工具栏按钮 | `src/App.vue#L168-L200` | ❌ 无表格按钮 |
| 快捷键 | `src/composables/useHotkey.ts` | ❌ 无表格快捷键 |
| 后端适配器 markdown/notion/flowus/local | `src-tauri/src/adapters/*.rs` | ✅ 转换逻辑齐全 |
| 后端适配器 lark | `src-tauri/src/adapters/lark.rs#L483-L506` | ⚠️ 飞书无原生表格，降级为 `|` 分隔文本 |

**修复方案（最小改动）**：

1. `useEditorFormat.ts` 加 `insertTable` 方法：`editor.chain().focus().insertTable({ rows: 3, cols: 3 }).run()`
2. `App.vue` BubbleMenu 加"插入表格"按钮
3. `useEditor.ts#L272` 的 `looksLikeMarkdown` 正则补上 `\|` 表格模式，让纯文本 GFM 表格也能粘贴识别

**关联矛盾**：`README.md` 第 10、17 行已宣传"支持表格"，但实际前端无法触发，需同步修复或在 README 标注限制。

---

## 已处理（归档）

*（暂无）*
