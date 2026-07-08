@echo off
REM Start-WorkspaceEcosystem.cmd
REM Launches the complete Workspace Ecosystem with a single command.
REM
REM Usage:
REM   Start-WorkspaceEcosystem.cmd [options]
REM
REM Options:
REM   --mode desktop|desktop+usb   Launch mode (default: desktop)
REM   --strict-app                 Require successful app install/launch (USB mode)
REM   --no-tests                   Skip USB regression tests
REM   --preflight-only             Run checks only; do not launch
REM   --api-port <port>            API port to wait for (default: 11369)
REM   --serial <adb-serial>        Android device serial for USB testing
REM   --apk-path <path>            APK path for app testing
REM   --fast                       Fast repeat-launch (skips npm install)
REM   --remote-surface-smoke       Run remote-surface smoke tests
REM
REM Examples:
REM   Start-WorkspaceEcosystem.cmd
REM   Start-WorkspaceEcosystem.cmd --mode desktop+usb
REM   Start-WorkspaceEcosystem.cmd --preflight-only

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "ORCHESTRATOR=%SCRIPT_DIR%orchestrate-workspace-ecosystem.mjs"

if not exist "%ORCHESTRATOR%" (
    echo Error: Orchestrator script not found: %ORCHESTRATOR%
    exit /b 1
)

echo.
echo Starting Workspace Ecosystem...
echo.

node "%ORCHESTRATOR%" %*
exit /b %ERRORLEVEL%
