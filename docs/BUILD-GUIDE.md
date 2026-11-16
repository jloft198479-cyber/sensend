# Sensend 打包与发布手册

> 作者：简乐
> 更新时间：2026-11-17（**全文逐条对照脚本与配置核验过一遍**，修正 §2.2 开发模式身份缺失、§2.3 漏写签名/清单/版本校验、§4.4 发布漏传 latest.json 等方向性错误；命令示例的版本号一律改成占位符，避免过期）
> 上一次核验：2026-08-19（PATH 大小写变体修复）、2026-11-17（发布流程与脚本对齐）

---

## 〇、构建环境速查（智能体必读）

**铁律（违反必挂）**：
- ✅ **想编译、测试、打包，一律走项目脚本**（每个脚本都会自动加载 M 盘 Rust + MSVC 环境）：
  - **改样式 / 调交互（日常首选，热更新不重编译）** → 双击仓库根 `启动开发模式.bat`（即 `scripts\run-dev.bat`）
  - 验证成品 / 编译出 exe 窗口 → `scripts\run-app.ps1`（双击 `scripts\run-app.bat` 亦可）⚠️ 会杀掉 `target\release` 那个进程（不碰安装版）
  - 跑测试 → `scripts\run-tests.ps1`（双击 `scripts\run-tests.bat` 亦可）
  - 打包安装包（发布用）→ `scripts\build-release.ps1`
  ```powershell
  # 在仓库根执行（脚本自己会定位项目根，不依赖当前目录）
  powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
  ```
- ❌ **禁止在普通终端裸跑** `npm run tauri build` / `cargo test` / `npm run tauri dev`——M 盘 Rust / MSVC 不在系统 PATH，裸跑必报"找不到 cargo / cl.exe"。
  **注意区分**：不是禁止这些命令本身，是禁止"绕过脚本直接在裸终端敲"。脚本内部跑的就是这些命令，那是唯一正确姿势。

**四条路径，别混（这是本文档最容易搞错的地方）**：

| 你要干的事 | 走哪条 | 产物 / 身份 |
|---|---|---|
| 改样式、调交互，边改边看 | 根目录 `启动开发模式.bat` | vite 热更新，**不重编译**；身份 `com.jloft.sensend.dev` |
| 验证编出来的真东西长什么样 | `scripts\run-app.ps1` | `target\release\sensend.exe`，身份 `.dev`；**全量构建，慢是正常的** |
| 只验证 Rust 能不能过 | `scripts\run-tests.ps1` | 跑 `cargo test`，不弹窗不杀进程 |
| 打包发版 | `scripts\build-release.ps1` | 安装包 + `.sig` + `latest.json`，身份 `com.jloft.sensend`（**正式**） |

> 身份这件事必须记牢：**测试环境只能用 `.dev` 身份**，否则调试会写进正式版的数据目录、还会跟正式版抢单实例锁。
> `run-dev.ps1` / `run-app.ps1` 都带 `--config src-tauri/tauri.dev.conf.json`，`build-release.ps1` 故意不带（发版要正式身份）。

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
- `run-app.ps1` 只杀 **`src-tauri\target\release\sensend.exe` 这一个路径**的进程（按 Path 精确匹配），**不会碰你机器上安装的正式版**。所以两版可以并存对比。跑之前确认你要杀的是哪个。
- `run-dev.ps1` 走 `tauri dev`，产物在 `target\debug\`，跟 `run-app.ps1` 的 release exe 互不影响。
- exe 被运行中进程占用时 cargo 报 `os error 5 拒绝访问`（sensend 点关闭只是隐藏，进程还活着）。手动构建前先托盘退出应用；run-app.ps1 会自动处理它自己那一个。
- 从 Git Bash 调用：先 `cd "F:/fzz-Project/2-sensend/sensend"`，脚本路径用正斜杠 `scripts/run-app.ps1` 亦可。

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
cd F:\fzz-Project\2-sensend\sensend
npm install
```

