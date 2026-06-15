# Check for Private Names – CI/CD Automation (PowerShell)
#
# This script verifies that no private or internal model names appear
# anywhere in the public repository (source code, documentation, config).
#
# Usage: .\scripts\check_no_private_names.ps1
#
# Private Names (MUST NOT APPEAR):
#   - Psychopathy Octopus (use: Custom Octopus AI, Server-Specific Model)
#   - Guardrail (use: Safety Model, Internal Research Model)
#   - Flowers (use: Fine-Tuned Model, User-Specific LoRA)

param(
    [switch]$Fix = $false
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir

Write-Host "🔍 Checking for private names in repository..." -ForegroundColor Cyan
Write-Host "Repository root: $RepoRoot" -ForegroundColor Gray
Write-Host ""

# Private names to check (case-insensitive)
$PrivateNames = @("Psychopathy", "Guardrail", "Flowers")

# File patterns to check
$FilePatterns = @("*.md", "*.rs", "*.toml", "*.yaml", "*.yml", "*.json", "*.sh", "*.ps1", "*.py")

$Found = $false
$Matches = @()

# Search for each private name
foreach ($name in $PrivateNames) {
    Write-Host "Checking for '$name'..."

    # Find all matching files
    foreach ($pattern in $FilePatterns) {
        $Files = Get-ChildItem -Path $RepoRoot -Recurse -Filter $pattern -File -ErrorAction SilentlyContinue | `
            Where-Object { $_.FullName -notlike "*\.git*" -and $_.FullName -notlike "*target*" }

        foreach ($file in $Files) {
            try {
                $Content = Get-Content -Path $file.FullName -ErrorAction SilentlyContinue
                if ($Content -and ($Content -match [regex]::Escape($name))) {
                    $Found = $true

                    # Find line numbers
                    $LineNum = 0
                    foreach ($line in $Content) {
                        $LineNum++
                        if ($line -match [regex]::Escape($name)) {
                            $Match = "$($file.FullName):$LineNum`: $line"
                            $Matches += $Match
                            Write-Host "  ❌ $Match" -ForegroundColor Red
                        }
                    }
                }
            }
            catch {
                # Skip binary files and files we can't read
            }
        }
    }
}

Write-Host ""

if (-not $Found) {
    Write-Host "✅ No private names found. Repository is clean." -ForegroundColor Green
    exit 0
}
else {
    Write-Host "❌ FAILED: Found $($Matches.Count) reference(s) to private names." -ForegroundColor Red
    Write-Host ""
    Write-Host "Private names MUST NOT appear in public code/docs. Replace with:" -ForegroundColor Yellow
    Write-Host "  - 'Psychopathy Octopus' → 'Custom Octopus AI' or 'Server-Specific Model'"
    Write-Host "  - 'Guardrail' → 'Safety Model' or 'Internal Research Model'"
    Write-Host "  - 'Flowers' → 'Fine-Tuned Model' or 'User-Specific LoRA'"
    Write-Host ""

    if ($Fix) {
        Write-Host "🔧 Attempting automated fixes..." -ForegroundColor Yellow
        Write-Host "   (Automated fixes not implemented; please replace manually)" -ForegroundColor Yellow
        exit 1
    }
    else {
        Write-Host "Run with -Fix flag to attempt automated replacement (not implemented yet)." -ForegroundColor Gray
        exit 1
    }
}
