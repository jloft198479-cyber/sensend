# Sensend 打包与发布手册

> 作者：简乐
> 更新时间：2026-08-19（修复 PATH 大小写变体覆盖 bug（智能体工具环境必踩）；编译命令统一指向项目脚本，清理过时内容；BUILD-ENV.md 已并入本文件后删除）

---

## 〇、构建环境速查（智能体必读）

**铁律（违反必挂）**：
- ✅ **想编译、测试、打包，一律走项目脚本**（每个脚本都会自动加载 M 盘 Rust + MSVC 环境）：
  - 编译并启动应用窗口 → `scripts\run-app.ps1`（双击 `scripts\run-app.bat` 亦可）
  - 跑测试 → `scripts\run-tests.ps1`（双击 `scripts\run-tests.bat` 亦可）
  - 打包安装包（发布用）→ `scripts\build-release.ps1`
  ```powershell
  cd F:\fzz-Project\sensend\sensend
  powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
  ```
- ❌ **禁止**在普通终端直接执行 `npm run tauri build`、`cargo build`、`npm run tauri dev`——M 盘 Rust / MSVC 不在系统 PATH，裸跑必报"找不到 cargo / cl.exe"。

**原因（一句话）**：Rust（`M:\rust`）和 VS Build Tools（`M:\VS\BuildTools`）都**不在系统 PATH**，普通终端找不到 `cargo` / `cl.exe`。只有脚本会设置 `CARGO_HOME` / `RUSTUP_HOME` 并调用 `vcvars64.bat` 加载 MSVC 环境（INCLUDE/LIB）。

> **环境本身完好，报错 ≠ 环境坏，无需重装、无需修复。**

**环境现状（已核验 2026-08-17）**：

| 项 | 值 |
|---|---|
| 工具链 | `stable-x86_64-pc-windows-msvc`（rustc / cargo 1.96.0） |
| MSVC | 14.44.35207（`cl.exe` 就位） |
| NSIS | `makensis.exe` + `nsis_tauri_utils.dll` v0.5.3 就位 |
| cargo 源 | 中科大 sparse 镜像 |

**调试后端（单独跑 cargo / 测试）**：直接用测试脚本，它已加载双环境：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\run-tests.ps1"
```

> 只想验证"Rust 代码能编译"而不启动窗口、不打包 → 也用 run-tests.ps1（`cargo test` 会完整编译一遍，不杀进程、不弹窗）。

**智能体执行注意（防再翻车）**：
- **直接执行脚本文件本身**（如上命令），不要把环境加载步骤拆开在工具终端里逐条跑——智能体工具的终端有调用限制，且 PATH 隔离，逐条跑必失败。
- 构建耗时：增量约 1~2 分钟，全量 5~15 分钟。工具调用时设长超时或后台执行，勿因等待而误判失败。
- `run-app.ps1` 会**强杀运行中的 sensend 并在桌面弹出新窗口**——用户正在使用时别跑它；仅验证编译用 run-tests.ps1。
- exe 被运行中进程占用时 cargo 报 `os error 5 拒绝访问`（sensend 点关闭只是隐藏，进程还活着）。手动构建前先托盘退出应用；run-app.ps1 会自动杀进程。
- 从 Git Bash 调用：先 `cd "F:/fzz-Project/sensend/sensend"`，脚本路径用正斜杠 `scripts/run-app.ps1` 亦可。

**非要手动加载环境**（不推荐，易漏步骤，报错别怪环境）：

> ⚠️ 坑（2026-08-19 实测踩过）：智能体工具宿主进程的环境块里可能同时存在 `Path` / `PATH` / `path` 多个大小写变体，`cmd set` 会把它们**全部**输出。逐行回写时三者互相覆盖，最后写入的旧值会把 vcvars 设置的 MSVC 路径冲掉 → `link.exe was not found`。三个项目脚本已内置修复，手动加载必须照抄下面写法（循环内跳过 PATH，单独取含 Hostx64 的那份）：

```powershell
$env:CARGO_HOME = "M:\rust\.cargo"
$env:RUSTUP_HOME = "M:\rust\.rustup"
$env:PATH = "M:\rust\.cargo\bin;$env:PATH"
$raw = cmd /c "`"M:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat`" >nul 2>&1 && set"
foreach ($line in $raw) {
  if ($line -match "^([^=]+)=(.*)$") {
    if ($matches[1] -match "^(?i)path$") { continue }   # PATH 大小写变体防互相覆盖
    Set-Item -Path ("env:" + $matches[1]) -Value $matches[2]
  }
}
$vcPath = @($raw | Where-Object { $_ -match "^(?i)path=.*Hostx64" }) | Select-Object -First 1
if ($vcPath) { $env:PATH = ($vcPath -replace "^(?i)path=", "") }   # 只取含 MSVC 的完整 PATH
$env:PATH = "M:\rust\.cargo\bin;$env:PATH"   # 最后前置 cargo
# 验证：Get-Command link.exe 应指向 M:\VS\...\Hostx64\x64\link.exe，然后 cargo 命令在 src-tauri 目录跑
```