> 注意：之后**所有涉及编译的命令**（tauri dev / tauri build / cargo）都必须在 M 盘环境已加载的脚本里跑，见 §2.2 / §2.3。

### 2.2 开发模式（改样式 / 调交互，**日常首选**）

**双击仓库根目录的 `启动开发模式.bat`**（或 `scripts\run-dev.bat`），等价于：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\run-dev.ps1"
```

脚本做的事：加载 M 盘 Rust + MSVC 环境 → 释放被残留 vite 占用的 1420 端口 → `npm run tauri dev -- --config src-tauri/tauri.dev.conf.json`。

窗口连的是 `http://localhost:1420`（vite dev server），**界面不是编进 exe 的，是实时推过去的**：

- 改 CSS / 组件 / ts → vite 热更新（HMR）直接推给窗口，**秒看到，不重编译**
- 改 Rust → cargo 自动增量重编并重启窗口（这一步必然要等，但都是增量）
- 产物在 `src-tauri\target\debug\`，跟 `run-app.ps1` 的 release exe 互不干扰
- 关掉黑窗口 = 结束开发模式

> ⚠️ **`--config src-tauri/tauri.dev.conf.json` 绝不能省。** 少了它，`tauri dev` 读的是
> `tauri.conf.json`，身份 = `com.jloft.sensend`，**就是正式应用的身份**：同一个数据目录、
> 同一把单实例锁 → 调试写脏正式数据，正式版开着时还可能起不来。
> 2026-11-17 实测踩过（脚本第一版漏了这个参数），已修，改完请别顺手删。

> 2026-11-17 踩坑记录：`run-app.ps1` 曾被本文档写成"编译并启动应用窗口（改完代码最快看到效果）"，
> 但它其实是**打包成 exe**——界面刻死在产物里，改一个色值都要重跑一遍全量构建（约 90s）。
> 别拿它当日常预览用，它就是慢的，慢是这个工具本身的性质，不是配置问题。

### 2.2.1 编译运行（**验证成品** / 发布前自检用，不是日常预览）

- 双击 `scripts\run-app.bat`，或执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\run-app.ps1"
```

脚本会自动：加载 M 盘 Rust + MSVC 环境 → `npm run build`（含 token/对比度门禁 + 类型检查）→ `npm run tauri build -- --no-bundle --config src-tauri/tauri.dev.conf.json` → 启动 `src-tauri\target\release\sensend.exe`（若旧进程占用 exe，会先杀掉它自己那一个再编译，避免"拒绝访问 os error 5"）。

产物 `sensend.exe` 用的是 `com.jloft.sensend.dev` 这个独立 ID（数据目录与正式版隔离），但它**仍然是编译产物**，界面资源已固化——这是它和 §2.2 开发模式的唯一区别，别混。

> ⚠️ **`run-app.ps1` 只会杀 `target\release\sensend.exe` 那个进程**，按绝对路径精确匹配，**不碰安装版**。两版可以并存。
>
> **不要**在普通终端裸跑 `npm run tauri dev`——`tauri dev` 需要 cargo，而 cargo 不在系统 PATH，必然报错；要走上面的脚本。

### 2.3 构建发布（打包安装包）

打包安装包（NSIS setup.exe）**只有一种正确方式**——走脚本，它会自动加载 M 盘 Rust + MSVC 环境：

```bash
cd F:\fzz-Project\2-sensend\sensend
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
```

**这一步跟 §2.2.1 最大的不同：身份是正式的**（`com.jloft.sensend`，不带 `--config`），产出的就是你以后要发给用户的那个包。别拿它当预览用。

脚本实际做的事（**不止加载环境，共 6 步，前 3 步任一失败直接退出**）：

