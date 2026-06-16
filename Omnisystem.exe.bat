@echo off
REM Omnisystem v2.5.0 - Enterprise Application Manager
REM Launches the complete Omnisystem App Menu (Beautiful Graphical GUI)

setlocal enabledelayedexpansion
cd /d "%~dp0"

REM Launch the web-based graphical GUI in default browser
start "" "%~dp0AppMenu.html"

exit /b 0
