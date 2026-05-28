<#
.SYNOPSIS
    Build the Bonsai Survival Watchdog binary.

.DESCRIPTION
    Compiles bonsai-watchdog as a standalone release binary and copies it to
    target\release\ alongside the main app so Launch-BonsaiWorkspace.ps1 can
    find and start it automatically.

.PARAMETER Release
    Build in release mode (default). Pass -Release:$false for a dev build.

.EXAMPLE
    .\Build-Watchdog.ps1
    .\Build-Watchdog.ps1 -Release:$false
#>
param(
    [switch]$Release = $true
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ScriptDir  = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Manifest   = Join-Path $ScriptDir 'crates\bonsai-watchdog\Cargo.toml'
$OutDir     = Join-Path $ScriptDir 'target\release'

if (-not (Test-Path $Manifest)) {
    Write-Host "[watchdog] ERROR: manifest not found at $Manifest" -ForegroundColor Red
    exit 1
}

$cargoArgs = @('build', '--manifest-path', $Manifest)
if ($Release) { $cargoArgs += '--release' }

Write-Host "[watchdog] Building bonsai-watchdog ($( if ($Release) { 'release' } else { 'debug' } ))…" -ForegroundColor Cyan
& cargo @cargoArgs
if ($LASTEXITCODE -ne 0) {
    Write-Host "[watchdog] Build failed (exit $LASTEXITCODE)" -ForegroundColor Red
    exit $LASTEXITCODE
}

# Copy to workspace-level target/release for the launch script.
$profile     = if ($Release) { 'release' } else { 'debug' }
$builtExe    = Join-Path $ScriptDir "crates\bonsai-watchdog\target\$profile\bonsai-watchdog.exe"
$destExe     = Join-Path $OutDir 'bonsai-watchdog.exe'

if (Test-Path $builtExe) {
    $null = New-Item -ItemType Directory -Force -Path $OutDir
    Copy-Item -Path $builtExe -Destination $destExe -Force
    $sizeMb = [Math]::Round((Get-Item $destExe).Length / 1MB, 2)
    Write-Host "[watchdog] Binary ready: $destExe ($sizeMb MB)" -ForegroundColor Green
} else {
    Write-Host "[watchdog] WARNING: built exe not found at $builtExe" -ForegroundColor Yellow
}
