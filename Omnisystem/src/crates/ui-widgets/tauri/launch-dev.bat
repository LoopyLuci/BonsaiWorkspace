@echo off
REM Omnisystem Launcher Desktop App - Development Mode
REM This script launches the Tauri development server with hot reload

echo.
echo ╔══════════════════════════════════════════════╗
echo ║   OMNISYSTEM LAUNCHER - DESKTOP GUI         ║
echo ║   Development Mode (Hot Reload Enabled)     ║
echo ╚══════════════════════════════════════════════╝
echo.

echo 📦 Ensuring dependencies are installed...
call npm install --legacy-peer-deps

echo.
echo 🚀 Launching Tauri development server...
echo.
echo The window should open automatically.
echo Press Ctrl+C to stop the server.
echo.

call npx tauri dev

pause
