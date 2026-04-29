# Sensend 开发经验手册

> 作者：简乐  
> 更新时间：2026-04-29  
> 项目：Sensend v0.1.0

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

### 1.2 工作流程：双目录开发模式

由于沙箱环境无法直接访问主项目目录，采用双目录模式：

```
C:\Users\fzz198479\sensend-du\    ← 沙箱可访问的开发目录
         ↓ copy-to-sensend.bat
F:\sensend\                        ← 实际运行测试目录
```

**工作流程**：
1. 在 `sensend-du` 目录修改代码
2. 运行 `copy-to-sensend.bat` 复制到 `F:\sensend`
3. 在 `F:\sensend` 运行测试
4. 测试通过后提交发布

**复制脚本示例** (`copy-to-sensend.bat`)：
```batch
@echo off
xcopy /E /Y "C:\Users\fzz198479\sensend-du\src" "F:\sensend\src\"
xcopy /E /Y "C:\Users\fzz198479\sensend-du\src-tauri\src" "F:\sensend\src-tauri\src\"
echo Done
```

### 1.3 需求理解：修改前先确认

**踩坑案例**：

用户说"窗口高度不够"，我直接修改了主窗口高度，但用户实际指的是配置窗口。

**教训**：
- 用户提到"窗口"时，要确认是哪个窗口（主窗口/配置窗口/其他）
- 用户提到"高度/宽度"时，要确认具体数值和目标
- 修改前先复述需求，确认理解正确

**正确做法**：
```
用户：窗口高度不够
我：你说的是主窗口（420×210）还是配置窗口（420×580）？
用户：配置窗口
我：需要调整到多少？
用户：580px 吧
我：好的，我把配置窗口高度从 540px 改到 580px
```

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
| 引用 | quote | quote | 16 |
| 代码 | code | code | 17 |

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

**重构结果**：884 行 → 403 行

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

**正确做法**：
```rust
// 重构前：标记要保留的函数
// ✅ marks_to_text_elements - 保留，文本格式转换核心逻辑
// ✅ node_to_lark_block - 保留，节点转换核心逻辑
// ❌ create_folder - 删除，不再需要
// ❌ create_record - 删除，不再需要

// 重构时：只删除不需要的，保留核心逻辑
```

---

## 三、Tauri 打包

### 3.1 NSIS 环境配置

**问题**：Tauri 构建时报错找不到 NSIS 或插件。

**原因**：Tauri 需要特殊的 NSIS 目录结构，标准版 NSIS 不包含 Tauri 专用插件。

**解决方案**：

1. 创建目录结构：
```
%LOCALAPPDATA%\tauri\NSIS\
├── makensis.exe
├── Bin\
│   └── makensis.exe
├── Include\
│   ├── MUI2.nsh
│   ├── FileFunc.nsh
│   ├── x64.nsh
│   ├── nsDialogs.nsh
│   ├── WinMessages.nsh
│   └── Win\
│       ├── COM.nsh
│       ├── Propkey.nsh
│       └── RestartManager.nsh
├── Plugins\
│   └── x86-unicode\
│       └── additional\
│           └── nsis_tauri_utils.dll
└── Stubs\
    ├── lzma-x86-unicode
    └── lzma_solid-x86-unicode
```

2. 下载 NSIS 3.11：
```
https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
```

