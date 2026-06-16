# ============================================================================
# QUICK BUILD - Build BonsaiEcosystem Desktop using Omnisystem Native Code
# ============================================================================
# Usage: .\Quick-Build.ps1              (builds desktop, forces rebuild)
#        .\Quick-Build.ps1 -Release     (release build)
#        .\Quick-Build.ps1 -NoClean     (use cache, don't force rebuild)

param(
    [switch]$Release = $false,
    [ValidateSet("all", "desktop", "gui", "launcher")]
    [string]$Target = "all",
    [switch]$NoClean = $false
)

$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BuildScript = Join-Path $RootDir "Build-Launchers.ps1"

if (-not (Test-Path $BuildScript)) {
    Write-Host "✗ Build-Launchers.ps1 not found" -ForegroundColor Red
    exit 1
}

Write-Host "Building BonsaiEcosystem Omnisystem..." -ForegroundColor Cyan

$BuildArgs = @{
    Target = $Target
    Clean = if ($NoClean) { $false } else { $true }  # Force clean by default
}

if ($Release) {
    $BuildArgs["Release"] = $true
}

& $BuildScript @BuildArgs
