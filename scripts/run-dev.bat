@echo off
chcp 65001 >nul
cls
echo ============================================
echo  Sensend Dev Mode ^| tauri dev + HMR
echo  port 1420, changes apply instantly
echo ============================================
echo.

REM -- delegate to ps1 (env loading lives there only) --
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-dev.ps1"

echo.
echo Dev server stopped. Press any key to exit.
pause >nul
