# Fix script for Phase 4 migration TOML syntax errors

param(
    [switch]$DryRun = $false,
    [switch]$Verbose = $false
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CratesDir = Join-Path $ScriptDir "crates"

function Fix-CargoToml {
    param(
        [string]$TomlPath,
        [bool]$DryRun
    )

    if (-not (Test-Path $TomlPath)) {
        return @{ Success = $false; Reason = "File not found" }
    }

    $content = Get-Content $TomlPath -Raw
    $originalContent = $content
    $changed = $false

    # Check if this file has the broken syntax
    if (-not ($content -match "} }" -or $content -match "}, features")) {
        return @{ Success = $false; Reason = "No fixes needed" }
    }

    # Fix 1: Remove duplicate closing braces (} } → })
    if ($content -match "} }") {
        $content = $content -replace "} }", "}"
        $changed = $true
    }

    # Fix 2: Remove malformed dev-dependencies entries with old features syntax
    # Pattern: omnisystem-* = { path ... }, features = ["full"] }
    if ($content -match "omnisystem-.*= \{ path.*\}, features") {
        # Remove entire line with this pattern
        $content = $content -replace "omnisystem-.*= \{ path.*\}, features.*\n", ""
        $changed = $true
    }

    # Fix 3: Clean up empty dev-dependencies sections
    # Replace [dev-dependencies] followed by empty lines with just [dev-dependencies]
    $content = $content -replace "\[dev-dependencies\]\s*\n\s*\n", "[dev-dependencies]`n`n"

    if ($changed -and -not $DryRun) {
        # Backup original
        $backupPath = "$TomlPath.backup"
        Copy-Item -Path $TomlPath -Destination $backupPath -Force

        # Write fixed content
        Set-Content -Path $TomlPath -Value $content -Force

        return @{
            Success = $true
            Original = $backupPath
            Changes = @(
                "Removed duplicate closing braces"
                "Fixed malformed dev-dependencies"
                "Cleaned empty sections"
            )
        }
    }

    return @{
        Success = $changed
        Reason = "Would apply fixes"
    }
}

function Validate-TomlSyntax {
    param([string]$TomlPath)

    try {
        $content = Get-Content $TomlPath -Raw

        # Basic TOML validation
        $issues = @()

        # Check for obvious issues
        if ($content -match "} }") {
            $issues += "Duplicate closing braces found"
        }
        if ($content -match "}, features") {
            $issues += "Malformed features syntax found"
        }
        if ($content -match "\{\s*$") {
            $issues += "Unclosed brace at end of line"
        }

        # Check bracket balance (simple heuristic)
        $openCount = ($content | Select-String -Pattern "{" -AllMatches).Matches.Count
        $closeCount = ($content | Select-String -Pattern "}" -AllMatches).Matches.Count
        if ($openCount -ne $closeCount) {
            $issues += "Mismatched braces: $openCount opens, $closeCount closes"
        }

        if ($issues.Count -gt 0) {
            return @{ Valid = $false; Issues = $issues }
        }
        return @{ Valid = $true; Issues = @() }
    }
    catch {
        return @{ Valid = $false; Issues = @($_) }
    }
}

# Main execution
Write-Host "Phase 4 TOML Syntax Fix Script" -ForegroundColor Cyan
if ($DryRun) {
    Write-Host "Mode: DRY RUN (no changes will be made)" -ForegroundColor Yellow
}
Write-Host ""

$fixedCount = 0
$skippedCount = 0
$errorCount = 0
$totalCount = 0

Get-ChildItem -Path $CratesDir -Filter "*" -Directory | ForEach-Object {
    $cratePath = $_.FullName
    $tomlPath = Join-Path $cratePath "Cargo.toml"

    $totalCount++

    $preValidation = Validate-TomlSyntax -TomlPath $tomlPath

    $fixResult = Fix-CargoToml -TomlPath $tomlPath -DryRun $DryRun

    if ($fixResult.Success) {
        $postValidation = Validate-TomlSyntax -TomlPath $tomlPath

        if ($postValidation.Valid) {
            $crateName = (Get-Content $tomlPath -Raw | Select-String -Pattern 'name\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
            Write-Host "✓ Fixed: $crateName" -ForegroundColor Green
            $fixedCount++
        } else {
            $crateName = (Get-Content $tomlPath -Raw | Select-String -Pattern 'name\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value
            Write-Host "✗ Error: $crateName - Post-validation failed: $($postValidation.Issues -join ', ')" -ForegroundColor Red
            $errorCount++
        }
    } else {
        $skippedCount++
    }
}

Write-Host ""
Write-Host "Fix Summary:" -ForegroundColor Cyan
Write-Host "  Crates processed: $totalCount"
Write-Host "  Crates fixed: $fixedCount" -ForegroundColor Green
Write-Host "  Crates skipped: $skippedCount" -ForegroundColor Gray
Write-Host "  Crates with errors: $errorCount" -ForegroundColor Red

if ($DryRun) {
    Write-Host ""
    Write-Host "This was a DRY RUN. Run without -DryRun to apply fixes." -ForegroundColor Yellow
} else {
    if ($errorCount -eq 0) {
        Write-Host ""
        Write-Host "All fixes applied successfully! Run 'cargo check --all' to verify." -ForegroundColor Green
    } else {
        Write-Host ""
        Write-Host "Some files had errors. Check backups in respective crate directories." -ForegroundColor Yellow
    }
}
