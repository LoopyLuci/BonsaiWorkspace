#!/usr/bin/env pwsh
# START_UACS.ps1 - Launch Universal Agent Control System with Visual Mode + HITL

Write-Host "╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   🧠 Universal Agent Control System (Visual + HITL Mode)       ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

$BonsaiDir = Get-Location
Write-Host "`n📍 Working Directory: $BonsaiDir" -ForegroundColor Yellow

# Check if cargo exists
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "`n❌ Error: cargo not found. Please ensure Rust is installed." -ForegroundColor Red
    exit 1
}

Write-Host "`n🔨 Building mcp-server..." -ForegroundColor Cyan
cargo build -p mcp-server --release 2>&1 | Select-Object -Last 10
if ($LASTEXITCODE -ne 0) {
    Write-Host "`n❌ Build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "`n✅ Build successful!" -ForegroundColor Green

# Start in three separate terminal windows/tabs
Write-Host "`n📂 Opening three terminals for:" -ForegroundColor Yellow
Write-Host "   1️⃣  Terminal 1: UACS Server (Visual Mode + HITL)" -ForegroundColor Cyan
Write-Host "   2️⃣  Terminal 2: UACS Dashboard (Svelte)" -ForegroundColor Cyan
Write-Host "   3️⃣  Terminal 3: Browser (http://localhost:5173)" -ForegroundColor Cyan

# Terminal 1: UACS Server
Write-Host "`n[Terminal 1] Starting UACS Server..." -ForegroundColor Cyan
Start-Process pwsh -ArgumentList "-NoExit", "-Command", "cd '$BonsaiDir'; cargo run -p mcp-server -- visual --hitl-categories destructive,network --port 11426"

# Give the server a moment to start
Start-Sleep -Seconds 3

# Terminal 2: Dashboard
Write-Host "[Terminal 2] Starting Dashboard..." -ForegroundColor Cyan
Start-Process pwsh -ArgumentList "-NoExit", "-Command", "cd '$BonsaiDir/uacs-dashboard'; npm run dev"

# Terminal 3: Open browser
Write-Host "[Terminal 3] Opening dashboard in browser..." -ForegroundColor Cyan
Start-Sleep -Seconds 5
Start-Process "http://localhost:5173"

Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║ 🚀 UACS is starting!                                           ║" -ForegroundColor Green
Write-Host "╠════════════════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║                                                                ║" -ForegroundColor Green
Write-Host "║  Server:    http://127.0.0.1:11426                            ║" -ForegroundColor Green
Write-Host "║  Dashboard: http://localhost:5173                             ║" -ForegroundColor Green
Write-Host "║                                                                ║" -ForegroundColor Green
Write-Host "║  Configuration:                                                ║" -ForegroundColor Green
Write-Host "║  - Mode: Visual Agent Control                                 ║" -ForegroundColor Green
Write-Host "║  - HITL: ENABLED (destructive, network)                       ║" -ForegroundColor Green
Write-Host "║  - Status: Ready to connect agents                            ║" -ForegroundColor Green
Write-Host "║                                                                ║" -ForegroundColor Green
Write-Host "╠════════════════════════════════════════════════════════════════╣" -ForegroundColor Green
Write-Host "║ Next: Configure Claude and give it the self-improvement task  ║" -ForegroundColor Green
Write-Host "║ See: CLAUDE_SELF_IMPROVEMENT.md                               ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════════╝" -ForegroundColor Green

Write-Host "`n⏳ Keep all three terminals running. Press Ctrl+C to stop." -ForegroundColor Yellow
