@echo off
chcp 65001 >nul
title Sensend Dev Mode (Hot Reload)
cd /d "%~dp0"
echo ============================================
echo  Sensend Dev Mode ^| Hot Reload
echo  - edit CSS / components: changes apply instantly
echo  - close this window to stop
echo ============================================
echo.
if not exist "%~dp0scripts\run-dev.bat" (
  echo [ERROR] scripts\run-dev.bat not found.
  echo This file must stay in the repository root.
  pause
  exit /b 1
)
call "%~dp0scripts\run-dev.bat"
