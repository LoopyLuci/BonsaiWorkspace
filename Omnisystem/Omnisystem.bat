@echo off
REM Omnisystem Launcher - Batch wrapper for PowerShell launcher
REM This file gets renamed to Omnisystem.exe by the build script

setlocal enabledelayedexpansion

REM Get the directory where this script is located
set SCRIPT_DIR=%~dp0

REM Run PowerShell launcher with proper encoding
powershell -NoExit -ExecutionPolicy Bypass -File "%SCRIPT_DIR%Omnisystem\Omnisystem.Launcher.ps1" %*

exit /b !ERRORLEVEL!