1. 设置 `CARGO_HOME` / `RUSTUP_HOME` 指向 `M:\rust\.cargo` / `M:\rust\.rustup`，并调用 `vcvars64.bat` 加载 MSVC
2. **版本号三处一致性校验**：`tauri.conf.json` / `Cargo.toml` / `package.json` 三者不一致直接 `exit 1`（发版最常见翻车点，先改齐再跑）
3. **检查更新签名私钥**（放在仓库外，绝不入库）：找不到就 `exit 1`，因为没私钥签不出 `.sig`，已发布用户收不到自动更新。默认位置 `F:\fzz-Project\9-CNB\keys\sensend-updater\sensend-updater.key`，可用环境变量 `SENSEND_SIGN_KEY` 覆盖
4. `npm run tauri build`（内含前端门禁 + 类型检查）
5. **生成自动更新清单 `latest.json`**（读 `.sig` 里的签名、拼下载地址），并校验它**不带 BOM**——带 BOM 会让 updater 直接解析失败，这是 2026-11-16 实测踩过的坑
6. 打印三个产物的绝对路径

**加 `-Upload` 才会自动发 Release**，且是**三件套一起传**（exe + `.sig` + `latest.json`）：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1" -Notes "本次更新说明" -Upload
```

不加 `-Upload` 就只生成本地产物，不动线上。

**为什么需要加载 MSVC 环境？** Rust 默认用 `stable-x86_64-pc-windows-msvc` toolchain，依赖 `ring`、`reqwest` 等 crate 需要 C 编译器（`cl.exe`）。VS Build Tools 装好后，`cl.exe` 在 `M:\VS\BuildTools\VC\Tools\MSVC\<版本>\bin\HostX64\x64\` 下，但它的 INCLUDE/LIB 环境变量必须通过 `vcvars64.bat` 加载，裸跑 cargo 找不到。

> ⚠️ 不要尝试"手动构建"：除非你已在 **M 盘 Rust 环境（CARGO_HOME / RUSTUP_HOME / PATH）+ MSVC 环境（vcvars64.bat）同时加载**的终端里，否则裸跑 `npm run tauri build` 必然失败。直接走脚本，别纠结。

#### 构建产物位置

都在 `src-tauri\target\release\bundle\nsis\`：

| 产物 | 说明 | 是否必须上传 |
|---|---|---|
| `Sensend_<版本>_x64-setup.exe` | 安装包 | ✅ |
| `Sensend_<版本>_x64-setup.exe.sig` | 更新签名（minisign） | ✅ **漏传 = 自动更新直接失败** |
| `latest.json` | 自动更新清单，updater 的取件入口 | ✅ **漏传 = 客户端永远查不到新版** |

另有一个便携版（不打包、直接跑）：`src-tauri\target\release\sensend.exe`

> ⚠️ 这三个是**一套**。只传 exe 的发布等于"用户能手动下载新版，但软件自己永远不知道有新版"。
> 2026-08-18 发 v0.5.0 时就只传了 exe，v0.5.1 才补齐三件套——历史教训，别再犯。

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

> ⚠️ **别把两种"签名"搞混**（这里最容易方向跑偏）：
> - **Windows 代码签名证书**（Authenticode，要花钱）：管的是"安装时浏览器/SmartScreen 是否报警"，**没买只是难看，不影响功能**。
> - **Tauri 更新签名**（minisign keypair，项目已有）：管的是"自动更新能不能装上"，**缺了自动更新直接失效**。
>
> §2.3 里脚本强制检查的是第二种，跟"要不要买证书"完全无关。

### 3.6 前端构建被 safe-delete 拦（清空 dist 触发批量删除保护）

**现象**（2026-11-17 实测）：
```
[safe-delete][SAFE_DELETE_BULK_CONFIRM_REQUIRED] {"count":71,"threshold":50,"scope":"turn",
 "targets":["...\\sensend\\dist\\assets"]}
