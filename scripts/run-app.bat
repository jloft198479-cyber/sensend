@echo off
chcp 65001 >nul
cls
echo ============================================
echo  Sensend Build and Run ^| release exe window
echo  slow by design: full rebuild before launch
echo ============================================
echo.

REM -- delegate to ps1 (env loading lives there only) --
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-app.ps1"

echo.
echo Done. Press any key to exit.
pause >nul
