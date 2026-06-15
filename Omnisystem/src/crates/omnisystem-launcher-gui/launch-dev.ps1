# Omnisystem Launcher Desktop App - Development Mode
# This script launches the Tauri development server with hot reload

Write-Host "`n╔══════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   OMNISYSTEM LAUNCHER - DESKTOP GUI         ║" -ForegroundColor Cyan
Write-Host "║   Development Mode (Hot Reload Enabled)     ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════╝`n" -ForegroundColor Cyan

Write-Host "📦 Ensuring dependencies are installed..." -ForegroundColor Gray
npm install --legacy-peer-deps

Write-Host "`n🚀 Launching Tauri development server...`n" -ForegroundColor Green
Write-Host "The window should open automatically." -ForegroundColor Gray
Write-Host "Press Ctrl+C to stop the server.`n" -ForegroundColor Gray

npx tauri dev

Write-Host "`nPress any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
