# Sensend 打包与发布手册

> 作者：简乐
> 更新时间：2026-08-19（对齐 v0.4.0 主题化收尾；已并入原 BUILD-ENV.md）

---

## 〇、构建环境速查（智能体必读）

**铁律（违反必挂）**：
- ✅ **唯一正确构建方式**——走打包脚本：
  ```powershell
  cd F:\fzz-Project\sensend\sensend
  powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
  ```
- ❌ **禁止**在普通终端直接执行 `npm run tauri build`、`cargo build`、`npm run tauri dev`。

**原因（一句话）**：Rust（`M:\rust`）和 VS Build Tools（`M:\VS\BuildTools`）都**不在系统 PATH**，普通终端找不到 `cargo` / `cl.exe`。只有脚本会设置 `CARGO_HOME` / `RUSTUP_HOME` 并调用 `vcvars64.bat` 加载 MSVC 环境（INCLUDE/LIB）。

> **环境本身完好，报错 ≠ 环境坏，无需重装、无需修复。**

**环境现状（已核验 2026-08-17）**：

| 项 | 值 |
|---|---|
| 工具链 | `stable-x86_64-pc-windows-msvc`（rustc / cargo 1.96.0） |
| MSVC | 14.44.35207（`cl.exe` 就位） |
| NSIS | `makensis.exe` + `nsis_tauri_utils.dll` v0.5.3 就位 |
| cargo 源 | 中科大 sparse 镜像 |

**调试后端单独跑 cargo**（须先手动加载环境）：

```powershell
$env:CARGO_HOME = "M:\rust\.cargo"
$env:RUSTUP_HOME = "M:\rust\.rustup"
$env:PATH = "M:\rust\.cargo\bin;$env:PATH"
# 还需在 x64 Native Tools 终端里，或先执行 vcvars64.bat 注入 INCLUDE/LIB
```

---

## 一、环境准备

### 1.1 必需软件

| 软件 | 版本 | 用途 | 下载地址 |
|------|------|------|----------|
| Node.js | 18+ | 前端构建 | https://nodejs.org |
| Rust | 最新稳定版 | 后端编译 | https://rustup.rs |
| Git | 2.x | 版本控制 | https://git-scm.com |
| Tauri CLI | 2.x | 打包工具 | `npm install -g @tauri-apps/cli` |

### 1.2 Tauri 打包依赖

Windows 平台打包 NSIS 安装包需要额外下载：

