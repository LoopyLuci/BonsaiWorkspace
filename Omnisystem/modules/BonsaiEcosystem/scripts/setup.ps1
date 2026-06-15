$ErrorActionPreference = "Stop"
Write-Host "Setting up Bonsai Ecosystem..." -ForegroundColor Cyan

if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    throw "Rust is missing. Install rustup first."
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js is missing. Install Node 20+ first."
}
if (-not (Get-Command python -ErrorAction SilentlyContinue)) {
    throw "Python is missing. Install Python 3.11+ first."
}

New-Item -ItemType Directory -Force -Path target, dist, logs, manifests | Out-Null

if (Test-Path requirements.txt) {
    python -m pip install -r requirements.txt
}

cargo build -p bonsai-cli --release
Write-Host "Setup complete." -ForegroundColor Green
