# AUTO-UPDATE-DESIGN.md — 应用内自动更新设计（真源）

> 创建：2026-11-16 · 参考：solo（F:\fzz-Project\03-solo，生产验证同款方案）
> 相关文档：[BUILD-GUIDE.md](file:///F:/fzz-Project/2-sensend/sensend/docs/BUILD-GUIDE.md)（构建发版）、9-CNB 发版文档（CNB 渠道）

## 1. 架构总览

```
启动 8s 后静默检查（24h 限频）──┐
设置菜单「检查更新」（手动）────┤
                               ▼
              useUpdater.checkUpdateAvailability()
              ① detect_proxy_for_update（Rust：env→系统代理→本地端口 7890/10809）
              ② @tauri-apps/plugin-updater check()（endpoints 按序尝试）
                               ▼
        发现新版 ─→ toast「更新」按钮 ─→ download（常驻 toast 进度 %）
                               ▼
              beforeInstall = flushSave（保存笔记！安装器直接退出应用）
                               ▼
              update.install()（NSIS 静默升级后自动重启）
```

## 2. 更新源（endpoints，按序尝试）

| 序 | 端点 | 状态 |
|---|---|---|
| 1 | `https://github.com/jloft198479-cyber/sensend/releases/latest/download/latest.json` | ✅ 主路径（GitHub 直连 + 代理探测兜底，solo 同款） |
| 2 | `https://cnb.cool/fzz198479/sensend/-/releases/download/latest/latest.json` | ⏸ 预留（CNB 仓库未建；建为**公开**仓库后此直链免登录可下，把本行移到首位即成国内直连主路径） |

签名：minisign keypair（`npx tauri signer generate` 生成，2026-11-16）。
- 公钥：tauri.conf.json `plugins.updater.pubkey`（入库）
- 私钥：`F:\fzz-Project\9-CNB\keys\sensend-updater\sensend-updater.key`（**不入库；丢失 = 已发布用户永远收不到更新，必须备份**）
- 构建注入：`TAURI_SIGNING_PRIVATE_KEY_PATH`（build-release.ps1 自动指向）

## 3. 职责分布

| 层 | 文件 | 职责 |
|---|---|---|
| Rust | `src-tauri/src/commands/updater.rs` | 代理探测（env → winreg 系统代理 → 端口扫描）+ `detect_proxy_for_update` 命令 |
| Rust | `lib.rs` | `tauri_plugin_updater::Builder::new().build()` 注册 |
| 配置 | `tauri.conf.json` | `bundle.createUpdaterArtifacts: true`、plugins.updater（dialog:false，UX 自管） |
| 能力 | `capabilities/default.json` | `updater:default` / `allow-check` / `allow-download-and-install` |
| 前端 | `composables/useUpdater.ts` | 统一检查入口 + 状态机 + 下载进度 + 安装前善后；启动静默 24h 限频（localStorage） |
| 前端 | `useToast.ts` | `duration<=0` 常驻 toast + `updateMessage` 原地更新（进度显示） |
| 前端 | `useEditor.ts → flushSave` | 安装前保存笔记（doSave + 兜底等在途保存；安装器不走 app-exit-request 事件链！） |
| UI | `FooterBar.vue` 设置菜单 | 「检查更新」入口 → emit → App.vue `manualCheck(flushSave)` |
| 发版 | `scripts/build-release.ps1` | 版本三处一致性检查 → 签名构建 → 生成 `latest.json` →（-Upload）gh/cnb 尽力自动上传 |

## 4. 发版流程（发 vX.Y.Z）

1. 三处版本号同步：tauri.conf.json / Cargo.toml / package.json（脚本强校验，不一致直接退出）
2. `powershell -File scripts/build-release.ps1 -Notes "发版说明"` → 产物：`*-setup.exe` + `*.exe.sig` + `latest.json`（target/release/bundle/nsis/）
3. GitHub：`gh release create vX.Y.Z <exe> <exe.sig> latest.json`（或脚本 -Upload 自动）
4. CNB（建仓后）：按 9-CNB 文档三步上传同样三件产物，tag 同名；`latest.json` 的 url 字段此时仍是 GitHub（updater 从 endpoint 拿清单、从清单拿 url，两处可不同源——后续可做双 manifest）

## 5. 已知风险与待办

- **真机升级闭环未跑**：需构建 0.5.0（含 updater）→ 安装 → 发 0.5.1 → 验证应用内升级。当前 0.5.0 老用户无 updater，首轮只能手动安装新版。
- CNB 直链免登录**未实测**（仓库未建）；文档断言公开仓库可用，建仓发版时第一时间 curl 验证。
- `install()` 绕过 app-exit-request，故 flushSave 挂在 beforeInstall——若未来有其他退出善后，需同步在此登记。
- 代理探测只覆盖 http(s) 代理，socks 不支持（沿用 solo 约定）。
