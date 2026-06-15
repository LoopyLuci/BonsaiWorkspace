@echo off
REM Omnisystem Launcher - Desktop GUI Application
REM This script builds the frontend and launches the web server

setlocal enabledelayedexpansion

echo.
echo ========================================
echo  OMNISYSTEM LAUNCHER
echo  Building and launching...
echo ========================================
echo.

REM Change to script directory
cd /d "%~dp0"

REM Step 1: Check npm
echo [1] Checking Node.js...
npm --version >nul 2>&1
if errorlevel 1 (
    echo [ERROR] npm not found. Install Node.js from https://nodejs.org
    pause
    exit /b 1
)
echo [OK] npm is installed

REM Step 2: Install dependencies
echo.
echo [2] Installing npm dependencies...
if not exist "node_modules" (
    call npm install --legacy-peer-deps >nul 2>&1
)
echo [OK] Dependencies ready

REM Step 3: Build frontend
echo.
echo [3] Building Svelte frontend...
call npm run build >nul 2>&1
if not exist "dist" (
    echo [ERROR] Build failed
    pause
    exit /b 1
)
echo [OK] Frontend built

REM Step 4: Copy to web root
echo.
echo [4] Setting up web server...
if not exist "C:\Launcher\www" mkdir C:\Launcher\www
xcopy /Y /E /I "dist\*" "C:\Launcher\www\" >nul 2>&1
echo [OK] Frontend copied to C:\Launcher\www

REM Step 5: Launch web server
echo.
echo ========================================
echo  LAUNCHER READY
echo ========================================
echo.
echo Opening launcher at http://localhost:8080
echo.

REM Start web server
start "Omnisystem Launcher" /B "C:\Launcher\launcher-web.exe" --port 8080 --host 127.0.0.1

REM Wait for server to start
timeout /t 1 /nobreak >nul

REM Open browser
start http://localhost:8080

REM Keep window open
echo.
echo Server is running. Press any key to close this window.
pause >nul

REM Kill the server
taskkill /F /IM launcher-web.exe >nul 2>&1
