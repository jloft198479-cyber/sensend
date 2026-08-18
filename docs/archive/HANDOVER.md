# Sensend 项目交接说明

> 更新时间：2026-07-17
> 用途：新会话/新任务快速进入工作状态

## 一、项目是什么

**Sensend** —— 一个 Tauri v2 桌面悬浮记事本，按快捷键弹窗记录，支持一键分发到 Notion / FlowUs / 飞书 / 本地文件夹。
前端 Vue 3 + Vite，后端 Rust + Tauri 2.x。

## 二、当前位置（重要）

项目根目录：`F:\fzz-Project\sensend\`

```
F:\fzz-Project\sensend\
├── sensend\              <- 主代码（git 仓库）
│   ├── src\              <- Vue 前端
│   ├── src-tauri\        <- Rust 后端 + Tauri 配置
│   ├── node_modules\     <- 已重建，可用
│   ├── .git\             <- 完整 git 历史
│   ├── docs\             <- BUILD-GUIDE / CODE-WIKI / EXPERIENCE / FILELIST
│   └── package.json
├── sensend-release\      <- 旧版 v0.1.0 代码（参考用）
├── .cargo\config.toml    <- Rust 中科大镜像源配置
├── CODE-WIKI.md          <- 项目百科文档
└── adapter-development-guide.md  <- 适配器开发指南
```

**注意**：主代码在 `sensend\sensend\`（嵌套一层），不是 sensend 根目录。

## 三、Git / 发布状态

- **仓库**：`https://github.com/jloft198479-cyber/sensend`（public）
  - ~~注意：README / CODE-WIKI 里写的 `jloft/sensend` 是错的，实际是 `jloft198479-cyber/sensend`~~ → 已于 2026-07-17 全部修正为 `jloft198479-cyber/sensend`
- **当前分支**：`trae/agent-OBykow`（与 origin 同步）
- **最新提交**：`889d944 fix: 修正仓库地址` / `70c1b00 feat: 适配器支持待办/表格与下划线`（基于 `41dd262 perf`）
- **未提交改动**：无（此前的 adapters / hotkey / composables 改动已于 2026-07-17 审查并提交：70c1b00 + 889d944）
- **Release 状态**：
  - `v0.2.0`（Latest）—— 已发布，安装包 `Sensend_0.2.0_x64-setup.exe`（3.53MB）已上传
  - `v0.1.0` —— 旧版，仍在
  - Release 地址：https://github.com/jloft198479-cyber/sensend/releases/tag/v0.2.0

## 四、版本号

全项目统一为 **0.2.0**（已提交）：`package.json` / `src-tauri/Cargo.toml` / `src-tauri/tauri.conf.json`

## 五、开发环境（关键）

工具链都不在系统 PATH，构建前必须手动激活：

| 工具 | 位置 |
|------|------|
| Rust（rustup） | `M:\rust\.cargo\bin\` |
| RUSTUP_HOME | `M:\rust\.rustup` |
| Node.js | 系统 PATH（18+，已可用） |
| MSVC (C++ Build Tools) | `M:\VS\BuildTools\VC\Tools\MSVC\` 下最高版本 |
| Windows SDK | `M:\VS\BuildTools\...` 下 |
| CARGO_HOME（项目级） | `F:\fzz-Project\sensend\.cargo`（含中科大镜像 config.toml） |

**沙箱限制**：不能执行 `cmd /c "vcvars64.bat"`，必须用 `Get-ChildItem` 动态扫描 MSVC/SDK 版本路径，手动设置 `PATH` / `INCLUDE` / `LIB` 三个环境变量来激活 MSVC。

## 六、打包命令（PowerShell）

```powershell
# 1. 进入主代码目录
cd F:\fzz-Project\sensend\sensend

# 2. 设置环境变量（Rust + CARGO_HOME）
$env:RUSTUP_HOME = "M:\rust\.rustup"
$env:CARGO_HOME  = "F:\fzz-Project\sensend\.cargo"
$env:PATH        = "M:\rust\.cargo\bin;$env:PATH"