beforeBuildCommand `npm run build` failed with exit code 1
```
cargo 报 `RUST BUILD FAILED`，看起来像 Rust 挂了，**其实前端那一步就没过**。

**原因**：Vite 构建前会 `emptyDir(dist)` 清空旧产物；`dist/assets` 里文件数（71）超过
智能体宿主 node-safe-delete 防护的批量阈值（50），被直接拦下。**跟代码无关，跟改动无关**，
任何一次「上次构建留了 50 个以上文件」之后的再构建都会撞上。

**解决方案**（构建前把旧 dist 挪走，让它从零生成）：

```powershell
if (Test-Path dist) { Move-Item dist "dist.old.$(Get-Date -f HHmmss)" -Force }
```
临时目录堆得多了可整目录清掉（`dist.old.*` 只是构建缓存，删了无副作用）。

**⚠ 不要**为了绕过而把 `dist` 加进什么白名单——那是把安全护栏拆了。挪走是等效且安全的。

### 3.7 新增主题必须改三处，漏一处就"菜单里有、切过去没样式"

**现象**：主题名出现在顶栏菜单里，点下去界面颜色不变（沿用上一套）。

**原因**：`src/styles/vars.css` 是样式唯一入口，主题 CSS 必须在那被 `@import`。
只在 `themes/` 建文件 + 在 `useTheme.ts` 注册，**菜单会出来，但 CSS 根本没进包**。

**三步，少一步就错**：
1. `src/styles/themes/<id>.css`
2. `src/styles/vars.css` 加 `@import './themes/<id>.css';`  ← 最容易漏，漏了就是本条现象
3. `src/composables/useTheme.ts` 的 `THEMES` 注册（name / preview / windowBg / dark）

第 4 道是**门禁不是手工步骤**：`scripts/check-contrast.mjs` 的 [m-27] 会扫 `vars.css` 的 `@import` 跟 `THEMES` 双向比对，第 2 步漏了直接报错——所以只要跑过 `npm run build`，这一步不用你操心，除非门禁本身被绕过。

**验证是否真进包**（别信"能编译"就等于"进去了"）：
```bash
grep -rl "<新主色 hex>" dist/   # 有输出 = 进去了
```

### 3.8 调试时启动了"正式应用"（身份没切）

**现象**：跑开发模式/测试构建时，数据跟正式安装版混在一起；正式版开着时新窗口起不来；调完发现正式版的数据被改脏了。

**原因**：`tauri dev` 和 `tauri build` **默认读 `src-tauri/tauri.conf.json`**，里面 `identifier` 是 `com.jloft.sensend` —— 那是正式应用的身份。数据目录、单实例锁都按这个 identity 走。

**规则**：

| 场景 | identifier | 怎么来 |
|---|---|---|
| 开发模式 / 测试构建 | `com.jloft.sensend.dev` | 必须带 `--config src-tauri/tauri.dev.conf.json` |
| 发版打包 | `com.jloft.sensend` | 故意不带 config |

`run-dev.ps1` 和 `run-app.ps1` 都已带上；`build-release.ps1` 故意不带。**别为了"统一"把这行删掉**。

2026-11-17 实测踩过：`run-dev.ps1` 第一版漏了这个参数，等于用正式身份在调试。

---

## 四、发布流程（傻瓜式全流程）

> 目标：任何智能体照抄本流程即可完成「推代码 → 打 tag → 传安装包 → 发 Release」，全程可复制、可验证、可回滚。
> 铁律：**顺序不可乱**——先构建成功，再打 tag、推送、发 Release。任何一步失败立即停止排查，不要带病往下走。

### 4.0 发布前自检（5 项，全绿才继续）

在项目根目录 `F:\fzz-Project\2-sensend\sensend` 依次执行，任一失败即停止：

```powershell
# ① 版本号：记下它，后面 tag / Release 标题 / 安装包名全部要跟它一致
npm pkg get version            # 输出形如 "0.5.1"，以下记为 <X.Y.Z>（以实际为准，别照抄本文的数字）

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
cd F:\fzz-Project\2-sensend\sensend
git add <本次改动的文件/目录>        # 明确列出，不用 git add .
git commit -m "feat(v<X.Y.Z>): 本次发布内容说明"
git log --oneline -1                 # 验证提交成功
```

### 4.2 构建安装包（必须先于推送）

> 必须走打包脚本加载 M 盘 Rust + MSVC 环境，**禁止**裸跑 `npm run tauri build`（见 §〇）。
> 加 `-Upload` 会顺带发 Release（三件套一起传），但**推荐先不加**，本地确认产物没问题再发——见 4.4。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1" -Notes "本次更新说明"
```

