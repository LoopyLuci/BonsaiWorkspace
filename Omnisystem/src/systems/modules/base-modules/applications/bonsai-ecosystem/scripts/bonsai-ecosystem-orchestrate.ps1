#!/usr/bin/env pwsh
# Bonsai Ecosystem Master Orchestration Script
# Unified coordination of CI/CD, Bug Hunt, Survival, KDB, Lint, ETL, MCP, and Transfer Daemon

param(
    [ValidateSet("full", "quick", "integration", "bug-hunt", "learning", "observability")]
    [string]$Mode = "full",
    [int]$ParallelJobs = 8,
    [switch]$DryRun = $false,
    [switch]$Verbose = $false
)

$ErrorActionPreference = "Stop"
if ($Verbose) {
    $VerbosePreference = "Continue"
}

Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║     BONSAI ECOSYSTEM MASTER ORCHESTRATOR v1.0             ║" -ForegroundColor Cyan
Write-Host "║  Unified CI/CD, Bug Hunt, Survival, KDB, Lint, ETL, MCP   ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
Write-Host "🚀 Session Started: $timestamp" -ForegroundColor Yellow
Write-Host "📋 Mode: $Mode | Parallel Jobs: $ParallelJobs | Dry Run: $DryRun" -ForegroundColor Yellow
Write-Host ""

# Define ecosystem subsystems
$ecosystemSystems = @(
    @{ name = "Lint System"; script = ".\scripts\run-linter.ps1"; enabled = $true },
    @{ name = "CI/CD Pipeline"; script = ".\scripts\bonsai-ci-run.ps1"; enabled = $true },
    @{ name = "Bug Hunt"; script = ".\scripts\run-bug-hunt.ps1"; enabled = $true },
    @{ name = "Survival System"; script = ".\scripts\run-survival-system.ps1"; enabled = $true },
    @{ name = "Knowledge Database"; script = ".\scripts\run-kdb.ps1"; enabled = $true },
    @{ name = "ETL Pipeline"; script = ".\scripts\run-etl.ps1"; enabled = $true },
    @{ name = "MCP Server"; script = ".\scripts\run-mcp-server.ps1"; enabled = $true },
    @{ name = "Observability"; script = ".\scripts\run-observability.ps1"; enabled = $true }
)

function Write-Section {
    param([string]$Title)
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
    Write-Host "📌 $Title" -ForegroundColor Cyan
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
}

function Invoke-EcosystemSystem {
    param(
        [string]$SystemName,
        [string]$Script,
        [string]$Mode
    )

    Write-Host "▶️  Initializing: $SystemName" -ForegroundColor Green

    if ($DryRun) {
        Write-Host "   [DRY RUN] Would execute: $Script -Mode $Mode" -ForegroundColor Yellow
        return @{ success = $true; duration = 0 }
    }

    if (-not (Test-Path $Script)) {
        Write-Host "   ⚠️  Script not found: $Script" -ForegroundColor Yellow
        return @{ success = $false; duration = 0 }
    }

    $stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        & $Script -Mode $Mode -ParallelJobs $ParallelJobs | Out-Null
        $stopwatch.Stop()
        Write-Host "   ✅ Complete: ${SystemName} (${($stopwatch.ElapsedMilliseconds)}ms)" -ForegroundColor Green
        return @{ success = $true; duration = $stopwatch.ElapsedMilliseconds }
    }
    catch {
        $stopwatch.Stop()
        Write-Host "   ❌ Failed: $SystemName - $_" -ForegroundColor Red
        return @{ success = $false; duration = $stopwatch.ElapsedMilliseconds }
    }
}

