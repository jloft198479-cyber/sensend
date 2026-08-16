# Sensend 构建环境速查（智能体必读）

> 目的：让任何智能体 / 协作者在本机用**正确方式**构建，避免把"没加载环境"误判成"MSVC 构建失败"。

## 铁律（违反必挂）

- ✅ **唯一正确构建方式**——走打包脚本：

  ```powershell
  cd F:\fzz-Project\sensend\sensend
  powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\build-release.ps1"
  ```

- ❌ **禁止**在普通终端直接执行 `npm run tauri build`、`cargo build`、`npm run tauri dev`。

## 原因（一句话）

Rust（`M:\rust`）和 VS Build Tools（`M:\VS\BuildTools`）都**不在系统 PATH**，普通终端找不到 `cargo` / `cl.exe`。只有 `build-release.ps1` 会设置 `CARGO_HOME` / `RUSTUP_HOME` 并调用 `vcvars64.bat` 加载 MSVC 环境（INCLUDE/LIB）。

**环境本身完好，报错 ≠ 环境坏，无需重装、无需修复。**

## 环境现状（已核验 2026-08-17）

| 项 | 值 |
|---|---|
| 工具链 | `stable-x86_64-pc-windows-msvc`（rustc / cargo 1.96.0） |
| MSVC | 14.44.35207（`cl.exe` 就位） |
| NSIS | `makensis.exe` + `nsis_tauri_utils.dll` v0.5.3 就位 |
| cargo 源 | 中科大 sparse 镜像 |

## 调试后端时单独跑 cargo（须先手动加载环境）

```powershell
$env:CARGO_HOME = "M:\rust\.cargo"
$env:RUSTUP_HOME = "M:\rust\.rustup"
$env:PATH = "M:\rust\.cargo\bin;$env:PATH"
# 还需在 x64 Native Tools 终端里，或先执行 vcvars64.bat 注入 INCLUDE/LIB
```

## 产物位置

- 便携版：`src-tauri\target\release\sensend.exe`
- 安装包：`src-tauri\target\release\bundle\nsis\Sensend_<版本号>_x64-setup.exe`
