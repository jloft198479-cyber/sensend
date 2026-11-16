@echo off
chcp 65001 >nul
cls
echo ============================================
echo  Sensend Tests ^| cargo test
echo  no window, no process kill
echo ============================================
echo.

REM -- delegate to ps1 (env loading lives there only) --
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-tests.ps1"

echo.
echo Done. Press any key to exit.
pause >nul
