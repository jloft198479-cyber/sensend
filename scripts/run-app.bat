@echo off
cls
echo ============================================
echo  Sensend 一键构建 & 启动（应用窗口）
echo  用法：双击运行，等黑窗口转完自动弹出窗口
echo ============================================
echo.

REM -- 委托给 ps1 脚本加载 MSVC+Rust 环境 --
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-app.ps1"

echo.
echo Done. Press any key to exit.
pause >nul