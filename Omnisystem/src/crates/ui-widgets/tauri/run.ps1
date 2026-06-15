# Omnisystem Launcher GUI - Web-based Version
# Uses the compiled launcher-web.exe to serve the Svelte frontend

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " OMNISYSTEM LAUNCHER" -ForegroundColor Cyan
Write-Host " Building and launching..." -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Step 1: Check dependencies
Write-Host "[1] Checking Node.js..." -ForegroundColor Cyan
try {
    $npmVersion = npm --version 2>$null
    Write-Host "    [OK] npm v$npmVersion" -ForegroundColor Green
} catch {
    Write-Host "    [ERROR] npm not found" -ForegroundColor Red
    exit 1
}

# Step 2: Install npm dependencies
Write-Host ""
Write-Host "[2] Installing dependencies..." -ForegroundColor Cyan
if (-not (Test-Path "node_modules")) {
    npm install --legacy-peer-deps 2>&1 | Out-Null
    Write-Host "    [OK] Dependencies installed" -ForegroundColor Green
} else {
    Write-Host "    [OK] Dependencies already installed" -ForegroundColor Green
}

# Step 3: Build frontend
Write-Host ""
Write-Host "[3] Building Svelte frontend..." -ForegroundColor Cyan
npm run build 2>&1 | Out-Null
if (Test-Path "dist") {
    Write-Host "    [OK] Frontend built to dist/" -ForegroundColor Green
} else {
    Write-Host "    [ERROR] Build failed" -ForegroundColor Red
    exit 1
}

# Step 4: Check for launcher-web executable
Write-Host ""
Write-Host "[4] Checking launcher-web.exe..." -ForegroundColor Cyan
$launcherExe = "C:\Launcher\launcher-web.exe"
if (Test-Path $launcherExe) {
    Write-Host "    [OK] Found: $launcherExe" -ForegroundColor Green
} else {
    Write-Host "    [ERROR] launcher-web.exe not found" -ForegroundColor Red
    Write-Host "    Expected: $launcherExe" -ForegroundColor Yellow
    exit 1
}

# Step 5: Copy frontend to web root
Write-Host ""
Write-Host "[5] Setting up web server..." -ForegroundColor Cyan
$webRoot = "C:\Launcher\www"
if (-not (Test-Path $webRoot)) {
    New-Item -ItemType Directory -Path $webRoot -Force | Out-Null
}
Copy-Item -Path "dist\*" -Destination $webRoot -Force -Recurse
Write-Host "    [OK] Frontend copied to $webRoot" -ForegroundColor Green

# Step 6: Launch web server
Write-Host ""
Write-Host "[6] Starting web server..." -ForegroundColor Cyan
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host " LAUNCHER READY" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Opening: http://localhost:8080" -ForegroundColor Cyan
Write-Host ""
Write-Host "Press Ctrl+C in the launcher window to stop." -ForegroundColor Yellow
Write-Host ""

# Start web server in a new window
Start-Process -FilePath $launcherExe -ArgumentList "--port 8080 --host 127.0.0.1"

# Open browser
Start-Sleep -Milliseconds 500
Start-Process "http://localhost:8080"

Write-Host "Launcher started successfully!" -ForegroundColor Green
Read-Host "Press Enter to exit this script"
