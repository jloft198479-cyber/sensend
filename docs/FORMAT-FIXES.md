# Sensend 编辑器格式兼容性排查手册

> 作者：简乐 ｜ 最近更新：2026-08-17
> 项目：Sensend（v0.3.0 → 0.4.0 升级期）
> 定位：专门记录"编辑器格式进站/出站"类问题的现象、根因、修复与排查套路，方便后续快速定位同类 bug。

---

## 0. 先说清楚 sensend 的格式管线（理解所有问题的基础）

任何内容的流转都走同一条流水线：

```
外部内容（markdown 文本 / 网页 HTML）
   │  ← 进站：handlePaste 入口
   ▼
TipTap 编辑器（所见即所得）
   │  ← 存储：getJSON() → note.json
   ▼
Rust 端 IR 中间表示（src-tauri/src/adapters/ir.rs，唯一遍历点）
   │  ← 出站：parse() 之后
   ▼
四适配器（markdown / notion / flowus / lark）
   │
   ▼
目标平台
```

**两个铁律（本手册所有问题都源于这两条被违反）**：
1. **进站坏了，出站必坏**——TipTap 文档里结构不对，四平台导出跟着错，因为出站只认 IR。
2. **IR 是唯一出站关卡**——在 IR 里做的修正，一次生效四个平台；在前端做的修正，只影响编辑器显示。

---

## 1. 已修复问题清单

### 1.1 待办：勾选框和文字分成两行

