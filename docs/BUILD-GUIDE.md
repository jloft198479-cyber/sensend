# Sensend 打包与发布手册

> 作者：简乐  
> 更新时间：2026-04-29

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
cd F:\sensend
npm install
```

### 2.2 开发测试

```bash
npm run tauri dev
```

### 2.3 构建发布

```bash
npm run tauri build
```

构建产物位置：
- 便携版：`src-tauri\target\release\sensend.exe`
- 安装包：`src-tauri\target\release\bundle\nsis\Sensend_0.1.0_x64-setup.exe`

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

### 3.5 Gitee API 上传附件失败

**现象**：
```
你所访问的页面不存在 (404)
```

**原因**：Gitee API 上传 Release 附件可能需要特殊权限或 Token 配置。

**解决方案**：

方案一：使用官方命令行工具
```bash
npm install -g gitee-release-cli
gitee-release config accessToken 你的令牌
gitee-release assets upload /path/to/file.exe --target 0.1.0
```

方案二：手动上传
1. 打开 https://gitee.com/用户名/项目名/releases
2. 点击对应版本"编辑"
3. 拖入安装包文件
4. 点击"更新发行版"

### 3.6 Windows 安全警告

**现象**：下载安装包时浏览器提示"无法识别的应用"。

**原因**：安装包未购买代码签名证书。

**解决方案**：

临时方案：在 Release 说明中添加提示
> Windows 可能提示"无法识别的应用"，这是因为安装包未购买代码签名证书。点击"更多信息" → "仍要运行"即可正常安装。

长期方案：购买代码签名证书（每年 500-2000 元）

---

## 四、发布流程

### 4.1 创建干净源码包

```powershell
# 创建目录
mkdir C:\Users\fzz198479\sensend-release

# 复制文件（排除构建产物）
robocopy F:\sensend C:\Users\fzz198479\sensend-release /E `
    /XD node_modules target dist .git .backup backup gen `
    /XF *.txt *.log *.bak HANDOFF.md sensend-principles.md `
    /NFL /NDL /NJH /NJS
```

### 4.2 推送源码到 Gitee

```bash
cd F:\sensend
D:\Git\bin\git.exe init
D:\Git\bin\git.exe add .
D:\Git\bin\git.exe commit -m "v0.1.0 release"
D:\Git\bin\git.exe remote add origin https://gitee.com/用户名/项目名.git
D:\Git\bin\git.exe push -u origin master
```

### 4.3 创建 Release

**方式一：网页创建**
1. 打开仓库页面 → "发行版" → "+ 创建发行版"
2. 填写版本号、标题、描述
3. 上传安装包附件
4. 点击"创建发行版"

**方式二：API 创建**
```bash
curl -X POST "https://gitee.com/api/v5/repos/用户名/项目名/releases" \
  -d "access_token=你的令牌" \
  -d "tag_name=v0.1.0" \
  -d "name=Sensend v0.1.0" \
  -d "target_commitish=master"
```

### 4.4 Release 描述模板

```markdown
**Sensend v0.1.0**

首个正式发布版本

**功能特性**
- 悬浮记事本，一键发送到 Notion / FlowUs / 飞书 / 本地
- 全局快捷键快速唤起
- 极简、轻盈、优雅

**平台支持**
- Notion：支持数据库和页面
- FlowUs：支持多维表和页面
- 飞书：支持追加文档
- 本地：创建 .md 文件

**系统要求**
- Windows 10/11 x64

**安装说明**
- 下载 `Sensend_0.1.0_x64-setup.exe` 双击安装
- Windows 可能提示"无法识别的应用"，点击"更多信息" → "仍要运行"

**致谢**
送给儿子小柏
```

---

## 五、文件清单

### 5.1 必需文件

```
项目根目录/
├── .gitignore
├── index.html
├── LICENSE
├── package.json
├── package-lock.json
├── README.md
├── tsconfig.json
├── tsconfig.node.json
├── vite.config.ts
├── src/                    # 前端源码
└── src-tauri/              # 后端源码
    ├── Cargo.toml
    ├── Cargo.lock
    ├── tauri.conf.json
    ├── build.rs
    ├── capabilities/
    ├── icons/
    └── src/
```

### 5.2 排除文件

| 类型 | 示例 |
|------|------|
| 依赖目录 | node_modules/, target/ |
| 构建产物 | dist/, *.exe |
| 缓存文件 | .git/, gen/ |
| 临时文件 | *.txt, *.log, *.bak |
| 备份目录 | .backup/, backup/ |
| 开发笔记 | HANDOFF.md |

---

## 六、常用命令速查

```bash
# 开发
npm run tauri dev

# 构建
npm run tauri build

# 仅构建前端
npm run build

# 仅构建后端
cd src-tauri && cargo build --release

# 查看 Git 状态
git status

# 推送到远程
git push origin master

# 创建标签
git tag v0.1.0
git push origin v0.1.0
```

---

## 七、问题排查清单

| 问题 | 检查项 | 解决方案 |
|------|--------|----------|
| 构建失败 | NSIS 目录结构 | 检查 `%LOCALAPPDATA%\tauri\NSIS\` |
| 构建失败 | 插件版本 | 确认 nsis_tauri_utils.dll 为 v0.5.3 |
| 无图标 | tauri.conf.json | 添加 windows.nsis 配置 |
| Git 命令失败 | PATH 环境变量 | 使用完整路径或添加到 PATH |
| 上传失败 | Token 权限 | 检查 Token 是否有 repo 权限 |
| API 404 | 接口路径 | 确认 API 版本为 v5 |

---

> 本手册基于 Sensend v0.1.0 打包发布经验整理