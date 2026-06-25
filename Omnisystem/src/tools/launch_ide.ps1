$env:PATH += ";$env:USERPROFILE\.cargo\bin"
Write-Host "🌲 Omni Studio IDE v0.3.0" -ForegroundColor Green
Write-Host "============================" -ForegroundColor Green
Write-Host ""
Write-Host "[1/3] Compiler: Titan self-hosted" -ForegroundColor Cyan
Write-Host "[2/3] Runtime: OmniCore + Aether actors" -ForegroundColor Cyan  
Write-Host "[3/3] IDE: Omni Studio terminal environment" -ForegroundColor Cyan
Write-Host ""
cargo run --release --manifest-path titan-bootstrap/Cargo.toml -- --ide
