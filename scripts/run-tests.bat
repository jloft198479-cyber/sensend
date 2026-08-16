@echo off

echo ============================================
echo  Sensend Test Runner
echo ============================================

REM -- load MSVC environment --
set VC_DIR=M:\VS\BuildTools\VC\Auxiliary\Build
if exist "%VC_DIR%\vcvars64.bat" (
    call "%VC_DIR%\vcvars64.bat" >nul 2>&1
)

REM -- set Rust environment --
set CARGO_HOME=M:\rust\.cargo
set RUSTUP_HOME=M:\rust\.rustup
set PATH=M:\rust\.cargo\bin;%PATH%

REM -- run tests --
cd /d F:\fzz-Project\sensend\sensend\src-tauri
call cargo test 2>&1

echo.
echo Done. Press any key to exit.
pause >nul