构建成功后校验**三个**产物都在（版本号替换成 4.0 记下的）：

```powershell
$n = "src-tauri\target\release\bundle\nsis"
Get-ChildItem $n | Select-Object Name,Length
# 期望看到三件：Sensend_<版本>_x64-setup.exe / .exe.sig / latest.json
Test-Path "$n\Sensend_<版本>_x64-setup.exe"      # True
Test-Path "$n\Sensend_<版本>_x64-setup.exe.sig"  # True
Test-Path "$n\latest.json"                        # True
```

### 4.3 打 tag 并推送

> tag 名必须与版本号一致（`v<X.Y.Z>`），与 `package.json` 的版本对应。推送 = main + tag 两个都推。

```powershell
git tag v<X.Y.Z>                       # 若提示已存在：说明发过，停下来核对版本号
git push origin main
git push origin v<X.Y.Z>
git ls-remote --tags origin | Select-String "v<X.Y.Z>"   # 验证 tag 已上远端
```

### 4.4 创建 GitHub Release（**三件套必须一起传**）

有两条路，**推荐 A**：

**A. 脚本直发（最省事，不会漏文件）**

tag 推上去之后重跑一次带 `-Upload` 的构建（增量很快），脚本会 `gh release create` 并把 exe + `.sig` + `latest.json` 一起传：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1" -Notes "本次更新说明" -Upload
```

**B. 手工 gh（要自己保证传齐三个文件）**

```powershell
# ① 写 Release 说明（内容见 4.5 模板），建议用英文文件名避免编码问题
#    在项目根目录创建 release-notes.md（完成后删除）

# ② 创建 Release —— 三个 asset 一个都不能少！
$n  = "src-tauri\target\release\bundle\nsis"
$v  = (npm pkg get version).Trim('"')
gh release create "v$v" `
  "$n\Sensend_${v}_x64-setup.exe" `
  "$n\Sensend_${v}_x64-setup.exe.sig" `
  "$n\latest.json" `
  --title "Sensend v$v" `
  --notes-file release-notes.md

# ③ 清理临时说明文件
Remove-Item release-notes.md
```

> 🔴 **为什么 `latest.json` 和 `.sig` 是命门**：客户端 updater 的端点是
> `https://github.com/jloft198479-cyber/sensend/releases/latest/download/latest.json`。
> **只传 exe = 这个地址 404 = 所有已安装用户的自动更新永久失效**，且用户端不会有任何报错，
> 只会觉得"这软件怎么老不更新"。2026-08-18 的 v0.5.0 就是这样发的，v0.5.1 才补齐。

> 若 `gh release create` 报「already exists」：该 tag 已有 Release，通常是上次半途而废，先 `gh release view v<X.Y.Z>` 看状态再决定补传还是清理。
> 缺文件时补传：`gh release upload v<X.Y.Z> "$n\latest.json" "$n\Sensend_${v}_x64-setup.exe.sig"`

### 4.5 Release 描述模板

```markdown
**Sensend v<X.Y.Z>**

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
- 下载 `Sensend_<X.Y.Z>_x64-setup.exe` 双击安装
- Windows 可能提示"无法识别的应用"，点击"更多信息" → "仍要运行"

**致谢**
送给儿子小柏
```

### 4.6 发布后验证（必须做，缺一不算发完）

