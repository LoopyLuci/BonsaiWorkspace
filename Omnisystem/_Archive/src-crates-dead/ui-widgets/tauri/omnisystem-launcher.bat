@echo off
REM Omnisystem Launcher - Desktop GUI Executable
REM This is the standalone launcher that starts the application

setlocal enabledelayedexpansion

REM Get the directory where this script is located
set "SCRIPT_DIR=%~dp0"

REM Check if npm is available
where /q npm
if errorlevel 1 (
    echo.
    echo ============================================================
    echo  ERROR: Node.js is not installed
    echo ============================================================
    echo.
    echo npm is required to run this application.
    echo Please install Node.js from https://nodejs.org
    echo.
    pause
    exit /b 1
)

echo.
echo ============================================================
echo  OMNISYSTEM LAUNCHER
echo  Native Desktop Application
echo ============================================================
echo.

REM Change to script directory
cd /d "!SCRIPT_DIR!"

REM Install dependencies if node_modules doesn't exist
if not exist "node_modules" (
    echo [*] Installing dependencies (this may take a minute)...
    call npm install --legacy-peer-deps
    echo.
)

REM Check if dist folder exists, if not build it
if not exist "dist" (
    echo [*] Building application...
    call npm run build
    echo.
)

echo [*] Launching application...
echo.
echo The window should open automatically in 2 seconds.
echo.

REM Wait 2 seconds then try to open browser
timeout /t 2 /nobreak > nul

REM Try to open default browser (optional)
start http://localhost:5173 2>nul

REM Run the dev server
call npm run tauri:dev

pause
