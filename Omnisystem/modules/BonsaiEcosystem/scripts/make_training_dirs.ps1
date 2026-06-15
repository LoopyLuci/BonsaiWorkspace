<#
.SYNOPSIS
    Create all ~/.bonsai training directories needed by the pipeline.
    Safe to run multiple times. Run once before Phase 1.
#>
$dirs = @(
    "$env:USERPROFILE\.bonsai\training_export",
    "$env:USERPROFILE\.bonsai\adapters",
    "$env:USERPROFILE\.bonsai\models",
    "$env:USERPROFILE\.bonsai\logs"
)
foreach ($d in $dirs) {
    New-Item -ItemType Directory -Force -Path $d | Out-Null
    Write-Host "[setup] $d"
}
Write-Host "[setup] Done."
