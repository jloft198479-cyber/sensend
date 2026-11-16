# Sensend 编辑器格式兼容性排查手册

> 作者：简乐 ｜ 最近更新：2026-08-18
> 项目：Sensend v0.4.0
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
- **修复位置**：[src/App.vue](file:///f:/fzz-Project/2-sensend/sensend/src/App.vue) 的待办 CSS，选择器全部从 `li[data-type="taskItem"]` 改为 `li[data-checked]`。
- **排查要点**：写 TipTap 扩展的 CSS 时，**必须实测渲染后的 DOM 属性**，不要只看扩展源码的 renderHTML——NodeView 渲染路径和 renderHTML 序列化路径可以完全不同。

### 1.2 待办：从网页复制带 checkbox 的待办，勾选框全丢

- **现象**：从网页/在线文档复制待办列表粘进 sensend，checkbox 消失，退化成普通圆点列表。
- **根因**：原生 TaskItem 的 parseHTML 只认 `li[data-type="taskItem"]`，网页的 `li > input[type=checkbox]` 结构不认识，被当成普通 listItem。同时还会留下一个空的 bulletList 壳。
- **修复位置**（[src/composables/useEditor.ts](file:///f:/fzz-Project/2-sensend/sensend/src/composables/useEditor.ts)）：
  - `TaskItemBase.extend({...})` 补两条 `:has()` 解析规则（`li:has(> input[type=checkbox])`、`li:has(> label input[type=checkbox])`，priority 52 高于 listItem 的 50），并把 checkbox 勾选态读进 `attrs.checked`。
  - `handlePaste` 里：凡 HTML 含 checkbox，先用 DOMParser 把 `<ul>` 标成 `data-type="taskList"`、`<li>` 标成 `data-type="taskItem"`+`data-checked`、拆掉 label/删除 input，再 insertContent——彻底消灭空壳。
- **排查要点**：网页待办有裸 input 和 label 包 input 两种结构，两条规则都要有；普通列表项（无 checkbox）不能被误判成 taskItem。

### 1.3 表格：markdown 表格（`| 列 | 列 |`）粘贴后变成普通文字

- **现象**：从 Typora / GitHub / VSCode 复制的竖线表格文本，粘进 sensend 变成一行普通文字，不是表格。
- **根因**：`handlePaste` 入口的"是否像 markdown"粗判正则不含表格语法，`| --- | --- |` 分隔行不被识别，整串文本没走 markdown 解析。
- **修复位置**：[src/composables/useEditor.ts](file:///f:/fzz-Project/2-sensend/sensend/src/composables/useEditor.ts) 的 `looksLikeMarkdown` 正则，新增表格分隔行分支：`\s*\|?[\s:|-]*-{2,}[\s:|-]*\|`。
- **排查要点**：正则只在"分隔行"（含连续两个以上 `-`）上放行，普通句子里的单竖线不会误命中。`@tiptap/extension-table` 自带的 parseMarkdown 能力是够的，缺的只是入口关卡。

### 1.4 表格：粘进来是无线的字，看不出是表格

- **现象**：表格能解析出来，但完全没有边框和表头底色，显示成一堆字。
- **根因**：全项目没有一行表格 CSS。
- **修复位置**：[src/styles/editor.css](file:///f:/fzz-Project/2-sensend/sensend/src/styles/editor.css) 新增表格样式：`border-collapse: collapse`、td/th 边框 `var(--gray-border)`、表头浅底加粗、单元格内边距、`.selectedCell` 高亮。
- **排查要点**：TipTap 功能是否"看着有"要分三层验证——能否解析（parse）、能否渲染（DOM+CSS）、能否导出（IR+适配器）。少任何一层都是半废。

### 1.5 出站：粘贴/光标落脚导致的尾部空段

- **现象**：粘贴后导出到平台会多一个空块（空段落）。
- **根因**：编辑器粘贴后会给文档尾部补一个空 paragraph 作为光标落脚点，它不是用户内容，却被 getJSON 原样带走。
- **修复位置**：[src-tauri/src/adapters/ir.rs](file:///f:/fzz-Project/2-sensend/sensend/src-tauri/src/adapters/ir.rs) 的 `parse()`，尾部 while 剥掉空段（空或纯空白文本的 Paragraph）。
- **三条边界（都有测试）**：只剥**尾部**；**中间**空行是用户排版保留；尾部含 @mention 的段是有效内容不误杀。
- **测试**：`ir_parse_trailing_blank_paragraphs_trimmed` / `ir_parse_middle_blank_paragraph_kept` / `ir_parse_trailing_mention_paragraph_kept`。

### 1.6 构建：os error 5（拒绝访问）导致构建失败

- **现象**：`npm run tauri build` 报 `error: failed to remove file ...sensend.exe` / `拒绝访问 (os error 5)` / `RUST BUILD FAILED`。
- **根因**：有 sensend.exe 进程还开着（上次启动的应用窗口没关），Windows 不让删除/覆盖正在运行的文件。
- **修复位置**：[scripts/run-app.ps1](file:///f:/fzz-Project/2-sensend/sensend/scripts/run-app.ps1) 构建前会杀掉**路径等于 `src-tauri\target\release\sensend.exe`** 那个进程（按 Path 精确匹配，**不会碰你机器上安装的正式版**）。
- **排查要点**：报 os error 5 时先查 `Get-Process sensend`，确认是哪个路径的进程占用；双击 run-app.bat 会自动处理它自己那一个。安装版的进程要你自己从托盘退出。

### 1.7 出站：FlowUs 表格单元格多 rich_text 丢失（P1，d09a413；2026-11-16 修正）

- **现象**：FlowUs 表格单元格内文本有混合样式（如"**粗体**+普通+*斜体*"）时，只保留了第一段，后续格式或内容丢失。
- **根因（修正后）**：`flowus.rs` 表格单元格映射用 `rt[0].clone()`，把整个 rich_text 数组压成了一个元素。
- **d09a413 的修法不彻底**：当时改成 `row_cells.extend(rt)`，把格内每一段都摊成 cells 里的独立一项。但 FlowUs 的 `cells` 是**二维数组**——外层每项是一列，每列本身才是 rich_text 数组，列数必须等于 `table_width`（见 FlowUs 操作手册「表格」一节，结构与 `notion.rs` 同构）。摊平后「一格两段」会占两个列位，整行右移，列数与 `table_width` 也对不上。
- **当前修法**：`row_cells.push(json!(rt))`——一格一个数组，格内保留全部样式段（不丢格式、不打乱列数）；补列时用 `empty_cell()` 返回同样的二维元素。
- **测试**：`fix_p1_table_cell_keeps_all_rich_text_segments`（断言 1 列表格 `cells.len()==1`、该格内 2 段）、`table_col_count_matches_columns_not_segment_count`（2 列 × 2 段，列数恒为 2）、`fix5_table_cell_keeps_annotations`（取 `cells[0][0]`）；golden `table_with_inline` 已重生成。

### 1.8 出站：Notion 代码块语言映射缺失（P1，d09a413）

- **现象**：代码块语言用 TipTap 原始值直传（如 `cpp`、`plaintext`），Notion 只认官方枚举（如 `c++`、`plain text`），部分语言不被识别。
- **根因**：`notion.rs` 直接把 `language` 属性透传，无映射表。
- **修复位置**：[notion.rs](file:///f:/fzz-Project/2-sensend/sensend/src-tauri/src/adapters/notion.rs) 新增 `NOTION_LANG_MAP` 常量表，覆盖 40 个别名、大小写不敏感，未知语言回落 `plain text`。
- **测试**：`fix_p1_language_mapping`（notion.rs）。

### 1.9 出站：Notion 嵌套列表深度超限（P1，d09a413）

- **现象**：三层及以上嵌套列表触发 Notion `validation_error`（单请求 children 嵌套上限 2 层）。
- **根因**：`map_list_item` 递归构建 children 无深度截断。
- **修复位置**：[notion.rs](file:///f:/fzz-Project/2-sensend/sensend/src-tauri/src/adapters/notion.rs) 给 `map_list_item` 加 `depth` 参数（从 1 计），`depth < 2` 才允许继续嵌套，第 3 层起子项文本并入父项 rich_text（带层级前缀），内容不丢。
- **测试**：`fix_p1_list_depth_capped_at_two`（notion.rs）。

### 1.10 出站：FlowUs 表格裸 table_row 直发被拒（P1，2026-11-16 真机验证）

- **现象**：发往 FlowUs 的内容带表格时整个发送 400 失败（此前只做过本地快照测试，未真机验证）。
- **根因**：`flowus.rs` 把 `table_row` 块当普通块直发到页面 children，但 FlowUs v1 API **不接受裸 table_row**——必须先建 `table` 壳，再把行挂到壳下（同 Notion 官方结构）。
- **修复位置**：[flowus.rs](file:///f:/fzz-Project/2-sensend/sensend/src-tauri/src/adapters/flowus.rs) Table 分支输出 `table` 块（行随顶层 `children` 携带）；`append_children` 遇 table 拆两步：`flush_batch`（普通块攒批）→ `append_table`（建壳 → 取返回 id → 行按 100/批挂壳下）。
- **验证边界（如实记录）**：当时参考脚本发的还是裸 `table_row`、且 cells 用的是二维结构，与本仓库当时的代码路径不一致，**不能作为"多段单元格落库"的证据**。表格结构本身以 FlowUs 操作手册（`cells` 二维、列数 = `table_width`）为准，当前代码已按该结构对齐并有单测锁定；真机多段单元格落库仍待复验。
- **测试**：`fix5_table_cell_keeps_annotations` / `fix_p1_table_cell_keeps_all_rich_text_segments` 改为断言 table 壳结构；golden `table_with_inline` 重生成。

### 1.11 出站：FlowUs 待办块类型名错误，静默降级成段落（P2，2026-11-16 真机验证）

- **现象**：待办发到 FlowUs 不报错，但落库变成普通 paragraph，勾选框消失——静默降级最迷惑。
- **根因**：`flowus.rs` 类型名用了 `todo`，FlowUs v1 对未知类型不报错而是降级 paragraph；实测**正确类型名是 `to_do`**（v2 API 命名）。
- **修复位置**：[flowus.rs](file:///f:/fzz-Project/2-sensend/sensend/src-tauri/src/adapters/flowus.rs) `map_list_item` 的 `ListKind::Task => "to_do"`。
- **真机验证**：`to_do` + `checked:true` 完整落库。
- **测试**：golden `tasklist` 重生成。

### 1.12 出站：FlowUs 嵌套列表整段内容丢失（P1，2026-11-16 真机验证）

- **现象**：嵌套列表发到 FlowUs 不报错，但**子项内容整段消失**（比丢层级更严重的内容丢失）。
- **根因**：`flowus.rs` 把子列表塞进 `data.children`，FlowUs v1 对该字段**静默丢弃**（响应 200，不报错）。原代码注释标注的"真机验证点"由此关闭。
- **修复位置**：[flowus.rs](file:///f:/fzz-Project/2-sensend/sensend/src-tauri/src/adapters/flowus.rs) `map_list_item` 改为拍平：子项递归输出为同级块（父项在前、子项按原顺序紧随），不再携带任何 `children` 字段。
- **真机验证**：父项+两级子项全部落库为同级 bulleted_list_item，文本无丢失。
- **测试**：`fix2` 从断言"进 children"改写为 `fix2_nested_list_flattened_no_content_loss`（断言拍平顺序+无 children 字段+内容齐全）；golden `nested_list` 重生成。

> **1.10~1.12 的共同教训**：FlowUs v1 有两类坑——**硬拒绝**（裸 table_row → 400）和**静默降级/丢弃**（todo→paragraph、children→丢弃）。后者不报错、只丢数据，本地快照测试完全测不出来，必须真机读回验证。真机证据与复跑脚本：`F:\00-项目\05-flowus\.tmp\verify_sensend_v1.py`、调研文档 `F:\00-项目\05-flowus\Sensend×FlowUs博客-兼容性调研.md`。

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
4. **改完必测三样**：`npm run build`（类型+打包）、`powershell -File scripts/run-tests.ps1`（30 个后端黄金测试）、浏览器实测（渲染+粘贴）。任何一边红都不算修完。

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
