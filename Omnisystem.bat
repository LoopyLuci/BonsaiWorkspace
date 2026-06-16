@echo off
REM Omnisystem v2.5.0 - Enterprise Application Manager
REM Launches the complete Omnisystem GUI App Menu

setlocal enabledelayedexpansion
cd /d "%~dp0"

title Omnisystem v2.5.0 - Enterprise Application Manager
color 0B

REM Launch the GUI App Menu
"%~dp0Omnisystem_CLI.exe" gui

REM Keep window open
pause