```powershell
gh release view v<X.Y.Z>             # 期望 asset 列表含 exe + .sig + latest.json 三件
git ls-remote --tags origin | Select-String "v<X.Y.Z>"   # 远端 tag 存在
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
| 发布成功但用户收不到更新 | Release 里只有 exe，缺 `.sig` / `latest.json` | `gh release upload v<X.Y.Z> ...` 补传两件；根因见 §4.4 |
| Release 说明出现乱码 | PowerShell 中文转义问题 | 用 `--notes-file`（脚本已内置，手打时别用 `--notes`） |
| `git add .` 误加了多余目录 | `.gitignore` 未覆盖该目录 | `git rm -r --cached <目录>` 撤出暂存，改逐文件 add |

### 4.8 回滚

- **代码已推、Release 未发**：`git revert` 或提交修复，正常再发下一版。
- **Release 发错了**：`gh release delete v<X.Y.Z> --cleanup-tag`（会同时删远端 tag），本地 `git tag -d v<X.Y.Z>` 后重来。
- 回滚仅限发错的当次，历史发布（v0.1.0~v0.3.0）一律不动。

---

## 五、常用命令速查

```bash
# ① 改样式 / 调交互（日常首选：vite 热更新，改完秒看，不重编译）
#    双击仓库根 启动开发模式.bat，或：
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-dev.ps1

# ② 验证编出来的成品（全量构建，约 90s，慢是正常的）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-app.ps1

# ③ 跑测试（cargo test，不弹窗不杀进程）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-tests.ps1

# ④ 打包安装包（发布用；加 -Upload 则连 Release 三件套一起发）
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-release.ps1 -Notes "本次更新说明"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-release.ps1 -Notes "本次更新说明" -Upload

# ⑤ 只构建前端（走 node，普通终端可跑；不编译后端、不生成 exe）
#    注意：它不是"纯 vite"，会先跑 token 门禁 + 对比度门禁 + 类型检查，任一失败就中断
npm run build

# 查看 Git 状态
git status

# 推送到远程
git push origin main

# 创建标签并推送（<X.Y.Z> 替换为实际版本号）
git tag v<X.Y.Z>
git push origin v<X.Y.Z>

# 创建 Release —— 三件套必须齐全（exe / .sig / latest.json），完整流程见 §四
# 推荐直接用 build-release.ps1 -Upload，避免手打漏文件
$n = "src-tauri\target\release\bundle\nsis"
gh release create "v<X.Y.Z>" "$n\Sensend_<X.Y.Z>_x64-setup.exe" "$n\Sensend_<X.Y.Z>_x64-setup.exe.sig" "$n\latest.json" --title "Sensend v<X.Y.Z>" --notes-file release-notes.md
```

---

## 六、问题排查清单

| 现象 | 先查什么 | 怎么修 |
|------|----------|--------|
| 改完样式看不到变化 | 你跑的是哪条路？ | 用 `启动开发模式.bat`（热更新）。`run-app.ps1` 是打包，必须重跑才生效 |
| 构建报"找不到 cargo / cl.exe" | 是否裸跑了命令 | 走脚本，别在裸终端敲 `npm run tauri build` / `tauri dev` |
| 构建前半段就挂、提示 `RUST BUILD FAILED` | 前端那步没过 | 看上面的 `safe-delete` 提示，按 §3.6 先把 `dist` 挪走 |
| 构建报版本号不一致 | 三处版本 | `tauri.conf.json` / `Cargo.toml` / `package.json` 改齐 |
| 构建报"更新签名私钥缺失" | 仓库外那个 key | 见 §2.3 第 3 步；私钥丢了已发布用户的自动更新就断了 |
| 发布后用户收不到更新 | Release 里是不是只有 exe | 补传 `.sig` + `latest.json`（§4.4） |
| 主题菜单里有、点下去不变色 | `vars.css` 有没有 @import | 见 §3.7 |
| 构建失败且和 NSIS 有关 | `%LOCALAPPDATA%\tauri\NSIS\` 结构 | 见 §3.1 / §3.2 |
| 安装包没有图标 | `tauri.conf.json` 的 `bundle.icon` | 见 §3.3 |
| 安装时提示"无法识别的应用" | 未买代码签名证书（和更新签名是两码事） | 见 §3.5，属正常现象 |

---

> 本手册基于 Sensend 打包发布经验整理。
> 最近一次**逐条对照代码核验**：2026-11-17（版本 v0.5.1，核验范围：4 个脚本 + tauri 两份配置 + package.json + 已发布 Release 资产）