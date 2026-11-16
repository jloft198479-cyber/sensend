@echo off
cls
echo ============================================
echo  Sensend 开发模式（热更新 / HMR）
echo  改样式、改组件即时生效，不用重新编译
echo  关掉这个黑窗口 = 结束开发模式
echo ============================================
echo.

REM -- 委托给 ps1 脚本加载 MSVC+Rust 环境 --
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-dev.ps1"

echo.
echo Dev server stopped. Press any key to exit.
pause >nul