---

## 一、环境准备

### 1.1 必需软件

| 软件 | 版本 | 用途 | 下载地址 |
|------|------|------|----------|
| Node.js | 18+（本机 24.16.0） | 前端构建 | https://nodejs.org |
| Rust | 1.96.0（已装 M 盘 `M:\rust`） | 后端编译 | 走脚本自动加载，无需手动装 |
| Git | 2.x | 版本控制 | https://git-scm.com |
| Tauri CLI | 2.x（项目已内置） | 打包工具 | `package.json` 已含 `@tauri-apps/cli`，**无需全局安装** |

### 1.2 Tauri 打包依赖

Windows 平台打包 NSIS 安装包需要额外下载：

| 组件 | 下载地址 | 存放位置 |
|------|----------|----------|
| NSIS 3.11 | https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip | `%LOCALAPPDATA%\tauri\NSIS\` |
| nsis_tauri_utils.dll v0.5.3 | https://github.com/tauri-apps/nsis-tauri-utils/releases/download/nsis_tauri_utils-v0.5.3/nsis_tauri_utils.dll | `%LOCALAPPDATA%\tauri\NSIS\Plugins\x86-unicode\additional\` |

---

## 二、打包步骤

### 2.1 安装依赖

node / npm 在系统 PATH，此步可在普通终端执行：

```bash
cd F:\fzz-Project\sensend\sensend
npm install
```

> 注意：之后**所有涉及编译的命令**（tauri dev / tauri build / cargo）都必须在 M 盘环境已加载的脚本里跑，见 §2.2 / §2.3。

### 2.2 编译运行（日常开发最常用）

**编译并启动应用窗口**（不打包安装包，改完代码最快看到效果）：

- 双击 `scripts\run-app.bat`，或执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\run-app.ps1"
```

脚本会自动：加载 M 盘 Rust + MSVC 环境 → 构建前端 → `npm run tauri build -- --no-bundle` 编译后端 → 启动 `src-tauri\target\release\sensend.exe`（若旧进程占用 exe，会自动杀掉再编译，避免"拒绝访问 os error 5"）。

> **不要**在普通终端裸跑 `npm run tauri dev`——`tauri dev` 需要 cargo，而 cargo 不在系统 PATH，必然报错。

### 2.3 构建发布（打包安装包）

打包安装包（NSIS setup.exe）**只有一种正确方式**——走脚本，它会自动加载 M 盘 Rust + MSVC 环境：

```bash
cd F:\fzz-Project\sensend\sensend
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
```

脚本做三件事：
1. 设置 `CARGO_HOME` / `RUSTUP_HOME` 指向 `M:\rust\.cargo` / `M:\rust\.rustup`
2. 调用 `M:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat` 加载 MSVC 环境（`cl.exe` / `link.exe` / INCLUDE / LIB 等）
3. 执行 `npm run tauri build`

