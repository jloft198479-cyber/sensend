# Sensend

> 当前版本：**v0.4.0**

超轻量级桌面悬浮记事本，一键发送到 Notion / FlowUs / 飞书 / 本地文件夹。

## 特性

- **极速启动** — 悬浮窗口，全局快捷键唤醒，随时记录灵感
- **富文本编辑** — 支持标题、列表、引用、代码块，以及待办清单、下划线
- **多平台支持** — Notion、FlowUs、飞书、本地文件夹
- **自定义字体** — 选择你喜欢的编辑器字体
- **极简设计** — 无冗余，专注写作本身

## v0.4.0 更新

- **暗夜模式**：浅色/暗夜双主题，两窗口实时同步
- **发送成功标记**：发送后编辑区末尾显示"✓ 已发送"徽章，3 秒自动消失
- **note.json 迁移**：存储格式从 note.md 迁移到 note.json，旧文件自动兜底读取
- **飞书代码块语言映射**：覆盖 34 个别名，未知语言回落 PlainText
- **下划线兜底**：`<u>` HTML 标签粘贴兜底解析
- **saveStatus 并发守卫**：防止防抖保存期间重复写入
- **ConfigWindow 动态 import**：Rollup 自动分包，减小首屏体积
- **退出前等待在途保存**：防 app.exit 截断原子写丢最后一次编辑
- **授权码按平台记忆**：新增页面自动预填上次填写的 token/token2
- **Notion 表格修复**：表格单元格 rich_text 完整保留（不再只取首元素）
- **明暗主题入口移至主窗口顶栏**：'添加页面'更名'添加和修改页面'

## v0.3.0 更新

- **默认发送目标迁移**：从 localStorage 改为后端 config.json 存储，更可靠
- **适配器增强**：flowus / notion / local 适配器健壮性与能力提升
- **打包脚本**：新增 `scripts/build-release.ps1`，自动加载 MSVC 与 Rust 自定义环境
- **文档**：BUILD-GUIDE 打包说明更新

## v0.2.0 更新

- **新增格式**：待办清单（task list）、下划线，四大平台适配器同步支持
- **健壮性提升**：修复适配器多处边界问题，发送更稳定
- **性能优化**：飞书访问令牌进程内缓存（减少重复鉴权请求）、Notion 目标解析并行化（发送更快）

## 安装

前往 [Releases](https://github.com/jloft198479-cyber/sensend/releases) 下载最新版本。

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
| `Alt+Shift+F` | 全局唤醒窗口（可自定义） |
| `Ctrl+Enter` | 发送内容（可自定义） |
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

> Windows 打包若 Rust / VS Build Tools 装在自定义路径，推荐使用打包脚本：
> `powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"`
> 详见 [docs/BUILD-GUIDE.md](docs/BUILD-GUIDE.md)。

### 技术栈

- **前端**：Vue 3 + TipTap + TypeScript
- **后端**：Rust + Tauri 2
- **平台 API**：Notion API / FlowUs API / 飞书开放平台

## 致谢

这款工具送给我的儿子 **小柏**，愿你永远保持好奇心。

## 作者

**简乐** ([@jloft198479-cyber](https://github.com/jloft198479-cyber))

邮箱：jloft198479@gmail.com

## 许可证

[MIT License](LICENSE)

Copyright (c) 2026 简乐
