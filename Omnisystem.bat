@echo off
REM Omnisystem Application Launcher
REM Launches the built GUI executable from Omnisystem/launchers/
setlocal enabledelayedexpansion

REM Get the directory where this script is located (project root)
set ROOT_DIR=%~dp0
set LAUNCHERS_DIR=%ROOT_DIR%Omnisystem\launchers
set GUI_EXE=%LAUNCHERS_DIR%\Omnisystem.exe

REM Check if GUI executable exists
if not exist "%GUI_EXE%" (
    echo Omnisystem.exe not found at: %GUI_EXE%
    echo.
    echo Please build the executable first:
    echo   cd Omnisystem\scripts\build
    echo   .\Build-All.ps1
    echo.
    pause
    exit /b 1
)

REM Run the GUI launcher
"%GUI_EXE%"

exit /b !ERRORLEVEL!
