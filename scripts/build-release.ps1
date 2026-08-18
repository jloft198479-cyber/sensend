# Sensend Build Script
# 加载 MSVC 环境（vcvars64.bat）后执行 tauri build
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-release.ps1
$ErrorActionPreference = "Stop"

# ── Rust 环境（装在 M 盘自定义位置）──
$env:CARGO_HOME = "M:\rust\.cargo"
$env:RUSTUP_HOME = "M:\rust\.rustup"
$env:PATH = "M:\rust\.cargo\bin;$env:PATH"
$env:HTTP_PROXY = ""
$env:HTTPS_PROXY = ""
$env:CARGO_HTTP_CHECK_REVOKE = "false"

# ── 加载 VS Build Tools 的 MSVC 环境（INCLUDE/LIB/PATH 等）──
$vcvars = "M:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (Test-Path $vcvars) {
    Write-Host "Loading MSVC environment..." -ForegroundColor Cyan
    # cmd /c 调用 vcvars64.bat，输出 set 结果，再注入当前 PowerShell 会话
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
} else {
    Write-Host "WARNING: $vcvars not found, will try build without MSVC env" -ForegroundColor Yellow
}

# ── 切到项目根目录并打包 ──
Set-Location "F:\fzz-Project\sensend\sensend"
Write-Host ""
Write-Host "=== Sensend Build ===" -ForegroundColor Green
Write-Host "Rust: $(rustc --version)" -ForegroundColor Cyan
Write-Host "cl.exe: $((Get-Command cl.exe -ErrorAction SilentlyContinue).Source)" -ForegroundColor Cyan
Write-Host ""

npm run tauri build
