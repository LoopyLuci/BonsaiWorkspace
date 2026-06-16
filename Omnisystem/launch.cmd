@echo off
REM Omnisystem Launcher
setlocal enabledelayedexpansion

powershell -NoExit -ExecutionPolicy Bypass -File "%~dp0Omnisystem.Launcher.ps1" %*