**为什么需要加载 MSVC 环境？** Rust 默认用 `stable-x86_64-pc-windows-msvc` toolchain，依赖 `ring`、`reqwest` 等 crate 需要 C 编译器（`cl.exe`）。VS Build Tools 装好后，`cl.exe` 在 `M:\VS\BuildTools\VC\Tools\MSVC\<版本>\bin\HostX64\x64\` 下，但它的 INCLUDE/LIB 环境变量必须通过 `vcvars64.bat` 加载，裸跑 cargo 找不到。

> ⚠️ 不要尝试"手动构建"：除非你已在 **M 盘 Rust 环境（CARGO_HOME / RUSTUP_HOME / PATH）+ MSVC 环境（vcvars64.bat）同时加载**的终端里，否则裸跑 `npm run tauri build` 必然失败。直接走脚本，别纠结。

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

## 四、发布流程（傻瓜式全流程）

> 目标：任何智能体照抄本流程即可完成「推代码 → 打 tag → 传安装包 → 发 Release」，全程可复制、可验证、可回滚。
> 铁律：**顺序不可乱**——先构建成功，再打 tag、推送、发 Release。任何一步失败立即停止排查，不要带病往下走。

### 4.0 发布前自检（5 项，全绿才继续）

在项目根目录 `F:\fzz-Project\sensend\sensend` 依次执行，任一失败即停止：

```powershell
# ① 版本号：记下它，后面 tag / Release 标题 / 安装包名全部要跟它一致
npm pkg get version            # 期望输出如 "0.4.0"

# ② gh 已登录（发布必须走 gh CLI，别用网页手动，网页无法验证）
gh auth status                 # 期望：Logged in to github.com account xxx

# ③ git 凭据可用（能读到远端即证明 push 通道通畅）
git ls-remote --heads origin   # 期望输出 refs/heads/main

# ④ 工作区干净（只有你自己要提交的改动；出现 ?? 未跟踪目录要逐个确认，严禁 git add . 盲加）
git status

# ⑤ 本地 tag 是否已存在该版本（存在则说明发过，先 git tag 确认，不要覆盖）
git tag | Select-String "^v"
```

### 4.1 提交代码

> 编码提示：commit message 建议英文为主；若写中文且终端出现乱码，先执行 `chcp 65001` 再提交。提交前**必须**用 `git status` 核对暂存内容，明确列出要提交的文件（`git add <路径>`），不要 `git add .` 盲加。

```powershell
cd F:\fzz-Project\sensend\sensend
git add <本次改动的文件/目录>        # 明确列出，不用 git add .
git commit -m "feat(v0.4.0): 本次发布内容说明"
git log --oneline -1                 # 验证提交成功
```

### 4.2 构建安装包（必须先于推送）

> 必须走打包脚本加载 M 盘 Rust + MSVC 环境，**禁止**裸跑 `npm run tauri build`（见 §〇）。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
```

构建成功后校验产物存在（路径里的版本号替换成 4.0 记下的）：

```powershell
Test-Path "src-tauri\target\release\bundle\nsis\Sensend_0.4.0_x64-setup.exe"   # 期望 True
```

### 4.3 打 tag 并推送

> tag 名必须与版本号一致（`v0.4.0`），与 package.json 的 `0.4.0` 对应。推送 = main + tag 两个都推。

```powershell
git tag v0.4.0                         # 若提示已存在：说明发过，停下来核对版本号
git push origin main
git push origin v0.4.0
git ls-remote --tags origin | Select-String "v0.4.0"   # 验证 tag 已上远端
```

### 4.4 创建 GitHub Release

> 用 `--notes-file` 读文件方式写说明，彻底避开 PowerShell 里中文 `--notes` 的转义乱码。说明文件用完即删。

```powershell
# ① 写 Release 说明（内容见 4.5 模板），建议用英文文件名避免编码问题
#    在项目根目录创建 release-notes.md（完成后删除）

# ② 创建 Release（asset 用 4.2 校验过存在的安装包路径）
gh release create v0.4.0 "src-tauri\target\release\bundle\nsis\Sensend_0.4.0_x64-setup.exe" `
  --title "Sensend v0.4.0" `
  --notes-file release-notes.md

# ③ 清理临时说明文件
Remove-Item release-notes.md
```

