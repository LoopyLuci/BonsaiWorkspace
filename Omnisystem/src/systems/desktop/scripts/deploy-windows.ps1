$ErrorActionPreference = "Stop"
Write-Host "Deploy Windows placeholder script" -ForegroundColor Cyan
cargo build --release -p omnisystem-workspace
Write-Host "Add code signing / packaging step here." -ForegroundColor Yellow