3. 解压到 `%LOCALAPPDATA%\tauri\NSIS\`

4. 下载 Tauri 插件：
```
https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll
```

5. 放到 `Plugins\x86-unicode\additional\` 目录

### 3.2 插件版本问题

**问题**：
```
Warn NSIS directory contains mis-hashed files. Redownloading them.
Downloading nsis_tauri_utils-v0.5.3...
failed to bundle project `io: Connection refused`
```

**原因**：Tauri 2.x 需要 `nsis_tauri_utils.dll` v0.5.3，我下载的是 v0.4.1。

**版本对照**：

| Tauri 版本 | nsis_tauri_utils 版本 |
|------------|----------------------|
| Tauri 1.x | v0.4.x |
| Tauri 2.x | v0.5.x |

**解决方案**：下载正确版本的插件，Tauri 会检查文件 hash，版本不对会自动尝试重新下载。

### 3.3 安装包图标配置

**问题**：生成的安装包显示默认图标，不是自定义 Logo。

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

**图标文件要求**：
- `icon.ico`：Windows 应用图标，建议 256×256 或更大
- `icon.icns`：macOS 应用图标
- `icon.png`：通用图标，建议 512×512

### 3.4 构建产物位置

构建完成后，产物位置：

| 类型 | 路径 |
|------|------|
| 可执行文件 | `src-tauri\target\release\sensend.exe` |
| NSIS 安装包 | `src-tauri\target\release\bundle\nsis\Sensend_0.1.0_x64-setup.exe` |
| MSI 安装包 | `src-tauri\target\release\bundle\msi\Sensend_0.1.0_x64.msi` |

**文件大小参考**：
- 可执行文件：约 14MB
- NSIS 安装包：约 3.5MB

---

## 四、发布与分发

### 4.1 Gitee 发布流程

**步骤一：推送源码**

```bash
# 初始化仓库
git init

# 添加文件
git add .

# 提交
git commit -m "v0.1.0 release"

# 添加远程仓库
git remote add origin https://gitee.com/用户名/项目名.git

# 推送
git push -u origin master
```

**步骤二：创建 Release**

方式一：网页创建
1. 打开仓库页面
2. 点击"发行版" → "+ 创建发行版"
3. 填写版本号、标题、描述
4. 上传安装包附件
5. 点击"创建发行版"

方式二：API 创建
```bash
curl -X POST "https://gitee.com/api/v5/repos/用户名/项目名/releases" \
  -d "access_token=你的令牌" \
  -d "tag_name=v0.1.0" \
  -d "name=Sensend v0.1.0" \
  -d "target_commitish=master" \
  -d "body=首个正式发布版本"
```

**步骤三：上传附件**

方式一：命令行工具
```bash
npm install -g gitee-release-cli
gitee-release config accessToken 你的令牌
gitee-release assets upload /path/to/file.exe --target 0.1.0
```

方式二：手动上传
1. 打开 Release 页面
2. 点击"编辑"
3. 拖入安装包文件
4. 点击"更新发行版"

### 4.2 API 上传限制与替代方案

**问题**：使用 API 上传附件返回 404。

```bash
curl -X POST "https://gitee.com/api/v5/repos/用户名/项目名/releases/xxx/assets" \
  -F "access_token=xxx" \
  -F "file=@installer.exe"
# 返回：你所访问的页面不存在 (404)
```

**可能原因**：
1. Token 权限不足
2. API 接口限制
3. 文件大小限制

**替代方案**：

| 方案 | 优点 | 缺点 |
|------|------|------|
| gitee-release-cli | 官方工具，功能完整 | 需要额外安装 |
| 手动上传 | 简单可靠 | 需要人工操作 |
| GitHub Release | API 完善 | 国内访问慢 |

### 4.3 代码签名问题

**问题**：下载安装包时浏览器提示"无法识别的应用"。

**原因**：安装包未购买代码签名证书。

**解决方案对比**：

| 方案 | 成本 | 效果 |
|------|------|------|
| 购买证书 | 500-2000 元/年 | 无警告 |
| 免费签名 | 免费，配置复杂 | 需要特定环境 |
| 提示用户 | 免费 | 用户体验差 |

**临时方案**：在 Release 说明中添加提示

```markdown
> Windows 可能提示"无法识别的应用"，这是因为安装包未购买代码签名证书。
> 点击"更多信息" → "仍要运行"即可正常安装。
```

**免费签名方案**（仅限开源项目）：
- SignPath + Azure Key Vault
- 需要配置 Azure 账户
- 配置复杂，适合有经验的开发者

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

## 五、版本控制

### 5.1 Git 路径配置

**问题**：
```
git : The term 'git' is not recognized
```

**原因**：Git 安装在非标准路径（如 D:\Git），未添加到 PATH。

**解决方案**：

方式一：使用完整路径
```bash
D:\Git\bin\git.exe status
```

方式二：添加到 PATH（临时）
```powershell
$env:PATH += ";D:\Git\bin"
git status
```

方式三：添加到 PATH（永久）
1. 打开"系统属性" → "环境变量"
2. 在"Path"中添加 `D:\Git\bin`
3. 重启终端

**记录 Git 路径**：
- 将 Git 安装路径记录在项目文档中
- 避免每次都要重新查找

### 5.2 干净源码包制作

**目标**：创建一个不包含构建产物、依赖、临时文件的干净源码包。

**方法**：使用 robocopy 复制，排除不需要的目录和文件。

```powershell
# 删除旧目录
Remove-Item "C:\Users\fzz198479\sensend-release" -Recurse -Force