- **现象**：粘贴 markdown 待办（`- [ ] xxx`）后，勾选框在上一行、文字在下一行。
- **根因**（库坑，非我们代码错误）：`@tiptap/extension-list` 3.28 的 TaskItem 注册了自定义 `addNodeView`（node_modules/@tiptap/extension-list/dist/index.js L1360-1451），live DOM 里的 `<li>` 是手工 `createElement` 拼的，**只设了 `data-checked` 属性，不设 `data-type="taskItem"`**。我们原来的 CSS 选择器全用 `li[data-type="taskItem"]`，在真实 DOM 上一个都匹配不上，flex 横排布局失效，块级 div 和行内 label 自然各占一行。
- **修复位置**：[src/App.vue](file:///f:/fzz-Project/sensend/sensend/src/App.vue) 的待办 CSS，选择器全部从 `li[data-type="taskItem"]` 改为 `li[data-checked]`。
- **排查要点**：写 TipTap 扩展的 CSS 时，**必须实测渲染后的 DOM 属性**，不要只看扩展源码的 renderHTML——NodeView 渲染路径和 renderHTML 序列化路径可以完全不同。

### 1.2 待办：从网页复制带 checkbox 的待办，勾选框全丢

- **现象**：从网页/在线文档复制待办列表粘进 sensend，checkbox 消失，退化成普通圆点列表。
- **根因**：原生 TaskItem 的 parseHTML 只认 `li[data-type="taskItem"]`，网页的 `li > input[type=checkbox]` 结构不认识，被当成普通 listItem。同时还会留下一个空的 bulletList 壳。
- **修复位置**（[src/composables/useEditor.ts](file:///f:/fzz-Project/sensend/sensend/src/composables/useEditor.ts)）：
  - `TaskItemBase.extend({...})` 补两条 `:has()` 解析规则（`li:has(> input[type=checkbox])`、`li:has(> label input[type=checkbox])`，priority 52 高于 listItem 的 50），并把 checkbox 勾选态读进 `attrs.checked`。
  - `handlePaste` 里：凡 HTML 含 checkbox，先用 DOMParser 把 `<ul>` 标成 `data-type="taskList"`、`<li>` 标成 `data-type="taskItem"`+`data-checked`、拆掉 label/删除 input，再 insertContent——彻底消灭空壳。
- **排查要点**：网页待办有裸 input 和 label 包 input 两种结构，两条规则都要有；普通列表项（无 checkbox）不能被误判成 taskItem。

### 1.3 表格：markdown 表格（`| 列 | 列 |`）粘贴后变成普通文字

- **现象**：从 Typora / GitHub / VSCode 复制的竖线表格文本，粘进 sensend 变成一行普通文字，不是表格。
- **根因**：`handlePaste` 入口的"是否像 markdown"粗判正则不含表格语法，`| --- | --- |` 分隔行不被识别，整串文本没走 markdown 解析。
- **修复位置**：[src/composables/useEditor.ts](file:///f:/fzz-Project/sensend/sensend/src/composables/useEditor.ts) 的 `looksLikeMarkdown` 正则，新增表格分隔行分支：`\s*\|?[\s:|-]*-{2,}[\s:|-]*\|`。
- **排查要点**：正则只在"分隔行"（含连续两个以上 `-`）上放行，普通句子里的单竖线不会误命中。`@tiptap/extension-table` 自带的 parseMarkdown 能力是够的，缺的只是入口关卡。

### 1.4 表格：粘进来是无线的字，看不出是表格

- **现象**：表格能解析出来，但完全没有边框和表头底色，显示成一堆字。
- **根因**：全项目没有一行表格 CSS。
- **修复位置**：[src/styles/editor.css](file:///f:/fzz-Project/sensend/sensend/src/styles/editor.css) 新增表格样式：`border-collapse: collapse`、td/th 边框 `var(--gray-border)`、表头浅底加粗、单元格内边距、`.selectedCell` 高亮。
- **排查要点**：TipTap 功能是否"看着有"要分三层验证——能否解析（parse）、能否渲染（DOM+CSS）、能否导出（IR+适配器）。少任何一层都是半废。

### 1.5 出站：粘贴/光标落脚导致的尾部空段

- **现象**：粘贴后导出到平台会多一个空块（空段落）。
- **根因**：编辑器粘贴后会给文档尾部补一个空 paragraph 作为光标落脚点，它不是用户内容，却被 getJSON 原样带走。
- **修复位置**：[src-tauri/src/adapters/ir.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/ir.rs) 的 `parse()`，尾部 while 剥掉空段（空或纯空白文本的 Paragraph）。
- **三条边界（都有测试）**：只剥**尾部**；**中间**空行是用户排版保留；尾部含 @mention 的段是有效内容不误杀。
- **测试**：`ir_parse_trailing_blank_paragraphs_trimmed` / `ir_parse_middle_blank_paragraph_kept` / `ir_parse_trailing_mention_paragraph_kept`。

### 1.6 构建：os error 5（拒绝访问）导致构建失败

- **现象**：`npm run tauri build` 报 `error: failed to remove file ...sensend.exe` / `拒绝访问 (os error 5)` / `RUST BUILD FAILED`。
- **根因**：有 sensend.exe 进程还开着（上次启动的应用窗口没关），Windows 不让删除/覆盖正在运行的文件。
- **修复位置**：[scripts/run-app.ps1](file:///f:/fzz-Project/sensend/sensend/scripts/run-app.ps1) 构建前自动 `Get-Process -Name "sensend"` 并 `Stop-Process -Force`。
- **排查要点**：报 os error 5 时先查 `Get-Process sensend`，有进程就杀掉再构建；一键脚本已内置，双击 run-app.bat 不会再踩。

---

## 2. 排查套路（遇到新的格式问题怎么查）

1. **定位是哪一环坏了**：先分清"进站"还是"出站"。
   - 编辑器里显示就不对 → 进站/渲染问题，查 handlePaste、扩展 parseHTML、CSS。
   - 编辑器里对、导出不对 → 出站问题，查 ir.rs 与对应适配器。
2. **验证引擎能力 vs 自家关卡**：很多坑是"库能力在，我们入口/样式没接上"（如 1.3、1.4）。先用临时挂载点实测库能力（见下），再查自家代码。
3. **临时挂载编辑器到 window**（浏览器实测用）：
   ```ts
   onMounted(() => { (window as any).__sensendEditor = editor.value })
   ```
   然后 `npm run dev`，在浏览器 console 里调 `__sensendEditor.storage.markdown.manager.parse(...)`、`getJSON()`、`view.dom.querySelector(...)` 实测。测完删掉。
4. **改完必测三样**：`npm run build`（类型+打包）、`run-tests.bat`（62+ 后端黄金测试）、浏览器实测（渲染+粘贴）。任何一边红都不算修完。

---

## 3. 相关测试速查

| 测试 | 保护的内容 |
|------|-----------|
| `adapter::*::tasklist`（×4 平台） | 待办正常导出 |
| `ir_parse_tasklist_checked` | 待办勾选态解析 |
| `ir_parse_trailing_blank_paragraphs_trimmed` | 尾部空段剥除 |
| `ir_parse_middle_blank_paragraph_kept` | 中间空行保留 |
| `ir_parse_trailing_mention_paragraph_kept` | 尾部 mention 不误杀 |
| `adapter::*::table_with_inline`（×4 平台） | 表格导出 |

前端格式兼容性（粘贴识别、CSS）目前靠浏览器实测，未纳入自动化——如后续要固化，可考虑加 E2E。
