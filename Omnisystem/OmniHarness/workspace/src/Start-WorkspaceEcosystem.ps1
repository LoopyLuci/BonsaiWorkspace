#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Launches the complete Workspace Ecosystem with a single command.

.DESCRIPTION
    Orchestrates the startup of the full Workspace Ecosystem:
    - Tauri desktop application
    - workspace-bot messaging service
    - Preflight validation (dependencies, ports, tools)
    - Health confirmation and optional USB testing

.PARAMETER Mode
    Launch mode: 'desktop' (default) or 'desktop+usb' for Android testing

.PARAMETER StrictApp
    Require successful package install/launch for USB checks (desktop+usb mode)

.PARAMETER NoTests
    Skip USB regression tests (desktop+usb mode)

.PARAMETER PreflightOnly
    Run validation checks only; do not launch services

.PARAMETER ApiPort
    API port to wait for health on (default: 11369)

.PARAMETER Serial
    Android device serial for USB testing

.PARAMETER ApkPath
    Path to APK for strict app launch testing

.PARAMETER Fast
    Fast repeat-launch mode (skips npm install check)

.PARAMETER RemoteSurfaceSmoke
    Run remote-surface fallback/trampoline smoke tests (desktop+usb mode)

.EXAMPLE
    # Launch full ecosystem
    .\Start-WorkspaceEcosystem.ps1

.EXAMPLE
    # Launch with Android testing
    .\Start-WorkspaceEcosystem.ps1 -Mode desktop+usb

.EXAMPLE
    # Validate setup without launching
    .\Start-WorkspaceEcosystem.ps1 -PreflightOnly

.NOTES
    See orchestrate-workspace-ecosystem.mjs for full documentation.
#>

param(
    [ValidateSet('desktop', 'desktop+usb')]
    [string]$Mode = 'desktop',

    [switch]$StrictApp,
    [switch]$NoTests,
    [switch]$PreflightOnly,
    [switch]$Fast,
    [switch]$RemoteSurfaceSmoke,

    [int]$ApiPort = 11369,
    [string]$Serial,
    [string]$ApkPath
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommandPath
$OrchestratorScript = Join-Path $ScriptDir 'orchestrate-workspace-ecosystem.mjs'

if (-not (Test-Path $OrchestratorScript)) {
    Write-Error "Orchestrator script not found: $OrchestratorScript"
    exit 1
}

# Build arguments for the orchestrator
$Arguments = @($OrchestratorScript, '--mode', $Mode)

if ($StrictApp) { $Arguments += '--strict-app' }
if ($NoTests) { $Arguments += '--no-tests' }
if ($PreflightOnly) { $Arguments += '--preflight-only' }
if ($Fast) { $Arguments += '--fast' }
if ($RemoteSurfaceSmoke) { $Arguments += '--remote-surface-smoke' }
if ($ApiPort -ne 11369) { $Arguments += '--api-port', $ApiPort }
if ($Serial) { $Arguments += '--serial', $Serial }
if ($ApkPath) { $Arguments += '--apk-path', $ApkPath }

# Launch the orchestrator
Write-Host "🚀 Starting Workspace Ecosystem (mode: $Mode)..." -ForegroundColor Cyan
& node @Arguments
exit $LASTEXITCODE