| 组件 | 下载地址 | 存放位置 |
|------|----------|----------|
| NSIS 3.11 | https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip | `%LOCALAPPDATA%\tauri\NSIS\` |
| nsis_tauri_utils.dll v0.5.3 | https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll | `%LOCALAPPDATA%\tauri\NSIS\Plugins\x86-unicode\additional\` |

---

## 二、打包步骤

### 2.1 安装依赖

```bash
cd F:\fzz-Project\sensend\sensend
npm install
```

### 2.2 开发测试

```bash
npm run tauri dev
```

### 2.3 构建发布

#### 方式一：打包脚本（推荐）

本机的 Rust 和 VS Build Tools 都装在 M 盘自定义位置，普通终端的 PATH 里找不到 `cargo` 和 `cl.exe`，直接跑 `npm run tauri build` 会失败。项目内置了打包脚本，会自动加载环境：

```bash
cd F:\fzz-Project\sensend\sensend
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
```

脚本做三件事：
1. 设置 `CARGO_HOME` / `RUSTUP_HOME` 指向 `M:\rust\.cargo` / `M:\rust\.rustup`
2. 调用 `M:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat` 加载 MSVC 环境（`cl.exe` / `link.exe` / INCLUDE / LIB 等）
3. 执行 `npm run tauri build`

**为什么需要加载 MSVC 环境？** Rust 默认用 `stable-x86_64-pc-windows-msvc` toolchain，依赖 `ring`、`reqwest` 等 crate 需要 C 编译器（`cl.exe`）。VS Build Tools 装好后，`cl.exe` 在 `M:\VS\BuildTools\VC\Tools\MSVC\<版本>\bin\HostX64\x64\` 下，但它的 INCLUDE/LIB 环境变量必须通过 `vcvars64.bat` 加载，裸跑 cargo 找不到。

#### 方式二：手动构建（需已加载 MSVC 环境）

如果已经在「x64 Native Tools Command Prompt」或开发者版 PowerShell 里（环境已加载），可以直接：

```bash
npm run tauri build
```

#### 构建产物位置

- 便携版：`src-tauri\target\release\sensend.exe`
- 安装包：`src-tauri\target\release\bundle\nsis\Sensend_<版本号>_x64-setup.exe`

---

## 三、踩坑记录

### 3.1 NSIS 插件缺失

**现象**：
```
Warn NSIS directory contains mis-hashed files. Redownloading them.
failed to bundle project `io: Connection refused`
```

**原因**：Tauri 需要特殊的 NSIS 目录结构，缺少 `nsis_tauri_utils.dll` 插件。

**解决方案**：

1. 创建目录结构：
```
%LOCALAPPDATA%\tauri\NSIS\
├── makensis.exe
├── Bin\
├── Include\
├── Plugins\
│   └── x86-unicode\
│       └── additional\
│           └── nsis_tauri_utils.dll
└── Stubs\
```

2. 下载 NSIS 3.11 并解压到 `%LOCALAPPDATA%\tauri\NSIS\`

3. 下载 `nsis_tauri_utils.dll` 放到 `Plugins\x86-unicode\additional\` 目录

### 3.2 NSIS 插件版本不对

**现象**：
```
Warn NSIS directory contains mis-hashed files. Redownloading them.
Downloading nsis_tauri_utils-v0.5.3...
failed to bundle project
```

**原因**：Tauri 2.x 需要 `nsis_tauri_utils.dll` v0.5.3，而不是 v0.4.1。

**解决方案**：下载正确版本的插件：
```
https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll
```

### 3.3 安装包没有图标

**现象**：生成的安装包显示默认图标，不是自定义 Logo。

**原因**：需要在 `tauri.conf.json` 中配置 `windows.nsis` 选项。

**解决方案**：

```json
{
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "currentUser",
        "languages": ["SimpChinese", "English"]
      }
    }
  }
}
```

### 3.4 Git 不在 PATH 中

**现象**：
```
git : The term 'git' is not recognized
```

**原因**：Git 安装在非标准路径，未添加到 PATH。

**解决方案**：使用完整路径调用 Git：
```bash
D:\Git\bin\git.exe --version
```

或添加到 PATH：
```powershell
$env:PATH += ";D:\Git\bin"
```

### 3.5 Windows 安全警告

**现象**：下载安装包时浏览器提示"无法识别的应用"。

**原因**：安装包未购买代码签名证书。

**解决方案**：

临时方案：在 Release 说明中添加提示
> Windows 可能提示"无法识别的应用"，这是因为安装包未购买代码签名证书。点击"更多信息" → "仍要运行"即可正常安装。

长期方案：购买代码签名证书（每年 500-2000 元）

---

## 四、发布流程

### 4.1 提交并推送源码到 GitHub

```bash
cd F:\fzz-Project\sensend\sensend
git add .
git commit -m "feat(v0.x.0): 更新说明"
git push origin main
```

### 4.2 打 tag 并推送

```bash
git tag v0.x.0
git push origin v0.x.0
```

### 4.3 创建 Release 并上传安装包

**方式一：gh CLI（推荐）**

```bash
gh release create v0.x.0 "src-tauri\target\release\bundle\nsis\Sensend_0.x.0_x64-setup.exe" `
  --title "Sensend v0.x.0" `
  --notes "更新内容说明"
```

**方式二：网页创建**
1. 打开 https://github.com/jloft198479-cyber/sensend/releases/new
2. 选择刚推送的 tag
3. 填写标题和描述
4. 拖入安装包附件
5. 点击"Publish release"

### 4.4 Release 描述模板

```markdown
**Sensend v0.x.0**

**更新内容**
- 更新点 1
- 更新点 2

**平台支持**
- Notion：支持数据库和页面
- FlowUs：支持多维表和页面
- 飞书：支持追加文档
- 本地：创建 .md 文件

**系统要求**
- Windows 10/11 x64

**安装说明**
- 下载 `Sensend_0.x.0_x64-setup.exe` 双击安装
- Windows 可能提示"无法识别的应用"，点击"更多信息" → "仍要运行"

**致谢**
送给儿子小柏
```

---

## 五、常用命令速查

```bash
# 开发
npm run tauri dev

# 构建（需已加载 MSVC 环境，或用 scripts/build-release.ps1）
npm run tauri build

# 仅构建前端
npm run build

# 仅构建后端（须先加载 MSVC 环境，或用 build-release.ps1）
cd src-tauri && cargo build --release

# 查看 Git 状态
git status

# 推送到远程
git push origin main

# 创建标签并推送
git tag v0.x.0
git push origin v0.x.0

# 创建 Release 并上传安装包（gh CLI）
gh release create v0.x.0 "src-tauri\target\release\bundle\nsis\Sensend_0.x.0_x64-setup.exe" --title "Sensend v0.x.0" --notes "更新说明"
```

---

## 六、问题排查清单

| 问题 | 检查项 | 解决方案 |
|------|--------|----------|
| 构建失败 | NSIS 目录结构 | 检查 `%LOCALAPPDATA%\tauri\NSIS\` |
| 构建失败 | 插件版本 | 确认 nsis_tauri_utils.dll 为 v0.5.3 |
| 构建失败 | MSVC 环境未加载 | 用 `scripts/build-release.ps1`，不要裸跑 `npm run tauri build` |
| 无图标 | tauri.conf.json | 添加 windows.nsis 配置 |
| Git 命令失败 | PATH 环境变量 | 使用完整路径或添加到 PATH |
| 安全警告 | 未签名 | Release 说明中提示用户点击"仍可运行" |

---

> 本手册基于 Sensend 打包发布经验整理，最近一次验证：v0.4.0（2026-08-18）