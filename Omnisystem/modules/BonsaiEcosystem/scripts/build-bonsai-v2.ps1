#!/usr/bin/env pwsh
$ErrorActionPreference = "Stop"

Write-Host "Building BonsAI V2 ecosystem..." -ForegroundColor Cyan

$crates = @(
    "bonsai-bat",
    "bonsai-moe",
    "bonsai-kef",
    "bonsai-tdl",
    "bonsai-safety",
    "bonsai-package"
)

foreach ($crate in $crates) {
    Write-Host "Building $crate..." -ForegroundColor Yellow
    cargo build --release -p $crate
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to build $crate" -ForegroundColor Red
        exit 1
    }
}

Write-Host "All crates built successfully" -ForegroundColor Green
Write-Host "Running tests..." -ForegroundColor Cyan
cargo test --workspace --release
Write-Host "BonsAI V2 ecosystem ready" -ForegroundColor Green
