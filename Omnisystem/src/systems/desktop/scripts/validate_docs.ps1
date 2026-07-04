# Validate Rustdoc Coverage
#
# This script runs cargo doc and checks for missing documentation warnings.
# Enforces that all public APIs have /// doc comments.
#
# Usage: .\scripts\validate_docs.ps1

$ErrorActionPreference = "Stop"

Write-Host "📖 Validating rustdoc coverage..." -ForegroundColor Cyan
Write-Host ""

# Check if cargo is available
$CargoPath = (Get-Command cargo -ErrorAction SilentlyContinue).Source
if (-not $CargoPath) {
    Write-Host "❌ cargo not found. Please install Rust." -ForegroundColor Red
    exit 1
}

Write-Host "Running: cargo doc --no-deps --document-private-items --all" -ForegroundColor Gray

# Run cargo doc and capture output
$DocOutput = cargo doc --no-deps --document-private-items --all 2>&1
$DocExitCode = $LASTEXITCODE

Write-Host ""

# Check for warnings
$WarningCount = ($DocOutput | Select-String -Pattern "warning:" | Measure-Object).Count
$ErrorCount = ($DocOutput | Select-String -Pattern "^error" | Measure-Object).Count

if ($ErrorCount -gt 0) {
    Write-Host "❌ Documentation build failed with $ErrorCount error(s):" -ForegroundColor Red
    Write-Host ""
    $DocOutput | Select-String -Pattern "^error"
    Write-Host ""
    exit 1
}

if ($WarningCount -gt 0) {
    Write-Host "⚠️  Found $WarningCount documentation warning(s):" -ForegroundColor Yellow
    Write-Host ""
    $DocOutput | Select-String -Pattern "warning:"
    Write-Host ""
    Write-Host "Run the following to fix:" -ForegroundColor Gray
    Write-Host "  cargo fix --allow-dirty --workspace" -ForegroundColor Gray
    Write-Host ""
    exit 1
}

Write-Host "✅ All public APIs have documentation." -ForegroundColor Green
Write-Host "   Documentation built successfully." -ForegroundColor Gray
Write-Host ""
Write-Host "Generated docs in: target/doc/" -ForegroundColor Gray

exit 0
