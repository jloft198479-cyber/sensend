@echo off
chcp 65001 >nul
cls
echo ============================================
echo  Sensend 测试（cargo test，不杀进程、不弹窗）
echo ============================================
echo.

REM -- 委托给 ps1 脚本（环境加载逻辑只写在 ps1 里，bat 不重复实现）--
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-tests.ps1"

echo.
echo Done. Press any key to exit.
pause >nul
