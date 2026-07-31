# Sensend v0.3.0 源码目录

> 生成时间：2026-07-31（对齐 v0.3.0 发布状态）
> 仓库：[github.com/jloft198479-cyber/sensend](https://github.com/jloft198479-cyber/sensend)

---

## 目录结构

```
sensend/
│
├── .gitignore
├── index.html
├── LICENSE
├── logo.png
├── package.json
├── package-lock.json
├── README.md
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
│
├── docs/
│   ├── BUILD-GUIDE.md          # 打包发布手册
│   ├── CODE-WIKI.md            # 代码百科文档
│   ├── EXPERIENCE.md           # 开发经验手册
│   ├── FILELIST.md             # 本文档
│   └── TODO.md                 # 待办事项（v0.3.0 新增）
│
├── scripts/                    # 辅助脚本（v0.3.0 新增）
│   └── build-release.ps1       # Windows 打包脚本（自动加载 MSVC + Rust 环境）
│
├── src/
│   ├── App.vue
│   ├── ConfigWindow.vue
│   ├── main.ts
│   ├── vite-env.d.ts
│   │
│   ├── components/
│   │   ├── FontManager.vue
│   │   ├── FooterBar.vue
│   │   ├── HotkeyModal.vue
│   │   ├── MentionList.vue
│   │   ├── TitleBar.vue
│   │   └── ToastLayer.vue
│   │
│   ├── composables/
│   │   ├── useConfig.ts
│   │   ├── useEditor.ts
│   │   ├── useEditorFont.ts
│   │   ├── useEditorFormat.ts
│   │   ├── useHotkey.ts
│   │   ├── usePlatform.ts
│   │   └── useToast.ts
│   │
│   ├── styles/
│   │   ├── editor.css
│   │   └── vars.css
│   │
│   └── types/
│       └── platform.ts
│
└── src-tauri/
    ├── .gitignore
    ├── build.rs
    ├── Cargo.lock
    ├── Cargo.toml
    ├── tauri.conf.json
    │
    ├── capabilities/
    │   └── default.json
    │
    ├── icons/
    │   ├── 128x128.png
    │   ├── 128x128@2x.png
    │   ├── 32x32.png
    │   ├── 64x64.png
    │   ├── Square107x107Logo.png
    │   ├── Square142x142Logo.png
    │   ├── Square150x150Logo.png
    │   ├── Square284x284Logo.png
    │   ├── Square30x30Logo.png
    │   ├── Square310x310Logo.png
    │   ├── Square44x44Logo.png
    │   ├── Square71x71Logo.png
    │   ├── Square89x89Logo.png
    │   ├── StoreLogo.png
    │   ├── icon.icns
    │   ├── icon.ico
    │   └── icon.png
    │
    └── src/
        ├── lib.rs
        ├── main.rs
        │
        ├── adapters/
        │   ├── flowus.rs
        │   ├── lark.rs
        │   ├── local.rs
        │   ├── markdown.rs
        │   ├── mod.rs
        │   └── notion.rs
        │
        └── commands/
            ├── font.rs
            ├── hotkey.rs
            ├── mod.rs
            ├── note.rs
            └── platform.rs
```

---

## 文件统计

| 目录 | 文件数 | 说明 |
|------|--------|------|
| 根目录 | 11 | 配置文件 + README + LICENSE |
| docs/ | 5 | 文档（v0.3.0 新增 TODO.md）|
| scripts/ | 1 | 打包脚本（v0.3.0 新增）|
| src/ | 20 | 前端源码 |
| src-tauri/src/ | 13 | 后端源码 |
| src-tauri/icons/ | 17 | 图标资源 |
| src-tauri/capabilities/ | 1 | Tauri 权限配置 |
| **合计** | **68** | |

> 历史版本（v0.1.0）曾包含 android/ios 图标目录，v0.3.0 已移除移动端图标，仅保留桌面端。

---

## 技术栈

- **前端**：Vue 3 + TipTap + TypeScript + Vite
- **后端**：Rust + Tauri 2
- **平台 API**：Notion / FlowUs / 飞书

---

## 恢复说明

凭此目录可完整恢复 Sensend 项目：

1. 安装依赖：`npm install`
2. 开发运行：`npm run tauri dev`
3. 构建发布：`npm run tauri build`（或 Windows 自定义环境用 `scripts\build-release.ps1`）

---

> 作者：简乐
> 致谢：送给儿子小柏
