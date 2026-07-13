# Omnisystem Launcher GUI - PowerShell Launcher
# Run this to start the desktop application

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " OMNISYSTEM LAUNCHER" -ForegroundColor Cyan
Write-Host " Building and launching..." -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

Write-Host "Working directory: $((Get-Location).Path)" -ForegroundColor Gray
Write-Host ""

# Step 1: Check npm
Write-Host "[1] Checking Node.js..." -ForegroundColor Cyan
try {
    $npmVersion = npm --version 2>$null
    Write-Host "    [OK] npm v$npmVersion" -ForegroundColor Green
} catch {
    Write-Host "    [ERROR] npm not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "Node.js is required. Install from: https://nodejs.org" -ForegroundColor Yellow
    Read-Host "Press Enter to exit"
    exit 1
}

# Step 2: Install dependencies
Write-Host ""
Write-Host "[2] Installing dependencies..." -ForegroundColor Cyan
if (Test-Path "node_modules") {
    Write-Host "    [OK] Already installed" -ForegroundColor Green
} else {
    Write-Host "    Installing..." -ForegroundColor Gray
    npm install --legacy-peer-deps
    Write-Host "    [OK] Installed" -ForegroundColor Green
}

# Step 3: Build frontend
Write-Host ""
Write-Host "[3] Building Svelte frontend..." -ForegroundColor Cyan
if (Test-Path "dist") {
    Write-Host "    [OK] Already built" -ForegroundColor Green
} else {
    Write-Host "    Building..." -ForegroundColor Gray
    npm run build
    if (-not (Test-Path "dist")) {
        Write-Host "    [ERROR] Build failed" -ForegroundColor Red
        Read-Host "Press Enter to exit"
        exit 1
    }
    Write-Host "    [OK] Built" -ForegroundColor Green
}

# Step 4: Copy to web root
Write-Host ""
Write-Host "[4] Setting up web server..." -ForegroundColor Cyan
$webRoot = "C:\Launcher\www"
if (-not (Test-Path $webRoot)) {
    New-Item -ItemType Directory -Path $webRoot -Force | Out-Null
}
Copy-Item -Path "dist\*" -Destination $webRoot -Force -Recurse
Write-Host "    [OK] Frontend copied to $webRoot" -ForegroundColor Green

# Step 5: Check launcher-web.exe
Write-Host ""
Write-Host "[5] Checking launcher-web.exe..." -ForegroundColor Cyan
$launcherExe = "C:\Launcher\launcher-web.exe"
if (Test-Path $launcherExe) {
    $size = [math]::Round((Get-Item $launcherExe).Length / 1MB, 1)
    Write-Host "    [OK] Found ($size MB)" -ForegroundColor Green
} else {
    Write-Host "    [ERROR] Not found at $launcherExe" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

# Step 6: Launch web server
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host " LAUNCHER READY" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Starting launcher-web.exe..." -ForegroundColor Cyan
Write-Host "URL: http://localhost:8080" -ForegroundColor Yellow
Write-Host ""

# Start web server in new window
Start-Process -FilePath $launcherExe -ArgumentList "--port 8080 --host 127.0.0.1" -WindowStyle Hidden

# Wait for server
Start-Sleep -Seconds 2

# Open browser
Write-Host "Opening browser..." -ForegroundColor Gray
Start-Process "http://localhost:8080"

Write-Host ""
Write-Host "Desktop GUI is launching!" -ForegroundColor Green
Write-Host "Close this window or press Ctrl+C to stop the server." -ForegroundColor Yellow
Write-Host ""

# Keep the script running
try {
    while ($true) {
        Start-Sleep -Seconds 1
    }
} catch {
    # When Ctrl+C is pressed
    Write-Host ""
    Write-Host "Stopping launcher..." -ForegroundColor Yellow
    Stop-Process -Name "launcher-web" -Force -ErrorAction SilentlyContinue
    Write-Host "Done!" -ForegroundColor Green
    exit 0
}
