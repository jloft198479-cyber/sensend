# Sensend 开发经验手册

> 作者：简乐
> 最近更新：2026-08-19（文档同步：格式 P1 三连修状态归档、飞书格式规范并入 FEISHU-GUIDE、已知未修复问题并入 TODO、索引去死链）
> 项目：Sensend v0.4.0
> 仓库：[github.com/jloft198479-cyber/sensend](https://github.com/jloft198479-cyber/sensend)

---

## 一、产品与开发

### 1.1 产品原则与设计理念

Sensend 的核心设计理念：

| 原则 | 说明 | 实践 |
|------|------|------|
| 极简 | 功能聚焦，不做多余的事 | 只做"发送"这一件事 |
| 极致 | 每个细节都打磨到位 | 窗口尺寸精确到像素 |
| 轻盈 | 体积小、启动快、占用低 | 主窗口 420×210px |
| 优雅 | 界面美观，交互流畅 | 无边框窗口，自定义标题栏 |
| 极速 | 响应迅速，无等待感 | 全局快捷键秒开 |

**开发约束**：
- 无冗余：不添加非必要功能
- 原子化：每个组件职责单一
- 组件化：可复用、可组合
- 按需加载：不提前加载未使用的资源

**代码风格**：
- 用最少的代码实现功能
- 不将简单问题复杂化
- 讨论方案时不甩代码，需要时再写
- 错误解释用大白话

### 1.2 工作流程：单目录直接开发（v0.3.0 起）

> 历史：v0.1.0 时期因沙箱限制采用"双目录开发模式"（sensend-du → F:\sensend）。
> v0.3.0 起沙箱可直接访问项目目录，双目录模式已弃用，统一在 `F:\fzz-Project\sensend\sensend` 开发。

**当前工作流程**：
1. 在 `F:\fzz-Project\sensend\sensend` 直接修改代码
2. 后端测试：`powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\run-tests.ps1"`
3. 看效果：`scripts\run-app.ps1`（自动加载 Rust/MSVC 环境）
4. 打包：`scripts\build-release.ps1`（唯一正确构建方式，见 BUILD-GUIDE §〇）
5. 提交并推送到 GitHub，打 tag，创建 Release

### 1.3 需求理解：修改前先确认

**踩坑案例**：

用户说"窗口高度不够"，我直接修改了主窗口高度，但用户实际指的是配置窗口。

**教训**：
- 用户提到"窗口"时，要确认是哪个窗口（主窗口/配置窗口/其他）
- 用户提到"高度/宽度"时，要确认具体数值和目标
- 修改前先复述需求，确认理解正确

---

## 二、平台适配器开发

### 2.1 三大平台 API 差异对比

| 特性 | Notion | FlowUs | 飞书 |
|------|--------|--------|------|
| Block 类型 | 字符串 | 字符串 | 数字枚举 |
| 认证方式 | Bearer Token | Bearer Token | tenant_access_token |
| Token 格式 | secret_xxx | xxx | app_id:app_secret |
| 文本格式 | rich_text | rich_text | text_run |
| API 风格 | RESTful | RESTful | RESTful |
| 数据库/多维表 | 支持 | 支持 | 不适合长文本 |
| 追加文档 | 支持 | 支持 | 支持 |
| 文件夹操作 | 支持 | 支持 | 仅应用创建的 |

**Block 类型对照表**：

| 类型 | Notion | FlowUs | 飞书 |
|------|--------|--------|------|
| 段落 | paragraph | paragraph | 2 |
| 标题1 | heading_1 | heading_1 | 3 |
| 标题2 | heading_2 | heading_2 | 4 |
| 标题3 | heading_3 | heading_3 | 5 |
| 无序列表 | bulleted_list_item | bulleted_list_item | 12 |
| 有序列表 | numbered_list_item | numbered_list_item | 13 |
| 引用 | quote | quote | 15 |
| 代码 | code | code | 14 |
| 待办 | to_do | to_do | 17 |
| 分割线 | divider | divider | 22 |

> **注意**：飞书的 block_type 数字枚举容易记混（引用=15、代码=14，不是 16/17），代码中用常量定义在 `lark.rs` L14-L21。

**飞书认证流程**：
```rust
// 1. 获取 tenant_access_token
let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
let body = json!({
    "app_id": app_id,
    "app_secret": app_secret
});

// 2. 使用 token 调用 API
let headers = vec![
    ("Authorization", format!("Bearer {}", token)),
    ("Content-Type", "application/json".to_string()),
];
```

### 2.2 飞书适配器重构经验

**初始问题**：
- 代码臃肿，884 行
- 支持多维表、文件夹，但实际不适合
- 多维表只能写入标题，无法存储长文本

**重构决策**：

| 功能 | 决策 | 原因 |
|------|------|------|
| 文档追加 | ✅ 保留 | 核心功能 |
| 文件夹 | ❌ 移除 | tenant_access_token 只能访问应用创建的文件夹 |
| 多维表 | ❌ 移除 | 仅写入标题，不适合长文本存储 |

**前端配套改动**：
- 凭证输入拆分为 App ID、App Secret 两个输入框
- 提示文案改为"粘贴飞书文档链接，内容将追加到文档末尾"

### 2.3 重构时如何保留核心逻辑

**踩坑案例**：

重构飞书适配器时，我重写了 `marks_to_text_elements` 函数，导致中文无法正确发送。

**原因**：原始函数经过多次调试验证，包含处理特殊字符的逻辑，我随意修改破坏了它。

**教训**：
- 重构前先标记要保留的核心函数
- 核心逻辑不要重写，只做必要的结构调整
- 重构后必须测试所有功能点

---

## 三、重难点问题（新 Agent 必读）

> 以下是项目中最容易踩坑、最关键的设计约束，按优先级排列。理解这些可以避免 80% 的返工。

### ⚠️ 1. IR 是唯一出站关卡——改一处影响四平台

**位置**：`src-tauri/src/adapters/ir.rs`

所有平台适配器共享同一个 IR 中间层。`ir::parse(content)` 将 TipTap JSON 解析为 `Vec<Block>`，只遍历一次，四个适配器（markdown/notion/flowus/lark）各自把 IR 渲染成平台格式。

**含义**：
- 在 IR 层修一个 bug，一次修复四个平台同时生效
- 在 IR 层引入一个 bug，四个平台同时炸
- 改 IR 后必须跑完整黄金测试：`powershell -File scripts/run-tests.ps1`

**已有防御**：IR 尾部空段剥除逻辑（`parse()` 末尾 while 循环），有 4 条边界测试保护，不要随意改动。

### ⚠️ 2. 构建环境——普通终端跑 `npm run tauri build` 必挂

**根因**：Rust（`M:\rust`）和 VS Build Tools（`M:\VS\BuildTools`）都不在系统 PATH。

**唯一正确方式**：
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
```

详见 `docs/BUILD-GUIDE.md` §〇（构建环境速查，已并入原 BUILD-ENV.md）。**报错 ≠ 环境坏**，环境本身完好，只是没加载。

### ⚠️ 3. 飞书"能读不能写"——权限三扇门

飞书的权限模型是最容易卡住人的地方：

| 门 | 在哪设置 | 最容易踩的坑 |
|---|---|---|
| ① API 权限 Scope | 开放平台 → 权限管理 | 加了权限但**没发布版本**，线上不生效 |
| ② 版本发布 | 开放平台 → 版本管理与发布 | 改完权限忘了发版 = 白改 |
| ③ 资源协作 | 飞书客户端，把应用加进文档协作者 | 有权限但没加协作者，连看都看不了 |

**关键**：Sensend 的「测试连接」按钮**只测读取，不测写入**。测试通过 ≠ 能写入。详见 `FEISHU-GUIDE.md`。

### ⚠️ 4. 飞书 wiki URL 需要两步解析

飞书 wiki 链接（含 `/wiki/`）不能直接当 document_id 用，需要先调 `/wiki/v2/spaces/get_node` 把 `node_token` 换成 `obj_token`（真正的 document_id）。代码在 `lark.rs` L220-L258。

### ⚠️ 5. 飞书仅追加模式——不支持创建新文档

飞书适配器是唯一不支持 `publish`（创建新页面）的适配器，只支持 `append_blocks`（追加到已有文档）。前端表单中飞书的写入模式选择被隐藏。

### ⚠️ 6. 表格功能是半成品——前端无入口

后端四平台适配器都有表格转换逻辑，编辑器也注册了 TableKit，粘贴表格（HTML/GFM）能正常解析和显示。但**前端没有"插入表格"按钮**，用户无法手动创建表格。详见 `TODO.md`。

### ⚠️ 7. Notion resolve_target 三步试探法

Notion 的目标可以是 Database 也可以是 Page，API 没有统一查询接口。代码用三步试探：
1. 直接查 Database → 成功则按 Database 处理
2. 查子块 child_database → 成功则按 Database 处理
3. 兜底按 Page 处理

**v0.4.0 修复**：探测失败（超时/权限）时不再静默降级为 Page，而是阻止发送并报错，防止弱网下多维表内容误写为独立文章。

### ⚠️ 8. mention 唯一性约束

文档中同时只允许一个 mention 节点。插入新 mention 前先删除所有旧 mention 并修正 range 偏移。底栏选择目标与编辑区 mention 双向同步。

---

## 四、已知未修复问题

> 已并入 `docs/TODO.md`（平台发送健壮性）+ 各格式调研文档，不再在本手册重复维护，避免多份拷贝失同步。
> 涉及：Notion 429 限流无重试、飞书 429 限流无重试 / 引用无容器 / text_run 无上限 / 嵌套列表拍平 / 代码块 wrap、前端表格插入入口缺失、@实例名空格正则缺陷。

---

## 五、项目路径速查

| 路径 | 用途 |
|------|------|
| `F:\fzz-Project\sensend\sensend` | 主项目目录（v0.3.0 起） |
| `M:\rust\.cargo` | Rust CARGO_HOME（自定义位置）|
| `M:\rust\.rustup` | Rust RUSTUP_HOME（自定义位置）|
| `M:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat` | MSVC 环境加载脚本 |
| `%LOCALAPPDATA%\tauri\NSIS\` | Tauri NSIS 缓存 |
| `gh` CLI | GitHub Release 上传工具（已登录 jloft198479-cyber）|

> 历史：v0.1.0 时期主目录为 `F:\sensend`，沙箱目录为 `C:\Users\fzz198479\sensend-du`，已弃用。

---

## 六、文档索引

| 文档 | 定位 | 何时读 |
|------|------|--------|
| `CODE-WIKI.md` | 代码百科，系统全貌 | **第一个读**——了解架构、模块、数据流 |
| `BUILD-GUIDE.md` | 构建环境速查 + 打包发布手册（已并入原 BUILD-ENV） | 动手构建/发布前读 |
| `FORMAT-FIXES.md` | 格式兼容性排查 | 遇到编辑器格式问题时读 |
| `FEISHU-GUIDE.md` | 飞书对接指南 + 格式规范（已并入原 FEISHU-FORMAT-SPEC） | 配置飞书或排查飞书问题时读 |
| `NOTION-FORMAT-SPEC.md` | Notion 格式调研 | 改 Notion 适配器时读 |
| `SEND-EVALUATION.md` | 发送质量评估报告 | 想了解格式保真度/稳定性全貌时读 |
| `UX-PERFORMANCE-EVALUATION.md` | 体验与性能评估报告 | 想了解 UI/性能现状时读 |
| `TODO.md` | 待办事项（含已知未修复问题队列） | 规划下一步工作时读 |
| `YOUDAO-MCP-FEASIBILITY.md` | 有道云笔记接入调研 | 需要接入有道云时读 |
| 本文档 | 开发经验与重难点 | **第二个读**——掌握踩坑经验和关键约束 |

---

> 本手册基于 Sensend 开发经验整理，最近一次更新对齐 v0.4.0（2026-08-18）
> 记录人：简乐
> 致谢：送给儿子小柏
