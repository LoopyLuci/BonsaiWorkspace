# Omnisystem Launcher - Desktop GUI
# Run this script to launch the application

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " OMNISYSTEM LAUNCHER" -ForegroundColor Cyan
Write-Host " Native Desktop Application" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get script directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Check if npm is available
try {
    $npmVersion = npm --version 2>$null
    Write-Host "[OK] npm found: v$npmVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] npm not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "Node.js is required. Install from: https://nodejs.org"
    Write-Host ""
    Read-Host "Press Enter to exit"
    exit 1
}

# Change to script directory
Set-Location $scriptDir

# Install dependencies if needed
if (-not (Test-Path "node_modules")) {
    Write-Host ""
    Write-Host "[*] Installing dependencies..." -ForegroundColor Yellow
    npm install --legacy-peer-deps
    Write-Host ""
}

# Build frontend if needed
if (-not (Test-Path "dist")) {
    Write-Host "[*] Building application..." -ForegroundColor Yellow
    npm run build
    Write-Host ""
}

# Launch the app
Write-Host "[*] Launching application..." -ForegroundColor Yellow
Write-Host ""
Write-Host "The window should open in a few seconds." -ForegroundColor Gray
Write-Host ""

# Start dev server
npm run tauri:dev

Read-Host "Press Enter to exit"
