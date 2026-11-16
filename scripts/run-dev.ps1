# Sensend 开发模式（热更新）：改样式 / 前端代码即时生效，不用重新编译
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-dev.ps1
#
# 与 run-app.ps1 的区别（两个都留着，各管一段）：
#   run-dev.ps1  → 起 vite dev server + tauri dev，窗口连 http://localhost:1420。
#                  改 CSS / 组件 / ts 由 vite 直接推给窗口（HMR），秒看到，**不重编译**。
#                  适合调样式、调交互。Rust 侧改动仍会自动重编并重启窗口。
#   run-app.ps1  → 打包成 target\release\sensend.exe 再启动。
#                  界面资源是编译进 exe 的，改完必须重跑；适合"验证真实产物"和发布前自检。
$ErrorActionPreference = "Continue"

# 控制台按 UTF-8 输出：bat 已 chcp 65001，这里同步 PS 的输出编码，
# 否则 PowerShell 5.1 会按 GBK 写 console，中文提示全变乱码。
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

# ── 项目根自定位（不写死盘符，项目改名/挪位置都不坏）──
$Root = Split-Path -Parent $PSScriptRoot

# ── Rust 环境（装在 M 盘自定义位置）──
$env:CARGO_HOME = "M:\rust\.cargo"
$env:RUSTUP_HOME = "M:\rust\.rustup"
$env:PATH = "M:\rust\.cargo\bin;$env:PATH"
$env:HTTP_PROXY = ""
$env:HTTPS_PROXY = ""
$env:CARGO_HTTP_CHECK_REVOKE = "false"

# ── 加载 VS Build Tools 的 MSVC 环境 ──
$vcvars = "M:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (Test-Path $vcvars) {
    Write-Host "[1/3] Loading MSVC environment..." -ForegroundColor Cyan
    $output = cmd /c "chcp 65001 >nul && `"$vcvars`" >nul 2>&1 && set"
    foreach ($line in $output) {
        if ($line -match "^([^=]+)=(.*)$") {
            # PATH 大小写变体（Path/PATH/path）在智能体工具宿主进程里并存，
            # cmd set 会全部输出且循环内互相覆盖，导致 MSVC 路径丢失 → link.exe not found。
            if ($matches[1] -match "^(?i)path$") { continue }
            Set-Item -Path ("env:" + $matches[1]) -Value $matches[2]
        }
    }
    $vcPath = @($output | Where-Object { $_ -match "^(?i)path=.*Hostx64" }) | Select-Object -First 1
    if ($vcPath) { $env:PATH = ($vcPath -replace "^(?i)path=", "") }
    $env:PATH = "M:\rust\.cargo\bin;$env:PATH"
}

Set-Location $Root
Write-Host "=== Sensend Dev (hot reload) ===" -ForegroundColor Green
Write-Host "cargo: $((Get-Command cargo -ErrorAction SilentlyContinue).Source)"

# ── 1420 被残留的 vite 占着 → 先放掉 ──
# vite.config.ts 里 strictPort: true，端口被占会直接失败，不会自动换端口。
Write-Host "[2/3] Freeing port 1420..." -ForegroundColor Cyan
Get-NetTCPConnection -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue |
    Select-Object -ExpandProperty OwningProcess -Unique |
    ForEach-Object {
        Write-Host "  stopping PID $_ (stale vite dev server)" -ForegroundColor Yellow
        Stop-Process -Id $_ -Force -ErrorAction SilentlyContinue
    }

Write-Host "[3/3] Starting tauri dev (HMR)... 改样式即时生效，Ctrl+C 退出" -ForegroundColor Cyan
Write-Host "  窗口连 http://localhost:1420，改 CSS/组件/ts 由 vite 直接推给窗口。" -ForegroundColor DarkGray

# ── 必须带测试身份！──
# 不加 --config 时 tauri dev 读 src-tauri/tauri.conf.json，identifier = com.jloft.sensend
# 那是【正式应用】的身份：同一个数据目录、同一把单实例锁 → 调试会污染正式数据，
# 正式版开着时还可能起不来。tauri.dev.conf.json 把它改成 com.jloft.sensend.dev，
# 数据目录分离、锁分离，两版可并存。run-app.ps1 也走同一个配置，保持一致。
npm run tauri dev -- --config src-tauri/tauri.dev.conf.json
