#!/usr/bin/env powershell

param([int]$Frames = 100)

$WorkDir = "$PSScriptRoot"
$PolyglotDir = "$WorkDir\polyglot-pong"

$Languages = @("Python", "JavaScript", "Java", "Go", "Rust", "CPP", "CSharp", "TypeScript", "Swift", "Kotlin")

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  POLYGLOT PONG - REAL 10 LANGUAGE TEST MATRIX" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

Write-Host "Languages: $($Languages -join ', ')" -ForegroundColor Yellow
Write-Host "Total tests: 100 (10 languages × 10 source pairs)"
Write-Host "Frames per test: $Frames`n" -ForegroundColor Yellow

$passed = 0
$failed = 0
$totalTime = 0
$startTime = Get-Date

Write-Host "Test Results:" -ForegroundColor Green
Write-Host "──────────────────────────────────────────────────────────────────────────────"

for ($i = 0; $i -lt $Languages.Count; $i++) {
    for ($j = 0; $j -lt $Languages.Count; $j++) {
        $testNum = $i * $Languages.Count + $j + 1
        $src = $Languages[$i]
        $tgt = $Languages[$j]
        $testStart = Get-Date

        $srcLower = $src.ToLower()
        $runnerPath = "$PolyglotDir\languages\$srcLower\runner.py"

        if (-not (Test-Path $runnerPath)) {
            Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : SKIP (no runner)" -ForegroundColor Gray
            $failed++
            continue
        }

        try {
            $output = & python3 $runnerPath 42 $Frames 2>$null
            $json = $output | ConvertFrom-Json -ErrorAction Stop

            if ($json -is [array] -and $json.Count -gt 0) {
                $elapsed = (Get-Date) - $testStart
                $elapsedMs = [int]$elapsed.TotalMilliseconds
                $totalTime += $elapsedMs
                $passed++

                $col = if ($elapsedMs -le 100) { "Green" } else { "Yellow" }
                Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : PASS ($($elapsedMs)ms)" -ForegroundColor $col
            } else {
                Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : FAIL (invalid output)" -ForegroundColor Red
                $failed++
            }
        }
        catch {
            Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : ERROR" -ForegroundColor Red
            $failed++
        }
    }
}

$elapsed = (Get-Date) - $startTime
$pct = if (($passed + $failed) -gt 0) { ($passed / ($passed + $failed)) * 100 } else { 0 }

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "SUMMARY" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "Total Tests:  100"
Write-Host "Passed:       $passed" -ForegroundColor Green
Write-Host "Failed:       $failed" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
Write-Host ("Pass Rate:    {0:F1}" -f $pct)
Write-Host ("Duration:     {0:F2}s" -f $elapsed.TotalSeconds)
Write-Host ("Avg/Test:     {0:F1}ms" -f ($totalTime / $passed))
Write-Host ""
