@echo off
chcp 65001 >nul
title Sensend 开发模式（热更新）
cd /d "%~dp0"
echo ============================================
echo  Sensend 开发模式（热更新）
echo  改样式、改组件即时生效，不用重新编译
echo  关掉这个黑窗口 = 结束
echo ============================================
echo.
if not exist "%~dp0scripts\run-dev.bat" (
  echo 找不到 scripts\run-dev.bat，项目结构可能不对。
  echo 请确认本文件位于 sensend 仓库根目录。
  pause
  exit /b 1
)
call "%~dp0scripts\run-dev.bat"