# 复制文件
robocopy "F:\sensend" "C:\Users\fzz198479\sensend-release" /E `
    /XD node_modules target dist .git .backup backup gen `
    /XF *.txt *.log *.bak HANDOFF.md icon-source.png test-icon.ico `
    /NFL /NDL /NJH /NJS
```

**参数说明**：
- `/E`：复制子目录，包括空目录
- `/XD`：排除目录
- `/XF`：排除文件
- `/NFL /NDL /NJH /NJS`：减少输出

### 5.3 排除文件清单

**必须排除**：

| 类型 | 目录/文件 | 原因 |
|------|-----------|------|
| 依赖 | node_modules/ | 可通过 npm install 恢复 |
| 构建产物 | target/, dist/ | 可通过构建命令生成 |
| 缓存 | .git/, gen/ | 版本控制缓存 |
| 临时文件 | *.txt, *.log, *.bak | 开发过程产生 |
| 备份 | .backup/, backup/ | 本地备份，不需要发布 |
| IDE 配置 | .vscode/, .idea/ | 个人开发环境配置 |

**必须包含**：

| 类型 | 文件 | 原因 |
|------|------|------|
| 配置 | package.json, Cargo.toml, tauri.conf.json | 项目配置 |
| 源码 | src/, src-tauri/src/ | 核心代码 |
| 图标 | src-tauri/icons/ | 应用图标 |
| 文档 | README.md, LICENSE | 项目说明 |
| 忽略规则 | .gitignore | 版本控制配置 |

---

## 六、工具使用

### 6.1 Git 完整路径调用

当 Git 不在 PATH 中时：

```powershell
# 查看版本
& "D:\Git\bin\git.exe" --version

# 克隆仓库
& "D:\Git\bin\git.exe" clone https://gitee.com/user/repo.git

# 查看状态
& "D:\Git\bin\git.exe" status

# 提交
& "D:\Git\bin\git.exe" add .
& "D:\Git\bin\git.exe" commit -m "message"
& "D:\Git\bin\git.exe" push
```

### 6.2 robocopy 文件复制

robocopy 是 Windows 内置的强大文件复制工具：

```powershell
# 基本用法
robocopy "源目录" "目标目录" /E

# 排除目录
robocopy "源" "目标" /E /XD dir1 dir2

# 排除文件
robocopy "源" "目标" /E /XF *.log *.tmp

# 镜像复制（删除目标中多余的文件）
robocopy "源" "目标" /MIR

# 仅复制新文件
robocopy "源" "目标" /XO
```

**常用参数**：

| 参数 | 说明 |
|------|------|
| /E | 复制子目录，包括空目录 |
| /S | 复制子目录，不包括空目录 |
| /MIR | 镜像复制 |
| /XD | 排除目录 |
| /XF | 排除文件 |
| /XO | 排除较旧的文件 |
| /NFL | 不记录文件名 |
| /NDL | 不记录目录名 |
| /NJH | 不显示作业头 |
| /NJS | 不显示作业摘要 |

### 6.3 PowerShell 常用命令

```powershell
# 文件操作
Get-ChildItem "路径"                    # 列出文件
Get-Content "文件路径"                  # 读取文件
Copy-Item "源" "目标"                   # 复制文件
Move-Item "源" "目标"                   # 移动文件
Remove-Item "路径" -Recurse -Force      # 删除文件/目录

# 目录操作
New-Item -ItemType Directory -Path "路径"  # 创建目录
Test-Path "路径"                           # 检查是否存在

# 进程操作
Get-Process                              # 查看进程
Stop-Process -Name "进程名"              # 停止进程

# 网络操作
Invoke-WebRequest -Uri "URL" -OutFile "文件"  # 下载文件
Invoke-RestMethod -Uri "URL" -Method POST     # API 请求

# 环境变量
$env:PATH                               # 查看 PATH
$env:PATH += ";新路径"                   # 添加到 PATH
```

