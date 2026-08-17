# Sensend 测试脚本（PowerShell 版，供智能体/终端调用；双击用户请用 run-tests.bat）
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run-tests.ps1
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
    Write-Host "[1/2] Loading MSVC environment..." -ForegroundColor Cyan
    $output = cmd /c "chcp 65001 >nul && `"$vcvars`" >nul 2>&1 && set"
    foreach ($line in $output) {
        if ($line -match "^([^=]+)=(.*)$") {
            Set-Item -Path "env:$($matches[1])" -Value $matches[2]
        }
    }
    # vcvars 会重写 PATH，重新前置 cargo
    $env:PATH = "M:\rust\.cargo\bin;$env:PATH"
}

Set-Location "F:\fzz-Project\sensend\sensend\src-tauri"
Write-Host "=== Sensend Tests ===" -ForegroundColor Green
Write-Host "cargo: $((Get-Command cargo -ErrorAction SilentlyContinue).Source)"
Write-Host "rustc: $(rustc --version)"

Write-Host "[2/2] Running cargo test..." -ForegroundColor Cyan
cargo test
exit $LASTEXITCODE
