Write-Host "Running BonsAI multimodal verification checks..."

# Check models directory
$models = Join-Path $env:USERPROFILE ".bonsai\models"
if (-Not (Test-Path $models)) {
    Write-Host "Models directory not found: $models" -ForegroundColor Yellow
    exit 1
}

Write-Host "Listing models:"
Get-ChildItem $models | ForEach-Object { Write-Host $_.Name }

Write-Host "Done. You should run: `cargo test -p bonsai-workspace` and start the app to exercise tools."