---

## 七、问题排查

### 7.1 构建失败排查清单

| 错误信息 | 可能原因 | 解决方案 |
|----------|----------|----------|
| `NSIS directory contains mis-hashed files` | 插件版本不对 | 下载正确版本的 nsis_tauri_utils.dll |
| `failed to bundle project` | NSIS 配置不完整 | 检查目录结构是否完整 |
| `Connection refused` | 网络问题，无法下载依赖 | 手动下载并放到正确位置 |
| `git is not recognized` | Git 不在 PATH | 使用完整路径或添加到 PATH |
| `npm ERR!` | 依赖安装失败 | 删除 node_modules 重新安装 |

**排查步骤**：

1. 检查 NSIS 目录结构
```powershell
Get-ChildItem "$env:LOCALAPPDATA\tauri\NSIS" -Recurse
```

2. 检查插件是否存在
```powershell
Test-Path "$env:LOCALAPPDATA\tauri\NSIS\Plugins\x86-unicode\additional\nsis_tauri_utils.dll"
```

3. 检查 Rust 环境
```bash
rustc --version
cargo --version
```

4. 检查 Node.js 环境
```bash
node --version
npm --version
```

### 7.2 常见错误信息解读

**错误 1**：
```
error: linking with `link.exe` failed
```

**原因**：缺少 Visual Studio Build Tools 或 Windows SDK。

**解决方案**：安装 Visual Studio Build Tools，选择"使用 C++ 的桌面开发"。

---

**错误 2**：
```
error: failed to run custom build command for `openssl-sys`
```

**原因**：缺少 OpenSSL。

**解决方案**：
- Windows：安装 OpenSSL for Windows
- 或使用 `cargo install openssl` 安装

---

**错误 3**：
```
npm ERR! EACCES: permission denied
```

**原因**：权限不足。

**解决方案**：
- Windows：以管理员身份运行
- Linux/macOS：使用 `sudo npm install`

---

**错误 4**：
```
Error: WebView2 not found
```

**原因**：缺少 WebView2 运行时。

**解决方案**：
- Windows 10/11 通常已内置
- 手动下载安装：https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

## 八、附录

### 8.1 项目路径记录

| 路径 | 用途 |
|------|------|
| `F:\sensend` | 主项目目录 |
| `C:\Users\fzz198479\sensend-du` | 沙箱开发目录 |
| `C:\Users\fzz198479\sensend-release` | 干净发布目录 |
| `D:\Git\bin\git.exe` | Git 安装路径 |
| `%LOCALAPPDATA%\tauri\NSIS\` | Tauri NSIS 缓存 |

### 8.2 下载链接汇总

| 资源 | 链接 |
|------|------|
| NSIS 3.11 | https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip |
| nsis_tauri_utils v0.5.3 | https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll |
| Node.js | https://nodejs.org |
| Rust | https://rustup.rs |
| Git | https://git-scm.com |
| Visual Studio Build Tools | https://visualstudio.microsoft.com/visual-cpp-build-tools/ |

### 8.3 常用命令速查表

```bash
# === 开发 ===
npm install                    # 安装依赖
npm run tauri dev              # 开发模式运行
npm run build                  # 构建前端
npm run tauri build            # 构建发布版本

# === Rust ===
cargo build                    # 调试构建
cargo build --release          # 发布构建
cargo check                    # 快速检查
cargo clippy                   # 代码检查

# === Git ===
git status                     # 查看状态
git add .                      # 添加所有文件
git commit -m "message"        # 提交
git push                       # 推送
git pull                       # 拉取
git tag v0.1.0                 # 创建标签
git push --tags                # 推送标签

# === 清理 ===
rm -rf node_modules            # 删除依赖
rm -rf src-tauri/target        # 删除构建产物
npm cache clean --force        # 清理 npm 缓存
cargo clean                    # 清理 cargo 缓存
```

---

> 本手册基于 Sensend v0.1.0 开发经验整理  
> 记录人：简乐  
> 致谢：送给儿子小柏