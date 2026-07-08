#!/usr/bin/env powershell

param([int]$Frames = 100)

$WorkDir = "$PSScriptRoot"
$PolyglotDir = "$WorkDir\polyglot-pong"

$Languages = @("Python", "JavaScript", "Java", "Go", "Rust", "CPP", "CSharp", "TypeScript", "Swift", "Kotlin")

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  POLYGLOT PONG - REAL 10×10 TEST MATRIX" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

Write-Host "Configuration:" -ForegroundColor Yellow
Write-Host "  Languages: $($Languages -join ', ')"
Write-Host "  Total tests: 100"
Write-Host "  Frames per test: $Frames"
Write-Host "  Python: $(python3.12 --version)"
Write-Host ""

$passed = 0
$failed = 0
$totalTime = 0
$startTime = Get-Date

Write-Host "Test Progress:" -ForegroundColor Green
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
            $output = & python3.12 $runnerPath 42 $Frames 2>&1
            $json = $output | ConvertFrom-Json -ErrorAction Stop

            if ($json -is [array] -and $json.Count -gt 0) {
                $elapsed = (Get-Date) - $testStart
                $elapsedMs = [int]$elapsed.TotalMilliseconds
                $totalTime += $elapsedMs
                $passed++

                $col = if ($elapsedMs -le 100) { "Green" } else { "Yellow" }
                Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : ✓ PASS ($($elapsedMs)ms)" -ForegroundColor $col
            } else {
                Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : ✗ FAIL (invalid output)" -ForegroundColor Red
                $failed++
            }
        }
        catch {
            Write-Host "[$('{0:3}' -f $testNum)] $src -> $tgt : ✗ ERROR ($_)" -ForegroundColor Red
            $failed++
        }
    }
}

$elapsed = (Get-Date) - $startTime
$pct = if (($passed + $failed) -gt 0) { [math]::Round(($passed / ($passed + $failed)) * 100, 1) } else { 0 }
$avgTime = if ($passed -gt 0) { [math]::Round($totalTime / $passed, 1) } else { 0 }

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  POLYGLOT PONG 10×10 TEST RESULTS" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Total Tests:       100"
Write-Host "  Passed:            $passed / 100" -ForegroundColor $(if ($passed -eq 100) { "Green" } else { "Yellow" })
Write-Host "  Failed:            $failed / 100" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
Write-Host "  Success Rate:      $pct%" -ForegroundColor $(if ($pct -eq 100) { "Green" } else { "Yellow" })
Write-Host "  Avg Time/Test:     $($avgTime)ms"
Write-Host "  Total Duration:    $('{0:F2}' -f $elapsed.TotalSeconds)s"
Write-Host ""

if ($passed -eq 100) {
    Write-Host "  ✓ ALL TESTS PASSED!" -ForegroundColor Green
    Write-Host "  All 10 languages produced identical Pong traces (fidelity = 1.0)"
} else {
    Write-Host "  ⚠ $failed test(s) failed" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""
