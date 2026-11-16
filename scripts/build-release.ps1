# Sensend Build Script
# 加载 MSVC 环境（vcvars64.bat）后执行 tauri build，并生成自动更新清单 latest.json
# 用法：powershell -NoProfile -ExecutionPolicy Bypass -File scripts\build-release.ps1 [-Notes "发版说明"] [-Upload]
param(
    # 发版说明（写入 latest.json 的 notes，留空则用默认文案）
    [string]$Notes = "",
    # 构建完成后自动上传到远端 Release（默认 false，只生成本地产物与清单）
    [switch]$Upload
)
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

# ── 切到项目根目录（自定位，不写死盘符）──
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

# ── 版本三处一致性检查（tauri.conf.json / Cargo.toml / package.json）──
# -Encoding UTF8 必须显式：package.json 等含中文且无 BOM，PS5.1 默认按 ANSI 读，
# GBK 双字节会吞掉闭合引号导致 JSON 解析崩（发版实测踩坑 2026-11-16）
$confVersion = (Get-Content "src-tauri/tauri.conf.json" -Raw -Encoding UTF8 | ConvertFrom-Json).version
$cargoVersion = (Select-String -Path "src-tauri/Cargo.toml" -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$npmVersion = (Get-Content "package.json" -Raw -Encoding UTF8 | ConvertFrom-Json).version
if ($confVersion -ne $cargoVersion -or $confVersion -ne $npmVersion) {
    Write-Host "ERROR: 版本号三处不一致 conf=$confVersion cargo=$cargoVersion npm=$npmVersion（发版前必须同步）" -ForegroundColor Red
    exit 1
}

# ── 更新签名私钥（updater 工件签名必需；私钥不入库，见 docs/AUTO-UPDATE-DESIGN.md）──
# 注意：tauri build 只认 TAURI_SIGNING_PRIVATE_KEY（内容或路径均可，官方文档）；
# TAURI_SIGNING_PRIVATE_KEY_PATH 仅对 signer sign 子命令有效——用错变量名构建必失败
# 私钥位置：优先环境变量 SENSEND_SIGN_KEY 覆盖，否则用 9-CNB 下的默认位置
# （跨项目共享密钥，故意放在仓库外，不跟代码走；项目挪位置时要同步改这里）
$signKeyPath = if ($env:SENSEND_SIGN_KEY) { $env:SENSEND_SIGN_KEY } else { "F:\fzz-Project\9-CNB\keys\sensend-updater\sensend-updater.key" }
if (Test-Path $signKeyPath) {
    $env:TAURI_SIGNING_PRIVATE_KEY = $signKeyPath
    # 空口令密钥在非 CI 环境会弹一次 "Password:" 提示（按回车即过）；设 CI 让 CLI 跳过交互
    $env:CI = "true"
} else {
    Write-Host "ERROR: 更新签名私钥缺失：$signKeyPath" -ForegroundColor Red
    Write-Host "生成：npx tauri signer generate -w `"$signKeyPath`" --password `"`" --ci"
    Write-Host "私钥丢失 = 已发布用户收不到自动更新，务必备份。"
    exit 1
}

Write-Host ""
Write-Host "=== Sensend Build v$confVersion ===" -ForegroundColor Green
Write-Host "Rust: $(rustc --version)" -ForegroundColor Cyan
Write-Host "cl.exe: $((Get-Command cl.exe -ErrorAction SilentlyContinue).Source)" -ForegroundColor Cyan
Write-Host ""

npm run tauri build
if ($LASTEXITCODE -ne 0) { Write-Host "ERROR: tauri build 失败" -ForegroundColor Red; exit 1 }

# ── 生成自动更新清单 latest.json ──
$nsisDir = "src-tauri/target/release/bundle/nsis"
$exe = Get-ChildItem "$nsisDir/*-setup.exe" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$sigFile = "$($exe.FullName).sig"
if (-not (Test-Path $sigFile)) {
    Write-Host "ERROR: 未找到签名文件 $sigFile（createUpdaterArtifacts 未生效？）" -ForegroundColor Red
    exit 1
}
if (-not $Notes) { $Notes = "Sensend v$confVersion" }
$tag = "v$confVersion"
$manifest = [ordered]@{
    version  = $confVersion
    notes    = $Notes
    pub_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss'Z'")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = (Get-Content $sigFile -Raw).Trim()
            url = "https://github.com/jloft198479-cyber/sensend/releases/download/$tag/$($exe.Name)"
        }
    }
}
# 必须绝对路径：.NET File API 按进程 CWD 解析相对路径，不认 PowerShell 的 Set-Location
# （发版实测 2026-11-16：相对路径导致 ReadAllBytes 找不到 latest.json）
$manifestPath = Join-Path (Resolve-Path $nsisDir).Path "latest.json"
# 必须 UTF-8 无 BOM：PS5.1 的 -Encoding utf8 带 BOM，serde_json 对 BOM 直接解析失败，
# 且该失败发生在插件 endpoint 循环的 200 分支内（? 中止）→ 连第二端点兜底都不触发
$manifestJson = $manifest | ConvertTo-Json -Depth 5
[System.IO.File]::WriteAllText($manifestPath, $manifestJson, (New-Object System.Text.UTF8Encoding($false)))
$first3 = [System.IO.File]::ReadAllBytes($manifestPath)[0..2]
if ($first3[0] -eq 0xEF -and $first3[1] -eq 0xBB -and $first3[2] -eq 0xBF) {
    Write-Host "ERROR: latest.json 仍带 BOM，updater 将无法解析" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "=== 构建产物 ===" -ForegroundColor Green
Write-Host "安装包：$($exe.FullName)"
Write-Host "签名：  $sigFile"
Write-Host "清单：  $manifestPath"

# ── 上传（-Upload 开关；两条渠道尽力自动，不可用则打印手动指引）──
if (-not $Upload) {
    Write-Host ""
    Write-Host "（未加 -Upload，跳过上传。手动发布：GitHub 用 gh release create $tag；CNB 按 docs/AUTO-UPDATE-DESIGN.md）" -ForegroundColor Yellow
    exit 0
}

# GitHub：gh CLI 若可用则自动发 Release（latest.json 是 updater 的取件入口，必须同传）
$gh = Get-Command gh -ErrorAction SilentlyContinue
if ($gh) {
    # 说明文字必须走 --notes-file：PowerShell 里 --notes 直接传中文会转义乱码（§4.7 踩过）
    # 必须 UTF-8 无 BOM，否则 gh 读文件首行带 BOM
    $notesFile = Join-Path $env:TEMP "sensend-release-notes-$tag.md"
    [System.IO.File]::WriteAllText($notesFile, $Notes, (New-Object System.Text.UTF8Encoding($false)))
    gh release create $tag $exe.FullName $sigFile $manifestPath --title "Sensend v$confVersion" --notes-file $notesFile
    Remove-Item $notesFile -Force -ErrorAction SilentlyContinue
} else {
    Write-Host "未安装 gh CLI，GitHub Release 请手动创建并上传三件产物（tag=$tag）" -ForegroundColor Yellow
}

# CNB：国内直连渠道（公开仓库附件免登录可下，updater 端点已登记）
# 流程 = cnb CLI 三步（建 release → 拿 upload_url → PUT 上传 → confirmation），见 9-CNB 发版文档
$cnb = Get-Command cnb -ErrorAction SilentlyContinue
if ($cnb) {
    Write-Host "CNB 上传：按 F:\fzz-Project\9-CNB\docs\CNB发版分发-全流程与踩坑经验.md 三步走（tag=$tag）" -ForegroundColor Cyan
    # 附件上传细节依赖 cnb CLI 交互输出，此处仅提示；自动化待 cnb CLI 就位后补齐
} else {
    Write-Host "未安装 cnb CLI，CNB 发版按 9-CNB 文档手动执行（tag=$tag，附件含安装包+latest.json）" -ForegroundColor Yellow
}
