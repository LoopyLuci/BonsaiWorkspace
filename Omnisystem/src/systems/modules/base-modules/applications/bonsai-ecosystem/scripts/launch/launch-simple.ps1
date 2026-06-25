#!/usr/bin/env pwsh
# launch.ps1 — One-click Bonsai Workspace launcher.
# Sweeps stale sidecars, resets port config, then starts the binary.
# The binary was compiled with custom-protocol so it loads the UI from
# the embedded dist/ — no Vite dev server required.

param(
    [string]$BinaryPath = "Z:\Projects\BonsaiWorkspace\target\debug\bonsai-workspace.exe",
    [int]   $ApiPort    = 11369,
    [int]   $BuddyPort  = 11420
)

$CfgPath = "$env:APPDATA\com.bonsai.workspace\bonsai-config.json"

Write-Host "=== Bonsai Workspace ===" -ForegroundColor Cyan

# 1. Kill stale sidecars
Write-Host "Sweeping stale processes..." -ForegroundColor Yellow
Get-CimInstance Win32_Process -EA SilentlyContinue | Where-Object {
    $_.ExecutablePath -like "*llama-server*" -or
    $_.ExecutablePath -like "*piper.exe" -or
    $_.ExecutablePath -like "*bonsai-workspace.exe"
} | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -EA SilentlyContinue }
Start-Sleep -Milliseconds 600

# 2. Reset preferred ports
if (Test-Path $CfgPath) {
    $cfg = Get-Content $CfgPath | ConvertFrom-Json
    $cfg.api_port       = $ApiPort
    $cfg.buddy_api_port = $BuddyPort
    $cfg | ConvertTo-Json -Depth 10 | Set-Content $CfgPath
}

# 3. Start
Write-Host "Starting Bonsai Workspace..." -ForegroundColor Yellow
$app = Start-Process -FilePath $BinaryPath -PassThru -WindowStyle Normal

# 4. Wait for backend
$ok = $false
$deadline = [DateTime]::Now.AddSeconds(25)
while ([DateTime]::Now -lt $deadline) {
    Start-Sleep -Milliseconds 400
    try {
        $r = Invoke-WebRequest "http://127.0.0.1:$ApiPort/health" -TimeoutSec 1 -UseBasicParsing
        if ($r.StatusCode -eq 200) { $ok = $true; break }
    } catch {}
}

if ($ok) {
    Write-Host "Ready — API: http://127.0.0.1:$ApiPort" -ForegroundColor Green
} else {
    Write-Warning "Backend did not respond within 25s"
}

try { Wait-Process -Id $app.Id -EA SilentlyContinue } catch {}
