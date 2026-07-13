@echo off
REM Omnisystem Launcher - Desktop GUI Application (DEBUG VERSION)
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
echo Current directory: %cd%

REM Step 1: Check npm
echo.
echo [1] Checking Node.js...
npm --version
if errorlevel 1 (
    echo [ERROR] npm not found. Install Node.js from https://nodejs.org
    pause
    exit /b 1
)
echo [OK] npm is installed

REM Step 2: Check if node_modules exists
echo.
echo [2] Checking dependencies...
if exist "node_modules" (
    echo [OK] Dependencies already installed
) else (
    echo Installing dependencies...
    call npm install --legacy-peer-deps
    echo [OK] Dependencies installed
)

REM Step 3: Build frontend
echo.
echo [3] Building Svelte frontend...
if exist "dist" (
    echo [OK] Frontend already built
) else (
    echo Building...
    call npm run build
    if exist "dist" (
        echo [OK] Frontend built
    ) else (
        echo [ERROR] Build failed
        pause
        exit /b 1
    )
)

REM Step 4: Copy to web root
echo.
echo [4] Setting up web server...
if not exist "C:\Launcher\www" mkdir C:\Launcher\www
echo Copying dist to C:\Launcher\www
xcopy /Y /E /I "dist\*" "C:\Launcher\www\"
echo [OK] Frontend copied

REM Step 5: Check if launcher-web.exe exists
echo.
echo [5] Checking launcher-web.exe...
if exist "C:\Launcher\launcher-web.exe" (
    echo [OK] Found launcher-web.exe
) else (
    echo [ERROR] launcher-web.exe not found at C:\Launcher\launcher-web.exe
    pause
    exit /b 1
)

REM Step 6: Launch web server
echo.
echo ========================================
echo  LAUNCHER READY
echo ========================================
echo.
echo Opening launcher at http://localhost:8080
echo.

REM Start web server
echo Starting launcher-web.exe...
start "Omnisystem Launcher" /B "C:\Launcher\launcher-web.exe" --port 8080 --host 127.0.0.1

REM Wait for server to start
timeout /t 2 /nobreak

REM Open browser
echo Opening browser...
start http://localhost:8080

REM Keep window open
echo.
echo Server is running. Press any key to close this window.
pause

REM Kill the server
taskkill /F /IM launcher-web.exe