> 若 `gh release create` 报「already exists」：该 tag 已有 Release，通常是上次半途而废，先 `gh release view v0.4.0` 看状态再决定补传还是清理。

### 4.5 Release 描述模板

```markdown
**Sensend v0.4.0**

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
- 下载 `Sensend_0.4.0_x64-setup.exe` 双击安装
- Windows 可能提示"无法识别的应用"，点击"更多信息" → "仍要运行"

**致谢**
送给儿子小柏
```

### 4.6 发布后验证（必须做，缺一不算发完）

```powershell
gh release view v0.4.0                # 期望看到标题、tag、asset 列表含 setup.exe
git ls-remote --tags origin | Select-String "v0.4.0"   # 远端 tag 存在
git log origin/main --oneline -1      # main 已含最新提交
```

### 4.7 发布故障速查表

| 现象 | 原因 | 处理 |
|------|------|------|
| `gh` 命令不存在 | gh CLI 未装 | 安装 GitHub CLI 后 `gh auth login` |
| `gh auth status` 报未登录 | 登录态丢失 | `gh auth login`（用浏览器授权）后重跑自检 |
| `git push` 报认证失败 / 403 | 凭据过期 | 先 `gh auth status` 确认登录；凭据由 gh 管理，重登录即可 |
| `git push` 报 non-fast-forward | 远端有新提交（他人/上次半途的） | `git pull --rebase origin main` 后重新 push，**禁止 force push** |
| `git tag vX` 报已存在 | 该版本发过 | `git tag` 看清单确认，不要覆盖；要改说明用 `gh release edit` |
| `build-release.ps1` 报找不到 cl.exe / cargo | M 盘路径变了或未按脚本走 | 确认 `Test-Path M:\rust`、`Test-Path M:\VS\BuildTools`；必须走脚本，禁止裸跑 |
| `gh release create` 报 asset 不存在 | 安装包路径/版本号写错 | 回 4.2 用 `Test-Path` 核对实际文件名 |
| Release 说明出现乱码 | PowerShell 中文转义问题 | 改用 `--notes-file`（4.4 推荐路径） |
| `git add .` 误加了多余目录 | `.gitignore` 未覆盖该目录 | `git rm -r --cached <目录>` 撤出暂存，改逐文件 add |

### 4.8 回滚

- **代码已推、Release 未发**：`git revert` 或提交修复，正常再发下一版。
- **Release 发错了**：`gh release delete v0.4.0 --cleanup-tag`（会同时删远端 tag），本地 `git tag -d v0.4.0` 后重来。
- 回滚仅限发错的当次，历史发布（v0.1.0~v0.3.0）一律不动。

---

## 五、常用命令速查

```bash
# 编译并启动应用窗口（推荐，自动加载 M 盘 Rust + MSVC 环境）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-app.ps1

# 跑测试（推荐，自动加载环境）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-tests.ps1

# 打包安装包（发布用，自动加载环境）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-release.ps1

# 仅构建前端（只走 vite/node，普通终端可跑；不编译后端、不生成 exe）
npm run build

# 查看 Git 状态
git status

# 推送到远程
git push origin main

# 创建标签并推送（vX.Y.Z 替换为实际版本号）
git tag v0.4.0
git push origin v0.4.0

# 创建 Release 并上传安装包（gh CLI，完整流程见 §四）
# 注意：说明文字用 --notes-file 读文件，避免中文转义乱码
gh release create v0.4.0 "src-tauri\target\release\bundle\nsis\Sensend_0.4.0_x64-setup.exe" --title "Sensend v0.4.0" --notes-file release-notes.md
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

> 本手册基于 Sensend 打包发布经验整理，最近一次验证：v0.4.0（2026-08-18）；环境加载链路修复验证：run-tests.ps1 75 测试全过（2026-08-19）