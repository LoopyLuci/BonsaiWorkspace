@echo off
REM Omnisystem Application Launcher
setlocal enabledelayedexpansion

REM Get the directory where this script is located (the root)
set ROOT_DIR=%~dp0
set OMNISYSTEM_DIR=%ROOT_DIR%Omnisystem

REM Run the PowerShell launcher
powershell -NoExit -ExecutionPolicy Bypass -File "%OMNISYSTEM_DIR%\Omnisystem.Launcher.ps1"

exit /b !ERRORLEVEL!