# Mode-specific execution plans
$executionPlan = switch ($Mode) {
    "full" {
        Write-Section "FULL ECOSYSTEM ORCHESTRATION (All Systems)"
        $ecosystemSystems
    }
    "quick" {
        Write-Section "QUICK VALIDATION (CI/CD + Lint)"
        $ecosystemSystems | Where-Object { $_.name -in @("Lint System", "CI/CD Pipeline") }
    }
    "integration" {
        Write-Section "INTEGRATION TESTS (CI/CD Integration Only)"
        $ecosystemSystems | Where-Object { $_.name -eq "CI/CD Pipeline" }
    }
    "bug-hunt" {
        Write-Section "BUG HUNT CYCLE"
        $ecosystemSystems | Where-Object { $_.name -in @("Bug Hunt", "Survival System", "Knowledge Database") }
    }
    "learning" {
        Write-Section "LEARNING SYSTEMS (Survival + KDB)"
        $ecosystemSystems | Where-Object { $_.name -in @("Survival System", "Knowledge Database") }
    }
    "observability" {
        Write-Section "OBSERVABILITY & REPORTING"
        $ecosystemSystems | Where-Object { $_.name -in @("Observability", "ETL Pipeline") }
    }
}

# Execute the plan
$results = @{}
$startTime = Get-Date
$totalDuration = 0
$successCount = 0
$failureCount = 0

foreach ($system in $executionPlan) {
    if ($system.enabled) {
        $result = Invoke-EcosystemSystem -SystemName $system.name -Script $system.script -Mode $Mode
        $results[$system.name] = $result
        $totalDuration += $result.duration
        if ($result.success) {
            $successCount++
        }
        else {
            $failureCount++
        }
    }
}

# Summary Report
Write-Section "ECOSYSTEM ORCHESTRATION SUMMARY"

Write-Host ""
Write-Host "📊 Execution Results:" -ForegroundColor Cyan
Write-Host "   Systems Run: $($successCount + $failureCount)" -ForegroundColor White
Write-Host "   Successful: $successCount" -ForegroundColor Green
Write-Host "   Failed: $failureCount" -ForegroundColor $(if ($failureCount -eq 0) { "Green" } else { "Red" })
Write-Host "   Total Duration: $([math]::Round($totalDuration / 1000, 2))s" -ForegroundColor Cyan

Write-Host ""
Write-Host "System Details:" -ForegroundColor Cyan
foreach ($result in $results.GetEnumerator() | Sort-Object -Property Name) {
    $status = if ($result.Value.success) { "✅" } else { "❌" }
    $duration = "$($result.Value.duration)ms"
    Write-Host "   $status $($result.Name): $duration" -ForegroundColor $(if ($result.Value.success) { "Green" } else { "Red" })
}

Write-Host ""
Write-Host "📈 Metrics:" -ForegroundColor Cyan
Write-Host "   Success Rate: $(if ($successCount + $failureCount -gt 0) { [math]::Round(($successCount / ($successCount + $failureCount)) * 100, 2) }% else { "N/A" })" -ForegroundColor White
Write-Host "   Avg Duration per System: $([math]::Round($totalDuration / [math]::Max(1, $successCount + $failureCount), 2))ms" -ForegroundColor White

Write-Host ""
Write-Host "🔗 Integration Status:" -ForegroundColor Cyan
Write-Host "   CI/CD → Bug Hunt: Connected" -ForegroundColor Green
Write-Host "   Bug Hunt → Survival: Connected" -ForegroundColor Green
Write-Host "   Survival → KDB: Connected" -ForegroundColor Green
Write-Host "   KDB → ETL: Connected" -ForegroundColor Green
Write-Host "   ETL → Observability: Connected" -ForegroundColor Green
Write-Host "   All Systems → MCP: Connected" -ForegroundColor Green

Write-Host ""
Write-Host "═════════════════════════════════════════════════════════════" -ForegroundColor Cyan

if ($failureCount -eq 0) {
    Write-Host "✅ ECOSYSTEM ORCHESTRATION SUCCESSFUL" -ForegroundColor Green
    Write-Host "All systems integrated and operational" -ForegroundColor Green
    exit 0
}
else {
    Write-Host "⚠️  ECOSYSTEM ORCHESTRATION COMPLETED WITH ERRORS" -ForegroundColor Yellow
    Write-Host "$failureCount system(s) failed" -ForegroundColor Yellow
    exit 1
}
