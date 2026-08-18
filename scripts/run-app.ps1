# Sensend 一键启动脚本：构建 release 版并启动应用窗口
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-app.ps1
$ErrorActionPreference = "Continue"

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
    Write-Host "[1/4] Loading MSVC environment..." -ForegroundColor Cyan
    $output = cmd /c "chcp 65001 >nul && `"$vcvars`" >nul 2>&1 && set"
    foreach ($line in $output) {
        if ($line -match "^([^=]+)=(.*)$") {
            # PATH 大小写变体（Path/PATH/path）在智能体工具宿主进程里并存，
            # cmd set 会全部输出且循环内互相覆盖，导致 MSVC 路径丢失 → link.exe not found。
            # 循环内跳过 PATH，下面单独取含 Hostx64 的那份整体替换。
            if ($matches[1] -match "^(?i)path$") { continue }
            Set-Item -Path ("env:" + $matches[1]) -Value $matches[2]
        }
    }
    $vcPath = @($output | Where-Object { $_ -match "^(?i)path=.*Hostx64" }) | Select-Object -First 1
    if ($vcPath) { $env:PATH = ($vcPath -replace "^(?i)path=", "") }
    # cargo 前置（无论 vcvars 是否覆盖过 PATH，保证 cargo 可用）
    $env:PATH = "M:\rust\.cargo\bin;$env:PATH"
}

Set-Location "F:\fzz-Project\sensend\sensend"
Write-Host "=== Sensend Build & Run ===" -ForegroundColor Green
Write-Host "cargo: $((Get-Command cargo -ErrorAction SilentlyContinue).Source)"
Write-Host "rustc: $(rustc --version)"

# ── 杀掉残留的 sensend 进程（否则 exe 被占用，构建报“拒绝访问”os error 5）──
Get-Process -Name "sensend" -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "Killing lingering sensend process (PID $($_.Id))..." -ForegroundColor Yellow
    Stop-Process -Id $_.Id -Force
}

Write-Host "[2/4] Building frontend..." -ForegroundColor Cyan
npm run build
if ($LASTEXITCODE -ne 0) { Write-Host "FRONTEND BUILD FAILED" -ForegroundColor Red; exit 1 }

Write-Host "[3/4] Building Rust (release, WebView2 full sidecar)..." -ForegroundColor Cyan
npm run tauri build -- --no-bundle
if ($LASTEXITCODE -ne 0) { Write-Host "RUST BUILD FAILED" -ForegroundColor Red; exit 1 }

$exe = "src-tauri\target\release\sensend.exe"
if (Test-Path $exe) {
    Write-Host "[4/4] Launching $exe ..." -ForegroundColor Cyan
    Start-Process $exe
} else {
    Write-Host "exe not found: $exe" -ForegroundColor Red
    exit 1
}