@echo off
REM Omnisystem v2.5.0 - Enterprise Application Manager
REM Launches the native TITAN GUI App Menu

setlocal enabledelayedexpansion
cd /d "%~dp0"

REM Launch the native TITAN GUI framework
cd /d "%~dp0Omnisystem\languages\titan"
call "%~dp0Omnisystem\titan_compiler\target\release\titan.exe" run OmnisystemGUI_v2.ti

exit /b 0
