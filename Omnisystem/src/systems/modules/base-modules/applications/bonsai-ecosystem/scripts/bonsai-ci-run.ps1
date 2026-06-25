#!/usr/bin/env pwsh
# Bonsai Native CI/CD Runner - Replaces GitHub Actions
# Orchestrates parallel testing across all BEDF teams

param(
    [string]$Mode = "full",           # full, quick, integration
    [int]$ParallelJobs = 8,           # Number of parallel jobs
    [switch]$NoRetry = $false,        # Disable retry on failure
    [switch]$DashboardOnly = $false   # Show results only
)

$ErrorActionPreference = "Stop"

Write-Host "🚀 Bonsai CI/CD Runner - BEDF Orchestration" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# Team definitions
$teams = @(
    @{ id = "A"; name = "Fuzzing"; crate = "bonsai-bedf-fuzzing" },
    @{ id = "B"; name = "Concurrency"; crate = "bonsai-bedf-concurrency" },
    @{ id = "C"; name = "Sanitizers"; crate = "bonsai-bedf-sanitizers" },
    @{ id = "D"; name = "Property"; crate = "bonsai-bedf-property" },
    @{ id = "E"; name = "Pentest"; crate = "bonsai-bedf-pentest" },
    @{ id = "F"; name = "Sandbox"; crate = "bonsai-bedf-sandbox" },
    @{ id = "G"; name = "Triage"; crate = "bonsai-bedf-triage" },
    @{ id = "H"; name = "MCP"; crate = "bonsai-bedf-mcp" },
    @{ id = "I"; name = "Enhancements"; crate = "bonsai-bedf-enhancements" },
    @{ id = "J"; name = "Survival"; crate = "bonsai-survival-system-ext" },
    @{ id = "K"; name = "KDB"; crate = "bonsai-kdb-ext" }
)

# Results tracking
$results = @{}
$startTime = Get-Date

Write-Host "📋 Pipeline Mode: $Mode" -ForegroundColor Yellow
Write-Host "⚙️  Parallel Jobs: $ParallelJobs" -ForegroundColor Yellow
Write-Host "🔄 Retry Enabled: $(-not $NoRetry)" -ForegroundColor Yellow
Write-Host ""

# Stage 1: Build all teams in parallel
Write-Host "📦 STAGE 1: Building all teams..." -ForegroundColor Cyan
Write-Host ""

$buildJobs = @()
foreach ($team in $teams) {
    $job = Start-Job -ScriptBlock {
        param($team)
        $output = cargo build --package $team.crate --release 2>&1
        @{
            team = $team.id
            crate = $team.crate
            status = if ($LASTEXITCODE -eq 0) { "✅ PASSED" } else { "❌ FAILED" }
            time = (Measure-Command { $output }).TotalSeconds
        }
    } -ArgumentList $team -Name "build_$($team.id)"
    $buildJobs += $job
}

$buildResults = @()
foreach ($job in $buildJobs) {
    $result = Wait-Job -Job $job | Receive-Job
    $buildResults += $result
    Write-Host "Team $($result.team): $($result.status) ($($result.time.ToString('F2'))s)" -ForegroundColor $(if ($result.status -like "*PASSED*") { "Green" } else { "Red" })
}

Write-Host ""
$buildPassed = ($buildResults | Where-Object { $_.status -like "*PASSED*" }).Count
Write-Host "Build Results: $buildPassed/$($teams.Count) passed" -ForegroundColor $(if ($buildPassed -eq $teams.Count) { "Green" } else { "Yellow" })
Write-Host ""

# Stage 2: Run tests
if ($Mode -ne "dashboard") {
    Write-Host "🧪 STAGE 2: Running tests..." -ForegroundColor Cyan
    Write-Host ""

    $testJobs = @()
    foreach ($team in $teams) {
        $job = Start-Job -ScriptBlock {
            param($team)
            $output = cargo test --package $team.crate --release 2>&1
            @{
                team = $team.id
                crate = $team.crate
                status = if ($LASTEXITCODE -eq 0) { "✅ PASSED" } else { "❌ FAILED" }
                time = (Measure-Command { $output }).TotalSeconds
            }
        } -ArgumentList $team -Name "test_$($team.id)"
        $testJobs += $job
    }

    $testResults = @()
    foreach ($job in $testJobs) {
        $result = Wait-Job -Job $job | Receive-Job
        $testResults += $result
        Write-Host "Team $($result.team): $($result.status) ($($result.time.ToString('F2'))s)" -ForegroundColor $(if ($result.status -like "*PASSED*") { "Green" } else { "Red" })
    }

    Write-Host ""
    $testPassed = ($testResults | Where-Object { $_.status -like "*PASSED*" }).Count
    Write-Host "Test Results: $testPassed/$($teams.Count) passed" -ForegroundColor $(if ($testPassed -eq $teams.Count) { "Green" } else { "Yellow" })
    Write-Host ""
}

# Stage 3: Lint
if ($Mode -ne "dashboard") {
    Write-Host "🔍 STAGE 3: Linting..." -ForegroundColor Cyan
    Write-Host ""

    $lintJobs = @()
    foreach ($team in $teams) {
        $job = Start-Job -ScriptBlock {
            param($team)
            $output = cargo clippy --package $team.crate -- -D warnings 2>&1
            @{
                team = $team.id
                crate = $team.crate
                status = if ($LASTEXITCODE -eq 0) { "✅ PASSED" } else { "⚠️  WARNINGS" }
                time = (Measure-Command { $output }).TotalSeconds
            }
        } -ArgumentList $team -Name "lint_$($team.id)"
        $lintJobs += $job
    }

    $lintResults = @()
    foreach ($job in $lintJobs) {
        $result = Wait-Job -Job $job | Receive-Job
        $lintResults += $result
        Write-Host "Team $($result.team): $($result.status)" -ForegroundColor $(if ($result.status -like "*PASSED*") { "Green" } else { "Yellow" })
    }

    Write-Host ""
}

# Final Summary
$duration = (Get-Date) - $startTime
Write-Host ""
Write-Host "╔════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║         CI/CD PIPELINE SUMMARY         ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "Total Teams: $($teams.Count)" -ForegroundColor White
Write-Host "Build Passed: $buildPassed/$($teams.Count)" -ForegroundColor $(if ($buildPassed -eq $teams.Count) { "Green" } else { "Red" })
Write-Host "Tests Passed: $testPassed/$($teams.Count)" -ForegroundColor $(if ($testPassed -eq $teams.Count) { "Green" } else { "Red" })
Write-Host "Duration: $([math]::Round($duration.TotalMinutes, 2))m" -ForegroundColor Cyan
Write-Host ""

$allPassed = ($buildPassed -eq $teams.Count) -and ($testPassed -eq $teams.Count)

if ($allPassed) {
    Write-Host "✅ PIPELINE PASSED" -ForegroundColor Green
    Write-Host ""
    Write-Host "All teams built and tested successfully!" -ForegroundColor Green
    Write-Host "Ready for production deployment." -ForegroundColor Green
    exit 0
} else {
    Write-Host "❌ PIPELINE FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Some teams did not pass. Review failures above." -ForegroundColor Red
    exit 1
}