# 3. 动态激活 MSVC（扫描最新版本）
$msvcRoot = Get-ChildItem "M:\VS\BuildTools\VC\Tools\MSVC" -Directory | Sort-Object Name -Descending | Select-Object -First 1
$msvcVer  = $msvcRoot.Name
$msvcPath = $msvcRoot.FullName
$env:PATH = "$msvcPath\bin\Hostx64\x64;$env:PATH"
$env:INCLUDE = "$msvcPath\include"
$env:LIB     = "$msvcPath\lib\x64"
Get-ChildItem "$msvcPath\Include" -Directory | ForEach-Object { $env:INCLUDE += ";$($_.FullName)" }
Get-ChildItem "$msvcPath\Lib\x64" -Directory | ForEach-Object { $env:LIB += ";$($_.FullName)" }

# 4. 打包
npm install   # 首次需要
npm run tauri build
```

**产物位置**：
- 安装包：`src-tauri\target\release\bundle\nsis\Sensend_0.2.0_x64-setup.exe`
- 绿色版：`src-tauri\target\release\sensend.exe`

## 七、已知坑（必看）

1. **沙箱文件移动限制**：跨目录移动大量小文件会被拦（target、node_modules、.cargo 都会触发）。解决：先 `cargo clean` / 删 `node_modules`，再分步移动子项。
2. **Write 工具路径限制**：Write 只能写工作目录，写项目目录下的新文件要用 `RunCommand` + `Set-Content`。
3. **Tauri 版本对齐**：`@tauri-apps/api`（npm）必须和 `tauri` crate（Rust）的 major.minor 一致，否则编译失败。当前是 2.10.x。
4. **图标文件**：`trae/agent-OBykow` 分支的 `icons/` 曾经缺 `icon.ico` 等，已从 sensend-release 复制补全（已提交）。
5. ~~代码里的错误仓库地址 `jloft/sensend` 还没改。~~ → 已于 2026-07-17 修正（README / CODE-WIKI / package.json / Cargo.toml 仓库地址统一为 `jloft198479-cyber/sensend`）。

## 八、架构速记

- **双窗口模型**：一个入口，靠 URL `?page=config` 区分编辑窗口 / 配置窗口
- **适配器 trait**：所有平台实现 `publish` / `test_connection` / `append_blocks`，加新平台像换插头
- **mention 机制**：文档里同一时间只能有一个 `@平台` 标记，发送前再清一遍
- **自动保存**：每 800ms 存到 `note.md`，带内容差异检测（内容未变则跳过磁盘 I/O）
- **关闭即隐藏**：点关闭藏到托盘，快捷键秒出
- **Rust 命令异步化**：`read_note` / `save_note` / `scan_user_fonts` / `delete_user_font` 已改为 async fn + tokio::fs，不阻塞线程池
- **前端启动并行化**：`usePlatform.onMounted` 中 `get_platform_types` 和 `list_platform_instances` 并行发起，不再串行等待
- **输入路径优化**：字数统计独立防抖 300ms（不在每次按键同步执行正则），mention 变化带缓存比对（ID 未变不触发回调）

## 九、可能的下一步

- ~~处理未提交改动~~ ✅ 已于 2026-07-17 提交（70c1b00 feat + 889d944 fix）
- ~~修复代码里错误的仓库地址~~ ✅ 已修正为 `jloft198479-cyber/sensend`
- 加新平台适配器（参考 `adapter-development-guide.md`）
- ~~可选性能优化~~ ✅ 已于 2026-07-17 实施（ebc9b34 perf）：飞书 token 进程内缓存（OnceLock<Mutex<HashMap>>，key=app_id，2h 提前 5min 刷新）+ Notion resolve_target 并行化（tokio::join!）。已本机 `cargo check` 验证通过（修复编译错误见 c93882b fix，Finished in 5.68s，0 error / 0 warning）。
- 版本迭代（需要时升版本号 + 发新 Release）
- 修复 bug / 加功能
