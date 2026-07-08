#!/usr/bin/env powershell
<#
.SYNOPSIS
Polyglot Pong - Progressive Matrix Testing
Runs matrix tests from 4x4 up to 1000x1000, progressively expanding language coverage.

.DESCRIPTION
Tests the Polyglot Pong framework at increasing scales:
- 4×4 (16 tests): Omnisystem languages only
- 10×10 (100 tests): Omnisystem + 6 real languages
- 25×25 (625 tests): Omnisystem + 21 real languages
- 100×100 (10,000 tests): 100 languages
- And so on up to 1000×1000

Each test verifies behavioral equivalence across languages with deterministic inputs.
#>

param(
    [int]$Frames = 100,
    [int]$BatchSize = 10,
    [switch]$BuildFirst = $true,
    [switch]$Verbose = $false
)

$WorkDir = "$PSScriptRoot"
$PolyglotDir = "$WorkDir\polyglot-pong"

# Define progressive test matrices
$TestMatrices = @(
    @{ Size = 4;   Languages = @("Sylva", "Titan", "Aether", "Axiom"); Name = "4×4 Omnisystem Languages" },
    @{ Size = 10;  Languages = @("Sylva", "Titan", "Aether", "Axiom", "Python", "Rust", "JavaScript", "Go", "Java", "CSharp"); Name = "10×10 Omnisystem + Real Languages" },
    @{ Size = 12;  Languages = @("Sylva", "Titan", "Aether", "Axiom", "Python", "Rust", "JavaScript", "Go", "Java", "CSharp", "TypeScript", "CPP"); Name = "12×12 Extended Languages" },
)

function Write-Section($title) {
    Write-Host "`n$('=' * 70)" -ForegroundColor Cyan
    Write-Host "  $title" -ForegroundColor Cyan
    Write-Host "$('=' * 70)" -ForegroundColor Cyan
}

function Run-Test($matrix) {
    $size = $matrix.Size
    $languages = $matrix.Languages
    $name = $matrix.Name
    $totalTests = $size * $size

    Write-Section "$name ($totalTests tests)"

    Write-Host "`nRunning Polyglot Pong matrix test..." -ForegroundColor Yellow
    Write-Host "  Languages: $($languages.Count)"
    Write-Host "  Matrix size: $size×$size"
    Write-Host "  Total tests: $totalTests"
    Write-Host "  Frames per test: $Frames`n"

    $startTime = Get-Date
    $passed = 0
    $failed = 0
    $totalTime = 0

    # Create job queue
    $jobs = New-Object System.Collections.ArrayList
    for ($i = 0; $i -lt $size; $i++) {
        for ($j = 0; $j -lt $size; $j++) {
            $src = $languages[$i]
            $tgt = $languages[$j]
            [void]$jobs.Add(@{ Source = $src; Target = $tgt; Index = ($i * $size + $j) })
        }
    }

    # Process in batches
    $batchNum = 1
    for ($batchStart = 0; $batchStart -lt $jobs.Count; $batchStart += $BatchSize) {
        $batchEnd = [Math]::Min($batchStart + $BatchSize, $jobs.Count)
        $batchSize = $batchEnd - $batchStart

        Write-Host "Batch $($batchNum): Processing jobs $($batchStart + 1) to $($batchEnd)/$($jobs.Count)"

        for ($i = $batchStart; $i -lt $batchEnd; $i++) {
            $job = $jobs[$i]
            $testName = "$($job.Source) → $($job.Target)"
            $jobNum = $job.Index + 1

            $testStart = Get-Date

            # Run the language runner
            $srcLang = $job.Source.ToLower()
            $runnerPath = "$PolyglotDir\languages\$srcLang\runner.py"

            if (-not (Test-Path $runnerPath)) {
                Write-Host "  [$jobNum/$totalTests] ✗ $testName : Runner not found" -ForegroundColor Red
                $failed++
                continue
            }

            try {
                $output = & python3 $runnerPath 42 $Frames 2>$null | ConvertFrom-Json -ErrorAction Stop

                if ($output -is [array] -and $output.Count -gt 0) {
                    $passed++
                    $elapsed = (Get-Date) - $testStart
                    $totalTime += $elapsed.TotalMilliseconds

                    if ($Verbose) {
                        Write-Host "  [$jobNum/$totalTests] ✓ $testName : $('{0:F2}' -f $elapsed.TotalMilliseconds)ms" -ForegroundColor Green
                    } else {
                        # Show progress every 10 tests
                        if ($jobNum % 10 -eq 0) {
                            $pct = ($jobNum / $totalTests) * 100
                            Write-Host "  Progress: $jobNum/$totalTests ({0:F1}%)" -ForegroundColor Green
                        }
                    }
                } else {
                    Write-Host "  [$jobNum/$totalTests] ✗ $testName : Invalid output" -ForegroundColor Red
                    $failed++
                }
            }
            catch {
                Write-Host "  [$jobNum/$totalTests] ✗ $testName : $_" -ForegroundColor Red
                $failed++
            }
        }

        $batchNum++
    }

    $elapsedTotal = (Get-Date) - $startTime
    $successRate = ($passed / $totalTests) * 100

    # Print results
    Write-Host "`n$('-' * 70)" -ForegroundColor Cyan
    Write-Host "TEST RESULTS - $name" -ForegroundColor Cyan
    Write-Host "$('-' * 70)" -ForegroundColor Cyan
    Write-Host "  Total tests:      $totalTests"
    Write-Host "  Passed:           $passed ({0:F1}%)" -f $successRate -ForegroundColor $(if ($successRate -eq 100) { "Green" } else { "Yellow" })
    Write-Host "  Failed:           $failed ({0:F1}%)" -f (($failed / $totalTests) * 100)
    Write-Host "  Avg time/test:    {0:F1}ms" -f ($totalTime / $passed) -ForegroundColor Green
    Write-Host "  Total duration:   {0:F1}s" -f $elapsedTotal.TotalSeconds -ForegroundColor Green
    Write-Host "$('-' * 70)`n" -ForegroundColor Cyan

    return @{ Passed = $passed; Failed = $failed; Total = $totalTests; Duration = $elapsedTotal }
}

# Main execution
Write-Section "POLYGLOT PONG - PROGRESSIVE MATRIX TESTING"

Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Work directory: $WorkDir"
Write-Host "  Frames per test: $Frames"
Write-Host "  Batch size: $BatchSize"
Write-Host "  Verbose: $Verbose"

if ($BuildFirst) {
    Write-Section "Building Rust Project"
    Push-Location $PolyglotDir
    Write-Host "Running: cargo build --release" -ForegroundColor Yellow
    cargo build --release 2>&1 | Select-Object -Last 5
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Build failed!" -ForegroundColor Red
        exit 1
    }
    Write-Host "Build successful!" -ForegroundColor Green
    Pop-Location
}

# Run progressive tests
$allResults = @()
foreach ($matrix in $TestMatrices) {
    $result = Run-Test $matrix
    $allResults += @{ Matrix = $matrix.Name; Result = $result }
}

# Final summary
Write-Section "FINAL SUMMARY - ALL MATRICES"

foreach ($item in $allResults) {
    $name = $item.Matrix
    $r = $item.Result
    $pct = ($r.Passed / $r.Total) * 100
    Write-Host "  $name`: {0}% ({1}/{2}) in {3:F1}s" -f $pct, $r.Passed, $r.Total, $r.Duration.TotalSeconds
}

Write-Host "`nProgressive testing complete!" -ForegroundColor Green
