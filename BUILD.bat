@echo off
REM Quick launcher for Omnisystem.exe build

setlocal enabledelayedexpansion

cd /d "%~dp0"

echo.
echo ╔════════════════════════════════════════════════════════════╗
echo ║      OMNISYSTEM BUILD LAUNCHER - Create Omnisystem.exe     ║
echo ╚════════════════════════════════════════════════════════════╝
echo.

REM Check if PowerShell is available
where powershell >nul 2>&1
if errorlevel 1 (
    echo ERROR: PowerShell not found
    echo Please ensure PowerShell 5.0+ is installed
    exit /b 1
)

REM Parse command line arguments
set "RELEASE="
set "CLEAN="
set "LAUNCH="

:parse_args
if "%1"=="" goto run_build
if /i "%1"=="-release" (
    set "RELEASE=-Release"
    shift
    goto parse_args
)
if /i "%1"=="-clean" (
    set "CLEAN=-Clean"
    shift
    goto parse_args
)
if /i "%1"=="-launch" (
    set "LAUNCH=-Launch"
    shift
    goto parse_args
)
if /i "%1"=="/?" goto show_help
if /i "%1"=="--help" goto show_help
shift
goto parse_args

:show_help
echo.
echo USAGE:
echo   BUILD.bat [OPTIONS]
echo.
echo OPTIONS:
echo   -release   Build in release mode (optimized, faster)
echo   -clean     Clean build artifacts before building
echo   -launch    Launch Omnisystem.exe after successful build
echo.
echo EXAMPLES:
echo   BUILD.bat -release -launch
echo   BUILD.bat -clean -release
echo.
exit /b 0

:run_build
echo Launching build script...
echo.

powershell -NoProfile -ExecutionPolicy Bypass -Command "& '.\Build-Omnisystem-Complete.ps1' %RELEASE% %CLEAN% %LAUNCH%"

if errorlevel 1 (
    echo.
    echo ERROR: Build failed
    exit /b 1
)

echo.
echo ═══════════════════════════════════════════════════════════════
echo Build complete! Your Omnisystem.exe is ready.
echo.
echo Quick start:
echo   .\Omnisystem.exe gui              Launch Omnisystem GUI
echo.
pause
