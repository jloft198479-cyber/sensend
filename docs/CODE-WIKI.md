# Sensend Code Wiki

> 超轻量级桌面悬浮记事本 — 代码百科文档
> 版本：v0.4.0 ｜ 仓库：[github.com/jloft198479-cyber/sensend](https://github.com/jloft198479-cyber/sensend)
> 最近更新：2026-08-18（对齐 v0.4.0 发布状态）

---

## 目录

- [1. 项目概览](#1-项目概览)
- [2. 整体架构](#2-整体架构)
- [3. 目录结构](#3-目录结构)
- [4. 前端模块详解](#4-前端模块详解)
- [5. 后端模块详解](#5-后端模块详解)
- [6. 关键类与函数说明](#6-关键类与函数说明)
- [7. 数据流与核心交互](#7-数据流与核心交互)
- [8. 依赖关系](#8-依赖关系)
- [9. 配置与运行方式](#9-配置与运行方式)
- [10. 数据存储](#10-数据存储)
- [11. 平台适配器扩展指南](#11-平台适配器扩展指南)

---

## 1. 项目概览

### 1.1 简介

**Sensend** 是一款超轻量级的桌面悬浮记事本，定位为「灵感速记 + 一键分发」工具。用户通过全局快捷键唤醒悬浮窗口，使用富文本编辑器记录内容，再通过 `@提及` 选择目标平台，一键发送到 Notion / FlowUs / 飞书 / 本地文件夹。

### 1.2 核心特性

| 特性 | 说明 |
|------|------|
| 极速启动 | 悬浮窗口 + 全局快捷键唤醒，随时记录 |
| 富文本编辑 | 基于 TipTap，支持标题、列表、引用、代码块、行内样式、待办清单、下划线 |
| 多平台分发 | Notion / FlowUs / 飞书 / 本地文件夹，统一适配器接口 |
| 自定义字体 | 用户可将 `.ttf/.otf/.woff2` 字体文件放入字体目录加载 |
| 极简设计 | 无边框窗口、自定义标题栏、托盘常驻 |
| 自动保存 | 编辑内容防抖 800ms 自动落盘到 `note.json`（v0.4.0 起从 note.md 迁移） |
| 默认目标记忆 | 上次发送/选择的平台实例 ID 存后端 `config.json`（v0.3.0 起，不再用 localStorage） |
| 暗夜模式 | 浅色/暗夜双主题，两窗口实时同步（v0.4.0 新增） |
| 发送成功标记 | 发送后编辑区末尾显示"✓ 已发送"徽章，3 秒自动消失（v0.4.0 新增） |

### 1.3 技术栈

| 层 | 技术 |
|----|------|
| 前端 | Vue 3.5 + TypeScript 5.6 + Vite 6 |
| 富文本 | TipTap 3（StarterKit + Mention + Placeholder + TableKit + Markdown 扩展）|
| 弹层 | tippy.js（Mention 下拉）|
| 桌面框架 | Tauri 2 |
| 后端 | Rust 2021 edition |
| HTTP | reqwest 0.12（rustls-tls）|
| 异步运行时 | tokio（full features）|
| 持久化 | tauri-plugin-store（JSON 文件）|

### 1.4 平台支持矩阵

| 平台 | 创建子页面 | 追加内容 | 说明 |
|------|-----------|---------|------|
| Notion | ✅ | ✅ | 支持 Database 与 Page，自动识别 |
| FlowUs | ✅ | ✅ | 支持多维表与页面 |
| 飞书 (Lark) | ❌ | ✅ | 仅追加到已有文档，需 App ID + App Secret |
| 本地 (Local) | ✅ | ❌ | 生成 `标题_时间戳.md` 文件（带 UTF-8 BOM）|

---

## 2. 整体架构

### 2.1 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│                      Tauri 应用进程                          │
│                                                             │
│  ┌───────────────────────┐      ┌────────────────────────┐  │
│  │   前端 (WebView)      │      │   后端 (Rust)          │  │
│  │   Vue 3 + TipTap      │      │   tauri::Builder       │  │
│  │                       │ IPC  │                        │  │
│  │  ┌─────────────────┐  │◀────▶│  ┌──────────────────┐  │  │
│  │  │  main 窗口     │  │invoke│  │  commands/       │  │  │
│  │  │  (编辑器)       │  │      │  │  note/platform/  │  │  │
│  │  └─────────────────┘  │emit  │  │  hotkey/font     │  │  │
│  │  ┌─────────────────┐  │◀─────│  └────────┬─────────┘  │  │
│  │  │  config 窗口   │  │      │           │             │  │
│  │  │  (平台管理)     │  │      │  ┌────────▼─────────┐   │  │
│  │  └─────────────────┘  │      │  │  adapters/       │   │  │
│  └───────────────────────┘      │  │ notion/flowus/   │   │  │
│                                 │  │ lark/local       │   │  │
│                                 │  └────────┬─────────┘   │  │
│                                 │           │             │  │
│  ┌─────────────────────────┐    │  ┌────────▼─────────┐   │  │
│  │ 插件层                  │    │  │ PlatformAdapter   │   │  │
│  │ store / dialog /        │    │  │ (统一 trait)      │   │  │
│  │ global-shortcut /       │    │  └────────┬─────────┘   │  │
│  │ single-instance / opener│    │           │             │  │
│  └─────────────────────────┘    └───────────┼─────────────┘  │
│                                              │               │
└──────────────────────────────────────────────┼───────────────┘
                                               │
                          ┌────────────────────┼────────────────────┐
                          ▼                    ▼                    ▼
                   Notion API           FlowUs API           飞书 OpenAPI
                   (REST)               (REST)               (REST)
                                                 本地文件系统 (fs)
```

### 2.2 前后端通信机制

Sensend 使用 Tauri 2 的标准 IPC 通信模型：

- **前端 → 后端**：通过 `@tauri-apps/api/core` 的 `invoke('command_name', { args })` 调用后端 `#[tauri::command]` 标注的函数。
- **后端 → 前端**：通过 `app.emit('event_name', payload)` 推送事件，前端用 `getCurrentWindow().listen('event_name', cb)` 监听。
- **窗口间通信**：配置窗口保存实例后，后端 `emit("instances-updated", ())` 广播，主窗口监听后刷新实例列表。

### 2.3 双窗口模型

项目通过 URL 参数 `?page=config` 在同一前端入口（`index.html`）中区分两个窗口：

- **主窗口（`main`）**：无边框、置顶、跳过任务栏、`visibleOnAllWorkspaces`，承载编辑器。关闭按钮被拦截为隐藏。
- **配置窗口（`config`）**：有边框、置顶、`420×580`，承载平台管理界面。由主窗口触发 `open_config_window` 命令创建。

入口判定见 [src/main.ts](file:///workspace/src/main.ts)：

```typescript
const page = params.get('page')
if (page === 'config') {
  createApp(ConfigWindow).mount('#app')
} else {
  createApp(App).mount('#app')
}
```

---

## 3. 目录结构

```
sensend/
├── index.html                  # HTML 入口
├── package.json                # 前端依赖与脚本
├── package-lock.json           # 前端依赖锁定
├── vite.config.ts              # Vite 配置（端口 1420）
├── tsconfig.json               # TypeScript 配置（strict）
├── tsconfig.node.json
├── README.md
├── LICENSE
├── logo.png
│
├── docs/                       # 文档目录
│   ├── BUILD-GUIDE.md          # 打包发布手册
│   ├── EXPERIENCE.md           # 开发经验手册
│   ├── FILELIST.md             # 源码清单
│   ├── CODE-WIKI.md            # 本文档
│   └── TODO.md                 # 待办事项（v0.3.0 新增）
│
├── scripts/                    # 辅助脚本（v0.3.0 新增）
│   └── build-release.ps1       # Windows 打包脚本（自动加载 MSVC + Rust 环境）
│
├── src/                        # ── 前端源码 ──
│   ├── main.ts                 # 入口：按 URL 参数路由 main/config
│   ├── App.vue                 # 主窗口根组件（编辑器）
│   ├── ConfigWindow.vue        # 配置窗口根组件（平台管理）
│   ├── vite-env.d.ts
│   │
│   ├── components/             # Vue 组件
│   │   ├── TitleBar.vue        # 自定义标题栏（置顶/发送/字体/隐藏）
│   │   ├── FooterBar.vue       # 底栏（目标选择/菜单/字数）
│   │   ├── HotkeyModal.vue     # 快捷键设置弹窗
│   │   ├── FontManager.vue     # 字体管理弹窗
│   │   ├── MentionList.vue      # @提及下拉列表
│   │   └── ToastLayer.vue     # Toast 提示浮层
│   │
│   ├── composables/            # 组合式函数（业务逻辑层）
│   │   ├── useEditor.ts        # TipTap 编辑器核心
│   │   ├── useEditorFormat.ts  # 格式操作（粗体/标题/列表…）
│   │   ├── useEditorFont.ts    # 字体切换与加载
│   │   ├── usePlatform.ts      # 平台实例与发送逻辑
│   │   ├── useConfig.ts        # 配置窗口表单状态机
│   │   ├── useHotkey.ts        # 快捷键录制与拦截
│   │   ├── useSentMark.ts      # 发送成功视觉标记（ProseMirror decoration，v0.4.0 新增）
│   │   └── useToast.ts         # 全局 Toast
│   │
│   ├── types/
│   │   └── platform.ts         # 平台相关 TS 类型与工具函数
│   │
│   └── styles/
│       ├── vars.css            # CSS 变量与主题
│       └── editor.css          # TipTap 编辑器样式
│
└── src-tauri/                  # ── 后端源码（Rust）──
    ├── Cargo.toml              # Rust 依赖
    ├── Cargo.lock
    ├── build.rs                # tauri-build 构建脚本
    ├── tauri.conf.json         # Tauri 应用配置
    ├── capabilities/
    │   └── default.json        # 窗口权限声明
    ├── icons/                  # 应用图标
    │
    └── src/
        ├── main.rs             # 二进制入口
        ├── lib.rs              # 应用主体（插件注册/托盘/窗口事件）
        │
        ├── commands/           # ── Tauri 命令层 ──
        │   ├── mod.rs
        │   ├── note.rs         # 笔记读写/窗口隐藏/退出
        │   ├── platform.rs     # 平台实例 CRUD/测试/发送
        │   ├── hotkey.rs       # 全局快捷键注册与保存
        │   └── font.rs         # 用户字体扫描/删除/打开目录
        │
        └── adapters/           # ── 平台适配器层 ──
            ├── mod.rs          # 公共 trait/类型/ID 解析/HTTP 客户端
            ├── markdown.rs     # TipTap JSON ↔ Markdown 转换
            ├── notion.rs       # Notion 适配器
            ├── flowus.rs       # FlowUs 适配器
            ├── lark.rs         # 飞书适配器
            └── local.rs       # 本地文件夹适配器
```

---

## 4. 前端模块详解

### 4.1 入口与路由：`src/main.ts`

职责：根据 URL 参数 `?page=config` 决定挂载 `App`（主窗口）还是 `ConfigWindow`（配置窗口）。引入全局样式 `vars.css` 与 `editor.css`。两窗口启动时均调用 `initTheme()` 读取后端 `get_theme` 设置 `document.documentElement.dataset.theme`，并监听 `theme-updated` 事件实现跨窗口实时同步（v0.4.0 新增）。ConfigWindow 采用动态 `import()` 懒加载，Rollup 自动分包（v0.4.0 优化）。

### 4.2 主窗口：`src/App.vue`

主窗口的根组件，负责编排各 composable 与子组件。

**编排逻辑**：

1. 调用 `usePlatform()` 获取平台实例列表与发送能力。
2. 调用 `useSensendEditor(instances, platformTypes)` 初始化 TipTap 编辑器，并接收 mention 同步回调。
3. 调用 `useEditorFormat(editor)` 获取格式操作。
4. 调用 `useEditorFont()` 获取字体管理。
5. 调用 `useHotkey(publishNote, () => editor.value, isSending)` 注册发送快捷键拦截。

**mention ↔ 底栏双向同步**（核心设计）：

- 编辑区 mention 变化 → `setOnMentionChange` 回调 → 同步 `activeInstanceId` 并写入后端 `config.json`（通过 `set_default_target`，v0.3.0 起）。
- 底栏选择目标 → `handleFooterSelect` → 调用 `selectTarget` + `setMention`（清除旧 mention、在文档开头插入新 mention）。
- `resolvedTarget` 计算属性：mention 优先于底栏默认目标，作为最终发送目标。

**模板结构**：`TitleBar` + `ToastLayer` + 编辑区（含 `BubbleMenu` + `EditorContent`）+ `FooterBar` + `HotkeyModal` + `FontManager`。

### 4.3 配置窗口：`src/ConfigWindow.vue`

平台实例的增删改查界面，逻辑全部委托给 `useConfig()`。

**两种视图**：
- 列表视图：展示已配置实例卡片（带平台色点、写入模式标签）。
- 表单视图：新增/编辑表单，含名称、平台类型、写入方式（page/block，local 与 lark 隐藏）、动态字段（按平台类型渲染）、测试连接、保存按钮。

字段渲染依据后端 `get_platform_types()` 返回的 `ConfigField` 元数据（`secret/hidden/browse/default_value/optional`）动态生成。

### 4.4 组件层

| 组件 | 文件 | 职责 |
|------|------|------|
| TitleBar | [src/components/TitleBar.vue](file:///workspace/src/components/TitleBar.vue) | 自定义标题栏；递归标记拖拽区域（`data-tauri-drag-region`）；字体菜单、置顶、发送、隐藏按钮 |
| FooterBar | [src/components/FooterBar.vue](file:///workspace/src/components/FooterBar.vue) | 底栏；目标选择器（picker）、设置菜单（配置/快捷键/数据目录）、字数统计 |
| HotkeyModal | [src/components/HotkeyModal.vue](file:///workspace/src/components/HotkeyModal.vue) | 快捷键设置弹窗；焦点陷阱（Tab 循环）、焦点恢复 |
| FontManager | [src/components/FontManager.vue](file:///workspace/src/components/FontManager.vue) | 字体管理弹窗；列出用户字体、删除、打开字体目录 |
| MentionList | [src/components/MentionList.vue](file:///workspace/src/components/MentionList.vue) | `@提及` 下拉项；键盘上下选择、自动滚动、hover 态 |
| ToastLayer | [src/components/ToastLayer.vue](file:///workspace/src/components/ToastLayer.vue) | 全局 Toast 浮层（Teleport 到 body）；success/error/info 三类 |

### 4.5 Composables 层

详见 [第 6 章](#6-关键类与函数说明)。

### 4.6 类型定义：`src/types/platform.ts`

定义前后端共享的平台相关 TypeScript 类型，并提供两个工具函数：

- `getColorForType(types, type)`：按平台 key 返回主题色。
- `getInstanceDisplayName(types, inst)`：生成「实例名-平台类型」统一显示名（如「工作 Notion-Notion」）。

### 4.7 样式：`src/styles/`

- `vars.css`：全局 CSS 变量（配色体系、字体栈）、全局 reset、`@fontsource/dm-sans` 引入。v0.4.0 新增 `[data-theme="dark"]` 暗夜主题变量覆盖（accent 暗变体 `#3dbd7e`）。
- `editor.css`：TipTap 编辑器内容样式（标题、列表、代码块、blockquote、placeholder 等），含 `.sent-mark-badge` 发送成功标记样式（v0.4.0 新增）。

---

## 5. 后端模块详解

### 5.1 入口：`src-tauri/src/main.rs` 与 `lib.rs`

- `main.rs`：二进制入口，仅调用 `sensend_lib::run()`；release 模式下隐藏控制台窗口。
- `lib.rs`：应用主体，`pub fn run()` 中完成：
  - 插件注册：`opener` / `dialog` / `store` / `global-shortcut` / `single-instance`。
  - 命令注册：`invoke_handler!` 注册全部 22 个命令（v0.4.0 新增 `get_theme` / `set_theme`）。
  - `setup` 钩子：创建应用数据目录、初始化全局快捷键、构建系统托盘（显示/退出菜单 + 左键点击显示窗口）。
  - 窗口事件处理：主窗口关闭请求被拦截为 `hide()`（最小化到托盘）。
  - `single-instance` 回调：二次启动时显示并聚焦主窗口。

### 5.2 命令层：`src-tauri/src/commands/`

所有命令通过 `#[tauri::command]` 标注，由前端 `invoke` 调用。

#### 5.2.1 `note.rs` — 笔记与窗口

| 命令 | 签名 | 说明 |
|------|------|------|
| `read_note` | `(app) -> Result<String>` | 读取 `app_data_dir/note.json`（v0.4.0 起），兜底读旧 `note.md` |
| `save_note` | `(app, content: String) -> Result<()>` | 写入 `note.json`（tmp 改名原子写入） |
| `hide_window` | `(app) -> Result<()>` | 隐藏主窗口 |
| `open_data_dir` | `(app) -> Result<()>` | 在文件管理器中打开数据目录 |
| `request_quit` | `(app) -> Result<()>` | `app.exit(0)` 退出应用 |

#### 5.2.2 `platform.rs` — 平台实例与发送

| 命令 | 说明 |
|------|------|
| `open_config_window` | 创建/显示配置窗口；定位在主窗口右下方 40,60 偏移 |
| `get_platform_types` | 返回平台类型元数据（local/notion/flowus/lark）|
| `list_platform_instances` | 从 `config.json` store 读取实例列表 |
| `save_platform_instance` | 新增或更新实例，emit `instances-updated` |
| `delete_platform_instance` | 删除实例，emit `instances-updated` |
| `test_platform_connection` | 调用适配器 `test_connection` |
| `probe_target` | 探测目标类型（page/database/bitable）|
| `publish_note` | 核心：按 `publish_mode` 分发到 `publish` 或 `append_blocks` |
| `get_default_target` | 读取后端 config.json 中记忆的默认发送目标 ID（v0.3.0 新增）|
| `set_default_target` | 写入默认发送目标 ID 到后端 config.json（v0.3.0 新增）|
| `get_theme` | 读取主题设置（light/dark），默认 light（v0.4.0 新增）|
| `set_theme` | 写入主题到 config.json + emit `theme-updated` 事件通知两窗口（v0.4.0 新增）|

**适配器工厂** `get_adapter(platform_type)` 根据 `platform_type` 字符串返回 `Box<dyn PlatformAdapter>`。

#### 5.2.3 `hotkey.rs` — 全局快捷键

| 函数 | 说明 |
|------|------|
| `init_hotkeys(app)` | setup 阶段调用，从 store 读取并注册唤醒快捷键 |
| `register_show_hotkey(app, hotkey_str)` | 注册全局快捷键，按下时切换主窗口显示/隐藏 |
| `get_hotkeys` | 命令：返回 `{show, send}` 配置 |
| `save_hotkeys` | 命令：校验格式 → 注销全部 → 重新注册 → 持久化 |
| `unregister_all_hotkeys` | 注销所有已注册快捷键 |

默认快捷键：唤醒 `Alt+Shift+F`，发送 `Control+Enter`。

> 注：发送快捷键并非全局注册，而是在前端 `useHotkey.ts` 中通过 `document.keydown` 监听拦截（仅当编辑器聚焦时生效）。

#### 5.2.4 `font.rs` — 用户字体

| 命令 | 说明 |
|------|------|
| `scan_user_fonts` | 扫描 `app_data_dir/fonts/` 下的 `ttf/otf/woff2/ttc` 文件，返回 `{name, path}`，path 转为 `https://asset.localhost/...` 供前端 `@font-face` 加载 |
| `open_fonts_dir` | 打开字体目录（不存在则创建）|
| `delete_user_font` | 按显示名删除字体文件 |

`strip_font_weight_suffix`：剥离 `-Regular/-Bold/-Italic` 等字重后缀，使同族不同字重合并为单一显示名。

### 5.3 适配器层：`src-tauri/src/adapters/`

#### 5.3.1 公共模块：`mod.rs`

定义统一接口与共享设施：

- **`http_client()`**：全局 `reqwest::Client`（`OnceLock` 单例，15s 超时，连接池复用）。
- **`PlatformInstance`**：用户配置的平台实例结构（id/name/platform_type/token/token2/target_id/publish_mode）。
- **`PlatformTypeInfo` / `ConfigField`**：平台类型元数据，驱动前端表单动态渲染。
- **`PlatformAdapter` trait**：统一适配器接口（见 [6.1](#61-platformadapter-trait)）。
- **`PublishResult` / `ProbeResult`**：发布与探测结果。
- **`get_platform_types()`**：返回四种平台的字段定义。
- **`resolve_target_id(platform_type, raw)`**：从 URL 或纯文本提取平台 ID（分发到 `resolve_notion_id` / `resolve_flowus_id` / `LarkAdapter::resolve_lark_id`）。

#### 5.3.2 公共转换：`markdown.rs`

TipTap JSON ↔ Markdown 转换，供 `local.rs` 与 `flowus.rs` 复用：

- `tiptap_to_markdown(tree)`：递归渲染 paragraph/heading/list/codeBlock/blockquote/horizontalRule/hardBreak/table，处理嵌套列表与 marks（粗体/斜体/删除线/代码/链接），mention 输出为 `@名称`。表格输出标准 GFM 语法（`| cell |` + `| --- |` 分隔行）。
- `extract_title(content)`：优先取首个 heading 文本，兜底取首个非空段落，截取前 18 字。
- `extract_plain_text(node)`：忽略格式的纯文本提取。

#### 5.3.3 适配器实现

| 适配器 | 文件 | 关键能力 |
|--------|------|---------|
| `LocalAdapter` | [local.rs](file:///workspace/src-tauri/src/adapters/local.rs) | 写入 `标题_时间戳.md`（UTF-8 BOM）；`test_connection` 通过写测试文件验证权限 |
| `NotionAdapter` | [notion.rs](file:///workspace/src-tauri/src/adapters/notion.rs) | `resolve_target` 三步试探法判断 Database/Page；`extract_schema_from_properties` 提取 title/date 列；TipTap→Notion blocks；分块追加（每批 100）|
| `FlowUsAdapter` | [flowus.rs](file:///workspace/src-tauri/src/adapters/flowus.rs) | `resolve_target` 检测 child_database；TipTap→FlowUs blocks（带完整 annotations）；创建页面后追加内容 |
| `LarkAdapter` | [lark.rs](file:///workspace/src-tauri/src/adapters/lark.rs) | App ID+Secret 换 `tenant_access_token`；wiki URL 解析 `node_token`→`document_id`；TipTap→飞书 blocks（block_type 常量）；仅追加模式 |

---

## 6. 关键类与函数说明

### 6.1 `PlatformAdapter` trait

[adapters/mod.rs](file:///workspace/src-tauri/src/adapters/mod.rs#L115-L129) 定义统一接口：

```rust
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    async fn publish(&self, content: &Value, instance: &PlatformInstance)
        -> Result<PublishResult, String>;
    async fn test_connection(&self, instance: &PlatformInstance) -> Result<(), String>;
    async fn probe_type(&self, _instance: &PlatformInstance) -> Result<String, String> { Ok("page".into()) }
    async fn append_blocks(&self, _content: &Value, _instance: &PlatformInstance)
        -> Result<PublishResult, String> { Err("该平台不支持追加写入".into()) }
}
```

- `publish`：创建新子页面（page 模式）。
- `append_blocks`：追加到已有页面（block 模式），默认不支持。
- `probe_type`：探测目标类型，默认 `"page"`。
- `test_connection`：测试连通性。

### 6.2 前端 Composables

#### `useSensendEditor(instances, platformTypes)` — [useEditor.ts](file:///workspace/src/composables/useEditor.ts)

编辑器核心。返回 `editor/saveStatus/wordCount/charCount/getMentionId/setMention/setOnMentionChange/markSent`。

关键内部逻辑：
- **TipTap 初始化**：`StarterKit` + `Placeholder` + `Mention`（自定义 `renderHTML`、`suggestion` 配置）+ `TableKit`（表格扩展，v0.2.0 起）+ `SentMarkExtension`（发送成功标记，v0.4.0 新增）。
- **mention 唯一性**：`suggestion.command` 在插入新 mention 前先删除所有旧 mention，并修正 range 偏移。
- **mention 渲染**：基于 `tippy.js` + `VueRenderer(MentionList)`，根据剩余空间自动判断向上/向下弹出。
- **自动保存**：`onUpdate` 触发防抖 800ms → `doSave` → `invoke('save_note')`。`doSave` 内置 `saveStatus === 'saving'` 并发守卫（v0.4.0 新增）。
- **退出前保存**：监听 `app-exit-request` 事件，仅在 `saveStatus === 'unsaved'` 时执行 `doSave`（v0.4.0 优化，避免重复写入）。
- **字数统计**：中文按字符计、英文按单词计。

#### `usePlatform()` — [usePlatform.ts](file:///workspace/src/composables/usePlatform.ts)

平台实例管理与发送。返回 `instances/activeInstanceId/platformTypes/isSending/selectTarget/openConfigWindow/publishNote/reloadInstances`。

关键逻辑：
- `publishNote(editorValue, overrideTargetId)`：返回 `Promise<boolean>`，发送成功返回 true。剔除 mention 节点 → `invoke('publish_note')` → 成功 Toast 带「查看 ↗」按钮 → 记忆目标到后端 `config.json`。App.vue 在成功时调用 `markSent(editor)` 显示已发送标记。
- `friendlyError(raw)`：仅做网络断开兜底判断，其余错误消息原样透传（v0.4.0 简化，后端错误消息已包含足够上下文）。
- 监听 `instances-updated` 事件自动刷新。

#### `useConfig()` — [useConfig.ts](file:///workspace/src/composables/useConfig.ts)

配置窗口状态机。返回表单状态、`canSave` 计算属性、`loadData/openAddModal/openEditModal/browseFolder/testConnection/saveInstance/deleteInstance`。

`canSave`：名称非空且所有非可选非隐藏字段已填。

#### `useEditorFormat(editor)` — [useEditorFormat.ts](file:///workspace/src/composables/useEditorFormat.ts)

纯函数集合，封装 TipTap chain 操作（toggleBold/toggleH1/toggleBulletList…）与 `isActive` 判定。

#### `useEditorFont()` — [useEditorFont.ts](file:///workspace/src/composables/useEditorFont.ts)

字体切换与加载。`applyUserFonts` 动态注入 `@font-face` style，更新 `--font-editor` CSS 变量；当前字体被删除时回退到默认。

#### `useHotkey(publishNote, editorRef, isSending)` — [useHotkey.ts](file:///workspace/src/composables/useHotkey.ts)

- `handleSendHotkey`：document 级 keydown 监听，匹配发送快捷键时调用 `publishNote`（仅编辑器聚焦且非发送中）。
- `onKeyDownForHotkey`：弹窗内快捷键录制，组装 `Control+Alt+Shift+Key` 格式串。
- `saveHotkeys`：调用后端 `save_hotkeys`。

#### `useToast()` — [useToast.ts](file:///workspace/src/composables/useToast.ts)

模块级单例 Toast 队列，提供 `success/error/info/remove`，自动定时移除。

### 6.3 关键后端函数

| 函数 | 位置 | 说明 |
|------|------|------|
| `resolve_target_id` | [mod.rs](file:///workspace/src-tauri/src/adapters/mod.rs#L139) | 平台 ID 解析分发 |
| `resolve_notion_id` | [mod.rs](file:///workspace/src-tauri/src/adapters/mod.rs#L156) | 从右向左扫描 32 位 hex，兼容 UUID 格式 |
| `NotionAdapter::resolve_target` | [notion.rs](file:///workspace/src-tauri/src/adapters/notion.rs#L274) | 三步试探：直接查 Database → 查子块 child_database → 兜底 Page |
| `NotionAdapter::extract_schema_from_properties` | [notion.rs](file:///workspace/src-tauri/src/adapters/notion.rs#L250) | 提取 title 列与 date 列名 |
| `LarkAdapter::get_tenant_token` | [lark.rs](file:///workspace/src-tauri/src/adapters/lark.rs#L37) | App ID+Secret 换租户 token |
| `LarkAdapter::resolve_document_id` | [lark.rs](file:///workspace/src-tauri/src/adapters/lark.rs#L139) | wiki URL → document_id（调用 `/wiki/v2/spaces/get_node`）|
| `tiptap_to_markdown` | [markdown.rs](file:///workspace/src-tauri/src/adapters/markdown.rs#L7) | TipTap JSON → Markdown 文本 |
| `extract_title` | [markdown.rs](file:///workspace/src-tauri/src/adapters/markdown.rs#L23) | 提取文档标题（前 18 字）|

---

## 7. 数据流与核心交互

### 7.1 编辑器自动保存

```
用户输入
   │
   ▼
TipTap onUpdate
   │
   ├─► saveStatus = 'unsaved'
   ├─► autoSave() ── 防抖 800ms ──► doSave()
   ├─► updateWordCount()
   └─► onMentionChange(mentionId)  // 通知 App.vue 同步底栏
                                    │
                                    ▼
                         doSave: invoke('save_note', { content: JSON.stringify(getJSON()) })
                                    │
                                    ▼
                          后端写 app_data_dir/note.json
```

### 7.2 `@mention` 机制

**唯一性约束**：文档中同时只允许一个 mention 节点。

**输入触发**：键入 `@` → TipTap suggestion `onStart` → tippy 弹出 `MentionList`（自动判断上下空间翻转）→ 键盘/鼠标选择 → `suggestion.command`：

1. 收集所有 mention 节点 range。
2. 倒序删除旧 mention，并修正 `range.from/to` 偏移。
3. 删除 `@触发字符 + 查询文本`。
4. 插入新 mention 节点，光标移到其后。

**底栏选择同步**：`handleFooterSelect(id)` → `setMention(id)`：删除所有 mention + 在文档开头插入新 mention + 空格。

### 7.3 发送流程

```
publishNote(editor, resolvedTargetId)
        │
        ├─ 1. 提取纯文本，空内容则报错
        ├─ 2. 检查 navigator.onLine
        ├─ 3. stripMentions(jsonTree)  // 剔除 mention 节点
        │
        ▼
   invoke('publish_note', { instanceId, content: jsonTree })
        │
        ▼
   platform.rs::publish_note
        ├─ 查找 instance
        ├─ get_adapter(platform_type)
        │
        ├─ publish_mode == "block" ─► adapter.append_blocks(content, instance)
        └─ publish_mode == "page"  ─► adapter.publish(content, instance)
                                        │
                                        ▼
                                  PublishResult { success, message, url }
                                        │
                                        ▼
   前端 Toast 成功 + 「查看 ↗」按钮 (openUrl)
   markSent(editor) → 编辑区末尾显示"✓ 已发送"徽章（3 秒后自动消失）
```

### 7.4 快捷键机制

| 快捷键 | 注册位置 | 作用域 |
|--------|---------|--------|
| 唤醒（默认 `Alt+Shift+F`） | 后端 `tauri-plugin-global-shortcut` | 系统全局 |
| 发送（默认 `Control+Enter`） | 前端 `document.keydown` | 编辑器聚焦时 |
| `Esc` | 前端 | 隐藏窗口 / 关闭弹窗 |

唤醒快捷键在 `lib.rs::setup` 中通过 `init_hotkeys` 注册，按下时切换主窗口可见性。

### 7.5 配置窗口更新广播

```
配置窗口 saveInstance/deleteInstance
        │
        ▼
   invoke('save_platform_instance' / 'delete_platform_instance')
        │
        ▼
   后端持久化 + app.emit("instances-updated", ())
        │
        ▼
   主窗口 usePlatform 监听 → reloadInstances()
```

---

## 8. 依赖关系

### 8.1 前端依赖（package.json）

**运行时**：

| 依赖 | 用途 |
|------|------|
| `vue` ^3.5.13 | 响应式 UI 框架 |
| `@tiptap/vue-3` / `@tiptap/starter-kit` / `@tiptap/pm` | 富文本编辑器 |
| `@tiptap/extension-mention` | `@提及` 扩展 |
| `@tiptap/extension-table` | 表格扩展（TableKit，v0.2.0 起）|
| `@tiptap/markdown` | Markdown 解析/序列化扩展 |
| `@tiptap/extension-placeholder` | 空内容占位 |
| `tippy.js` | Mention 下拉弹层 |
| `@tauri-apps/api` | Tauri IPC（invoke/listen）|
| `@tauri-apps/plugin-dialog` | 文件夹选择对话框 |
| `@tauri-apps/plugin-global-shortcut` | 快捷键（前端类型）|
| `@tauri-apps/plugin-opener` | `openUrl` 打开链接 |
| `@fontsource/dm-sans` | UI 字体（离线）|

**开发时**：`@tauri-apps/cli`、`@vitejs/plugin-vue`、`typescript`、`vite`、`vue-tsc`。

### 8.2 后端依赖（Cargo.toml）

| 依赖 | 用途 |
|------|------|
| `tauri` 2 (tray-icon) | 桌面框架核心 + 系统托盘 |
| `tauri-plugin-opener` | 打开 URL/文件 |
| `tauri-plugin-store` | JSON 持久化（config.json）|
| `tauri-plugin-single-instance` | 单实例保护 |
| `tauri-plugin-dialog` | 文件夹选择 |
| `tauri-plugin-global-shortcut` | 全局快捷键 |
| `reqwest` 0.12 (rustls-tls, json) | HTTP 客户端 |
| `tokio` 1 (full) | 异步运行时 |
| `serde` / `serde_json` | 序列化 |
| `async-trait` | trait 异步方法 |
| `chrono` | 时间戳生成 |
| `open` | 跨平台打开目录 |
| `winreg` 0.55 | Windows 注册表（平台检测）|
| `log` | 日志 |

### 8.3 模块间依赖（Rust）

```
lib.rs
  ├── commands::{note, platform, hotkey, font}
  │     └── adapters (via platform.rs::get_adapter)
  └── adapters::{mod, markdown, notion, flowus, lark, local}
        ├── mod.rs: PlatformAdapter trait, http_client, 类型
        ├── markdown.rs: 被 local / flowus 复用
        └── notion/flowus/lark: 各自实现 TipTap → 平台 blocks
```

### 8.4 前端模块依赖

```
App.vue
  ├── usePlatform ── useToast
  ├── useSensendEditor ── types/platform, MentionList.vue
  ├── useEditorFormat
  ├── useEditorFont
  └── useHotkey
ConfigWindow.vue
  └── useConfig ── types/platform
```

---

## 9. 配置与运行方式

### 9.1 环境要求

| 软件 | 版本 | 用途 |
|------|------|------|
| Node.js | 18+ | 前端构建 |
| Rust | 最新稳定版 | 后端编译 |
| Tauri CLI | 2.x | 打包（`npm install -g @tauri-apps/cli`）|

> Windows 打包 NSIS 安装包需额外下载 NSIS 3.11 与 `nsis_tauri_utils.dll` v0.5.3（详见 `docs/BUILD-GUIDE.md`）。

### 9.2 开发运行

```bash
# 安装依赖
npm install

# 开发模式（前端 + 后端热重载，端口 1420）
npm run tauri dev
```

### 9.3 构建发布

```bash
# 完整构建（vue-tsc 类型检查 + vite build + cargo build）
npm run tauri build
```

**产物**：
- 便携版：`src-tauri/target/release/sensend.exe`
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/Sensend_<版本号>_x64-setup.exe`

### 9.4 前端脚本

| 脚本 | 说明 |
|------|------|
| `npm run dev` | 仅启动 Vite 开发服务器 |
| `npm run build` | `vue-tsc --noEmit && vite build` |
| `npm run preview` | Vite 预览构建产物 |
| `npm run tauri` | 透传 tauri CLI |

### 9.5 Tauri 配置要点（`tauri.conf.json`）

- **窗口**：`main` 窗口 `420×210`，`decorations: false`、`alwaysOnTop: true`、`skipTaskbar: true`、`visibleOnAllWorkspaces: true`。
- **构建**：`beforeDevCommand: npm run dev`、`devUrl: http://localhost:1420`、`frontendDist: ../dist`。
- **打包**：`targets: ["nsis"]`，安装模式 `currentUser`，语言中英文。
- **CSP**：允许 `ipc:`、`https:` 连接，允许 `asset.localhost` 加载本地字体与图片。
- **标识符**：`com.jloft.sensend`。

### 9.6 权限声明（`capabilities/default.json`）

授权 `main` 与 `config` 两个窗口：`core:default`、`core:window:allow-start-dragging`、`core:window:allow-set-always-on-top`、`opener:default`、`dialog:default`。

### 9.7 Vite 配置要点（`vite.config.ts`）

- 固定端口 `1420`（`strictPort: true`），与 Tauri devUrl 对齐。
- 忽略监听 `src-tauri/**`。
- 支持 `TAURI_DEV_HOST` 环境变量（移动端/远程开发 HMR）。

---

## 10. 数据存储

### 10.1 存储位置

所有持久化数据位于 Tauri 应用数据目录（`app_data_dir`）：

| 文件/目录 | 内容 |
|----------|------|
| `note.json` | 当前编辑器内容（TipTap JSON 字符串，v0.4.0 起从 note.md 迁移；旧 note.md 自动兜底读取）|
| `config.json` | 平台实例列表、快捷键配置、默认目标、主题设置 |
| `fonts/` | 用户字体文件（ttf/otf/woff2/ttc）|

> Windows 默认路径：`C:\Users\<user>\AppData\Roaming\com.jloft.sensend\`

### 10.2 `config.json` 结构

由 `tauri-plugin-store` 管理，键值：

- `platform_instances`：`PlatformInstance[]`。
- `hotkey_show`：唤醒快捷键字符串。
- `hotkey_send`：发送快捷键字符串。
- `default_target`：上次发送/选择的平台实例 ID，用于启动时恢复默认目标（v0.3.0 起从 localStorage 迁移至此）。
- `theme`：主题设置（`light` / `dark`），默认 `light`（v0.4.0 新增）。

### 10.3 前端 localStorage

> v0.3.0 起，默认发送目标已迁移到后端 `config.json`，前端不再使用 localStorage 存储业务数据。

---

## 11. 平台适配器扩展指南

新增一个平台适配器（如「语雀」）需完成以下步骤：

### 11.1 后端

1. **新增适配器文件** `src-tauri/src/adapters/yuque.rs`：
   - 定义 `pub struct YuqueAdapter;`
   - `impl YuqueAdapter` 实现内部 HTTP 请求、TipTap→平台 blocks 转换、目标解析等。
   - `#[async_trait] impl PlatformAdapter for YuqueAdapter`：实现 `publish`、`test_connection`，按需实现 `probe_type`、`append_blocks`。

2. **注册模块**：在 [adapters/mod.rs](file:///workspace/src-tauri/src/adapters/mod.rs) 末尾 `pub mod yuque;`，并在 `get_platform_types()` 中添加 `PlatformTypeInfo`（定义 `key/name/color/fields`）。

3. **工厂分发**：在 [commands/platform.rs](file:///workspace/src-tauri/src/commands/platform.rs#L23) 的 `get_adapter` 中添加 `"yuque" => Ok(Box::new(adapters::yuque::YuqueAdapter::new()))`。

4. **ID 解析**（可选）：若平台 URL 需特殊解析，在 `resolve_target_id` 中增加分支。

### 11.2 前端

- 通常无需改动。前端表单由后端 `get_platform_types()` 返回的 `ConfigField` 元数据驱动动态渲染，平台色点由 `ConfigField.color` 决定。
- 仅当需要平台特定 UI 行为时才需修改 `useConfig.ts` 或 `ConfigWindow.vue`。

### 11.3 适配器实现要点

- **HTTP 客户端**：复用 `super::http_client()`，避免重复创建。
- **错误处理**：返回 `Result<_, String>`，错误消息会经前端 `friendlyError` 转译。
- **分块追加**：Notion/FlowUs 单次限制 100 块，飞书限制 50 块，需 `chunks(N)` 分批。
- **标题提取**：复用 `markdown::extract_title(content)`。
- **日志**：使用 `log::info!` / `log::debug!`，便于调试。

---

## 附：核心设计原则

源自 `docs/EXPERIENCE.md`：

| 原则 | 实践 |
|------|------|
| 极简 | 只做「发送」一件事，无冗余功能 |
| 极致 | 窗口尺寸精确到像素，每个细节打磨 |
| 轻盈 | 主窗口 420×210px，体积小启动快 |
| 优雅 | 无边框窗口、自定义标题栏、流畅交互 |
| 极速 | 全局快捷键秒开，防抖自动保存 |

**代码风格**：用最少的代码实现功能；原子化组件职责；按需加载；错误解释用大白话。
