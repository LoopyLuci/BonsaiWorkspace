# =============================================================================
# Bonsai Compilation Cache Setup
# Installs and configures sccache for lightning-fast incremental builds
# =============================================================================

Write-Host "⚡ Setting up Bonsai Compilation Cache..." -ForegroundColor Cyan
Write-Host ""

# Check if sccache is installed
$sccache = Get-Command sccache -ErrorAction SilentlyContinue

if (-not $sccache) {
    Write-Host "📦 Installing sccache..." -ForegroundColor Yellow
    cargo install sccache

    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Failed to install sccache" -ForegroundColor Red
        exit 1
    }
    Write-Host "✅ sccache installed successfully" -ForegroundColor Green
} else {
    Write-Host "✅ sccache already installed: $(sccache --version)" -ForegroundColor Green
}

Write-Host ""

# Configure sccache environment variables
Write-Host "🔧 Configuring sccache..." -ForegroundColor Cyan

$env:RUSTC_WRAPPER = "sccache"
$env:SCCACHE_CACHE_SIZE = "20G"
$env:SCCACHE_DIR = "$env:USERPROFILE\.sccache"

# Make these permanent by adding to user environment
[Environment]::SetEnvironmentVariable("RUSTC_WRAPPER", "sccache", "User")
[Environment]::SetEnvironmentVariable("SCCACHE_CACHE_SIZE", "20G", "User")
[Environment]::SetEnvironmentVariable("SCCACHE_DIR", "$env:USERPROFILE\.sccache", "User")

Write-Host "✅ Environment variables configured:" -ForegroundColor Green
Write-Host "   RUSTC_WRAPPER = sccache" -ForegroundColor Gray
Write-Host "   SCCACHE_CACHE_SIZE = 20G" -ForegroundColor Gray
Write-Host "   SCCACHE_DIR = $env:USERPROFILE\.sccache" -ForegroundColor Gray

Write-Host ""

# Show sccache status
Write-Host "📊 Current sccache status:" -ForegroundColor Cyan
sccache --show-stats

Write-Host ""
Write-Host "✅ Compilation cache setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "📝 Next steps:" -ForegroundColor Yellow
Write-Host "   1. Run: cd Z:\Projects\BonsaiWorkspace" -ForegroundColor Gray
Write-Host "   2. Run: .\build-and-run.ps1" -ForegroundColor Gray
Write-Host "   3. Subsequent builds will be dramatically faster (50-90% speedup)" -ForegroundColor Gray
Write-Host ""
