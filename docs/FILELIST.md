# Sensend v0.1.0 源码目录

> 生成时间：2026-04-29  
> 文件总数：100

---

## 目录结构

```
sensend-release/
│
├── .gitignore
├── index.html
├── LICENSE
├── package.json
├── package-lock.json
├── README.md
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
│
├── docs/
│   └── FILELIST.md
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
    │   ├── icon.icns
    │   ├── icon.ico
    │   ├── icon.png
    │   │
    │   ├── android/
    │   │   ├── mipmap-anydpi-v26/
    │   │   │   └── ic_launcher.xml
    │   │   ├── mipmap-hdpi/
    │   │   │   ├── ic_launcher.png
    │   │   │   ├── ic_launcher_foreground.png
    │   │   │   └── ic_launcher_round.png
    │   │   ├── mipmap-mdpi/
    │   │   │   ├── ic_launcher.png
    │   │   │   ├── ic_launcher_foreground.png
    │   │   │   └── ic_launcher_round.png
    │   │   ├── mipmap-xhdpi/
    │   │   │   ├── ic_launcher.png
    │   │   │   ├── ic_launcher_foreground.png
    │   │   │   └── ic_launcher_round.png
    │   │   ├── mipmap-xxhdpi/
    │   │   │   ├── ic_launcher.png
    │   │   │   ├── ic_launcher_foreground.png
    │   │   │   └── ic_launcher_round.png
    │   │   ├── mipmap-xxxhdpi/
    │   │   │   ├── ic_launcher.png
    │   │   │   ├── ic_launcher_foreground.png
    │   │   │   └── ic_launcher_round.png
    │   │   └── values/
    │   │       └── ic_launcher_background.xml
    │   │
    │   └── ios/
    │       ├── AppIcon-20x20@1x.png
    │       ├── AppIcon-20x20@2x.png
    │       ├── AppIcon-20x20@2x-1.png
    │       ├── AppIcon-20x20@3x.png
    │       ├── AppIcon-29x29@1x.png
    │       ├── AppIcon-29x29@2x.png
    │       ├── AppIcon-29x29@2x-1.png
    │       ├── AppIcon-29x29@3x.png
    │       ├── AppIcon-40x40@1x.png
    │       ├── AppIcon-40x40@2x.png
    │       ├── AppIcon-40x40@2x-1.png
    │       ├── AppIcon-40x40@3x.png
    │       ├── AppIcon-512@2x.png
    │       ├── AppIcon-60x60@2x.png
    │       ├── AppIcon-60x60@3x.png
    │       ├── AppIcon-76x76@1x.png
    │       ├── AppIcon-76x76@2x.png
    │       └── AppIcon-83.5x83.5@2x.png
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
| 根目录 | 9 | 配置文件 |
| src/ | 20 | 前端源码 |
| src-tauri/src/ | 13 | 后端源码 |
| src-tauri/icons/ | 57 | 图标资源 |
| src-tauri/capabilities/ | 1 | Tauri 权限配置 |
| docs/ | 1 | 文档 |
| **合计** | **101** | |

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
3. 构建发布：`npm run tauri build`

---

> 作者：简乐  
> 致谢：送给儿子小柏