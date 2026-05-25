#!/usr/bin/env pwsh
# launch-dev.ps1 — Start Vite dev server then the Bonsai binary.
# Use this instead of running the binary directly — the debug binary loads
# the UI from http://127.0.0.1:1420 (Vite), not from dist/.

param(
    [string]$BinaryPath = "Z:\Projects\BonsaiWorkspace\target\debug\bonsai-workspace.exe",
    [int]$VitePort = 1420,
    [int]$ViteTimeoutSec = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$SrcDir = Split-Path $PSScriptRoot -Parent   # bonsai-workspace/src

# 1. Kill any stale sidecars from previous sessions
Write-Host "[launch] Sweeping stale sidecars..." -ForegroundColor Cyan
Get-CimInstance Win32_Process | Where-Object {
    $_.ExecutablePath -like "*llama-server*" -or
    $_.ExecutablePath -like "*piper.exe*" -or
    $_.ExecutablePath -like "*bonsai-workspace.exe*"
} | ForEach-Object {
    Write-Host "  killing PID $($_.ProcessId): $($_.Name)"
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Seconds 1

# 2. Start Vite dev server if not already running
$viteRunning = $false
try {
    $null = Invoke-WebRequest "http://127.0.0.1:$VitePort" -TimeoutSec 1 -UseBasicParsing
    $viteRunning = $true
    Write-Host "[launch] Vite already running on port $VitePort" -ForegroundColor Green
} catch {}

if (-not $viteRunning) {
    Write-Host "[launch] Starting Vite dev server on port $VitePort..." -ForegroundColor Cyan
    $viteProc = Start-Process -FilePath "npm" `
        -ArgumentList "run", "dev" `
        -WorkingDirectory $SrcDir `
        -PassThru -WindowStyle Hidden

    # Wait for Vite to become ready
    $deadline = [DateTime]::Now.AddSeconds($ViteTimeoutSec)
    $ready = $false
    while ([DateTime]::Now -lt $deadline) {
        Start-Sleep -Milliseconds 400
        try {
            $null = Invoke-WebRequest "http://127.0.0.1:$VitePort" -TimeoutSec 1 -UseBasicParsing
            $ready = $true
            break
        } catch {}
    }
    if (-not $ready) {
        Write-Error "[launch] Vite did not start within ${ViteTimeoutSec}s — aborting"
        Stop-Process -Id $viteProc.Id -Force -ErrorAction SilentlyContinue
        exit 1
    }
    Write-Host "[launch] Vite ready on port $VitePort" -ForegroundColor Green
}

# 3. Reset api_port to preferred value so the backend always claims 11369
$cfgPath = "$env:APPDATA\com.bonsai.workspace\bonsai-config.json"
if (Test-Path $cfgPath) {
    $cfg = Get-Content $cfgPath | ConvertFrom-Json
    $cfg.api_port = 11369
    $cfg.buddy_api_port = 11420
    $cfg | ConvertTo-Json -Depth 10 | Set-Content $cfgPath
    Write-Host "[launch] Reset api_port=11369 in config" -ForegroundColor Cyan
}

# 4. Launch the binary
Write-Host "[launch] Starting Bonsai Workspace..." -ForegroundColor Cyan
$app = Start-Process -FilePath $BinaryPath -PassThru -WindowStyle Normal

# 5. Wait for backend to become healthy
$apiDeadline = [DateTime]::Now.AddSeconds(30)
$apiReady = $false
while ([DateTime]::Now -lt $apiDeadline) {
    Start-Sleep -Milliseconds 500
    try {
        $r = Invoke-WebRequest "http://127.0.0.1:11369/health" -TimeoutSec 1 -UseBasicParsing
        if ($r.StatusCode -eq 200) { $apiReady = $true; break }
    } catch {}
    # Also check bumped port in case 11369 is still in TIME_WAIT
    try {
        $cfg2 = Get-Content $cfgPath | ConvertFrom-Json
        if ($cfg2.api_port -ne 11369) {
            $r2 = Invoke-WebRequest "http://127.0.0.1:$($cfg2.api_port)/health" -TimeoutSec 1 -UseBasicParsing
            if ($r2.StatusCode -eq 200) { $apiReady = $true; break }
        }
    } catch {}
}

if ($apiReady) {
    Write-Host "[launch] Backend healthy. Bonsai Workspace is running." -ForegroundColor Green
} else {
    Write-Warning "[launch] Backend did not become healthy within 30s — check logs"
}

Write-Host "[launch] Press Ctrl+C to stop all processes" -ForegroundColor Yellow
try { Wait-Process -Id $app.Id } catch {}
