# Migration script for Phase 4 - Replace external dependencies with Omnisystem components

param(
    [string]$CratePattern = "*",
    [switch]$DryRun = $false,
    [switch]$Verbose = $false
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CratesDir = Join-Path $ScriptDir "crates"

$MigrationMap = @{
    'tokio = \{ workspace = true' = 'omnisystem-async-runtime = { path = "../omnisystem-async-runtime" }'
    'serde = \{ workspace = true' = 'omnisystem-serialization = { path = "../omnisystem-serialization" }'
    'dashmap = \{ workspace = true' = 'omnisystem-collections = { path = "../omnisystem-collections" }'
    'tracing = \{ workspace = true' = 'omnisystem-observability = { path = "../omnisystem-observability" }'
}

$RemoveDeps = @(
    'thiserror = \{ workspace = true'
    'serde_json = \{ workspace = true'
    'uuid = \{ workspace = true'
    'chrono = \{ workspace = true'
)

function Update-CrateToml {
    param(
        [string]$TomlPath,
        [hashtable]$ReplaceMap,
        [array]$RemoveList,
        [bool]$DryRun
    )

    if (-not (Test-Path $TomlPath)) {
        return $false
    }

    $content = Get-Content $TomlPath -Raw
    $originalContent = $content
    $changed = $false

    # Check if this crate uses any of our target dependencies
    $hasTargetDeps = $false
    foreach ($key in $ReplaceMap.Keys) {
        if ($content -match $key) {
            $hasTargetDeps = $true
            break
        }
    }

    if (-not $hasTargetDeps) {
        return $false
    }

    # Replace dependencies
    foreach ($key in $ReplaceMap.Keys) {
        if ($content -match $key) {
            $content = $content -replace $key, $ReplaceMap[$key]
            $changed = $true
        }
    }

    # Remove dependencies
    foreach ($dep in $RemoveList) {
        if ($content -match $dep) {
            $content = $content -replace "$dep[^\n]*\n", ""
            $changed = $true
        }
    }

    # Clean up empty dev-dependencies sections
    $content = $content -replace '\[dev-dependencies\]\s*\n\s*\n', "[dev-dependencies]`n`n"

    if ($changed -and -not $DryRun) {
        Set-Content -Path $TomlPath -Value $content -Force
        return $true
    }

    return $changed
}

function Get-CrateName {
    param([string]$Path)
    $toml = Get-Content $Path -Raw
    if ($toml -match 'name\s*=\s*"([^"]+)"') {
        return $matches[1]
    }
    return Split-Path (Split-Path $Path) -Leaf
}

# Main execution
Write-Host "Phase 4 Dependency Migration Script" -ForegroundColor Cyan
Write-Host "Pattern: $CratePattern" -ForegroundColor Gray
if ($DryRun) {
    Write-Host "Mode: DRY RUN (no changes will be made)" -ForegroundColor Yellow
}
Write-Host ""

$migratedCount = 0
$skippedCount = 0
$totalCount = 0

Get-ChildItem -Path $CratesDir -Filter $CratePattern -Directory | ForEach-Object {
    $cratePath = $_.FullName
    $tomlPath = Join-Path $cratePath "Cargo.toml"

    $totalCount++

    if (Update-CrateToml -TomlPath $tomlPath -ReplaceMap $MigrationMap -RemoveList $RemoveDeps -DryRun $DryRun) {
        $crateName = Get-CrateName -Path $tomlPath
        Write-Host "✓ Updated: $crateName" -ForegroundColor Green
        $migratedCount++
    } else {
        $skippedCount++
    }
}

Write-Host ""
Write-Host "Migration Summary:" -ForegroundColor Cyan
Write-Host "  Crates processed: $totalCount"
Write-Host "  Crates migrated: $migratedCount" -ForegroundColor Green
Write-Host "  Crates skipped: $skippedCount" -ForegroundColor Gray

if ($DryRun) {
    Write-Host ""
    Write-Host "This was a DRY RUN. Run without -DryRun to apply changes." -ForegroundColor Yellow
} else {
    Write-Host ""
    Write-Host "Migration complete! Run 'cargo check --all' to verify." -ForegroundColor Green
}
