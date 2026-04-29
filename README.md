# Sensend

超轻量级桌面悬浮记事本，一键发送到 Notion / FlowUs / 飞书 / 本地文件夹。

## 特性

- **极速启动** — 悬浮窗口，全局快捷键唤醒，随时记录灵感
- **富文本编辑** — 支持标题、列表、引用、代码块等格式
- **多平台支持** — Notion、FlowUs、飞书、本地文件夹
- **自定义字体** — 选择你喜欢的编辑器字体
- **极简设计** — 无冗余，专注写作本身

## 安装

前往 [Releases](https://github.com/jloft/sensend/releases) 下载最新版本。

## 快速开始

1. 首次启动，点击「配置页面」添加目标平台
2. 输入内容，使用 `@` 提及发送目标
3. 按 `Ctrl+Enter` 发送

## 支持的平台

| 平台 | 创建子页面 | 追加内容 | 说明 |
|------|-----------|---------|------|
| Notion | ✅ | ✅ | 支持 Database |
| FlowUs | ✅ | ✅ | 支持多维表 |
| 飞书 | ❌ | ✅ | 仅支持追加到文档 |
| 本地 | ✅ | ❌ | 创建 .md 文件 |

## 快捷键

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+Shift+N` | 全局唤醒窗口 |
| `Ctrl+Enter` | 发送内容 |
| `Esc` | 隐藏窗口 |

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建发布版本
npm run tauri build
```

### 技术栈

- **前端**：Vue 3 + TipTap + TypeScript
- **后端**：Rust + Tauri 2
- **平台 API**：Notion API / FlowUs API / 飞书开放平台

## 致谢

这款工具送给我的儿子 **小柏**，愿你永远保持好奇心。

## 作者

**简乐** ([@jloft](https://github.com/jloft))

邮箱：jloft198479@gmail.com

## 许可证

[MIT License](LICENSE)

Copyright (c) 2026 简乐