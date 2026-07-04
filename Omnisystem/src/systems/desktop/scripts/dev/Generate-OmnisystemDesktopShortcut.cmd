@echo off
setlocal
set SCRIPT_DIR=%~dp0
powershell -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%Generate-OmnisystemDesktopShortcut.ps1" -Force %*
exit /b %ERRORLEVEL%
