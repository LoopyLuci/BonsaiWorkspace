#!/usr/bin/env pwsh
# launch.ps1 — One-click Bonsai Workspace launcher.
# The debug binary loads its UI from the Vite dev server (port 1420).
# This script starts Vite first, waits until ready, then starts the binary.
#
# Usage: .\launch.ps1
#        .\launch.ps1 -VitePort 1420 -BinaryPath "path\to\bonsai-workspace.exe"

param(
    [string]$BinaryPath  = "Z:\Projects\BonsaiWorkspace\target\debug\bonsai-workspace.exe",
    [int]   $VitePort    = 1420,
    [int]   $ApiPort     = 11369,
    [int]   $BuddyPort   = 11420
)

Set-StrictMode -Version Latest

$SrcDir  = "$PSScriptRoot\bonsai-workspace\src"
$CfgPath = "$env:APPDATA\com.bonsai.workspace\bonsai-config.json"

Write-Host "=== Bonsai Workspace Launcher ===" -ForegroundColor Cyan

# ── 1. Sweep stale processes ──────────────────────────────────────────────────
Write-Host "[1/4] Sweeping stale sidecars..." -ForegroundColor Yellow
Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ExecutablePath -like "*llama-server*" -or
    $_.ExecutablePath -like "*piper.exe" -or
    $_.ExecutablePath -like "*bonsai-workspace.exe"
} | ForEach-Object {
    Write-Host "  killing PID $($_.ProcessId) ($($_.Name))" -ForegroundColor DarkGray
    Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue
}
Start-Sleep -Milliseconds 800

# ── 2. Start Vite dev server if not already running ──────────────────────────
Write-Host "[2/4] Starting Vite dev server (port $VitePort)..." -ForegroundColor Yellow
$viteAlready = $false
try { $null = Invoke-WebRequest "http://127.0.0.1:$VitePort" -TimeoutSec 1 -UseBasicParsing; $viteAlready = $true } catch {}

if (-not $viteAlready) {
    $viteJob = Start-Job -ScriptBlock {
        param($dir) Set-Location $dir; npm run dev 2>&1
    } -ArgumentList $SrcDir

    $deadline = [DateTime]::Now.AddSeconds(30)
    while ([DateTime]::Now -lt $deadline) {
        Start-Sleep -Milliseconds 500
        try { $null = Invoke-WebRequest "http://127.0.0.1:$VitePort" -TimeoutSec 1 -UseBasicParsing; $viteAlready = $true; break } catch {}
    }
    if (-not $viteAlready) {
        Write-Error "Vite did not start within 30s. Check node/npm are in PATH."
        Receive-Job $viteJob | Write-Host
        exit 1
    }
}
Write-Host "  Vite ready on http://127.0.0.1:$VitePort" -ForegroundColor Green

# ── 3. Reset config to preferred ports ───────────────────────────────────────
Write-Host "[3/4] Setting preferred ports in config..." -ForegroundColor Yellow
if (Test-Path $CfgPath) {
    $cfg = Get-Content $CfgPath | ConvertFrom-Json
    $cfg.api_port       = $ApiPort
    $cfg.buddy_api_port = $BuddyPort
    $cfg | ConvertTo-Json -Depth 10 | Set-Content $CfgPath
}

# ── 4. Start the binary ───────────────────────────────────────────────────────
Write-Host "[4/4] Starting Bonsai Workspace binary..." -ForegroundColor Yellow
$app = Start-Process -FilePath $BinaryPath -PassThru -WindowStyle Normal

$apiDeadline = [DateTime]::Now.AddSeconds(30)
$apiReady = $false
while ([DateTime]::Now -lt $apiDeadline) {
    Start-Sleep -Milliseconds 500
    try {
        $r = Invoke-WebRequest "http://127.0.0.1:$ApiPort/health" -TimeoutSec 1 -UseBasicParsing
        if ($r.StatusCode -eq 200) { $apiReady = $true; break }
    } catch {}
}

if ($apiReady) {
    Write-Host ""
    Write-Host "✓ Bonsai Workspace is running" -ForegroundColor Green
    Write-Host "  UI:      http://127.0.0.1:$VitePort  (WebView)" -ForegroundColor DarkGray
    Write-Host "  API:     http://127.0.0.1:$ApiPort" -ForegroundColor DarkGray
    Write-Host "  Buddy:   http://127.0.0.1:$BuddyPort" -ForegroundColor DarkGray
} else {
    Write-Warning "Backend did not respond on port $ApiPort within 30s — check logs."
}

Write-Host ""
Write-Host "Press Ctrl+C or close this window to stop." -ForegroundColor DarkYellow
try { Wait-Process -Id $app.Id -ErrorAction SilentlyContinue } catch {}
