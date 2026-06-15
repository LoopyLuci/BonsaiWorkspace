#!/usr/bin/env pwsh
# Bonsai CI/CD Runner - Execute pipeline stages locally or in CI
# Usage: .\bonsai-ci-runner.ps1 -Stage all
#        .\bonsai-ci-runner.ps1 -Stage test -ParallelJobs 4

param(
    [ValidateSet("quality", "test", "build", "package", "security", "all")]
    [string]$Stage = "all",

    [int]$ParallelJobs = 8,

    [switch]$Report,
    [switch]$Verbose
)

$ErrorActionPreference = "Continue"
$workspace = Get-Location
$startTime = Get-Date
$results = @()

# ──────────────────────────────────────────────────────
# Helper Functions
# ──────────────────────────────────────────────────────

function Invoke-Job {
    param(
        [string]$Name,
        [string]$Command,
        [int]$TimeoutSeconds = 1800,
        [switch]$Parallel
    )

    if ($Verbose) {
        Write-Host "  ▶ $Name (timeout: ${TimeoutSeconds}s)" -ForegroundColor Cyan
    } else {
        Write-Host "  ▶ $Name" -ForegroundColor Cyan
    }

    $jobStart = Get-Date

    try {
        $process = Start-Process -FilePath "pwsh" `
            -ArgumentList "-NoProfile", "-Command", $Command `
            -NoNewWindow -PassThru -Wait `
            -ErrorAction Stop

        $duration = (Get-Date) - $jobStart
        $durationStr = "{0:00}:{1:00}:{2:00}" -f [int]$duration.TotalHours, $duration.Minutes, $duration.Seconds

        if ($process.ExitCode -eq 0) {
            Write-Host "    ✅ PASSED ($durationStr)" -ForegroundColor Green
            return @{
                Name = $Name
                Status = "PASSED"
                Duration = $duration
                DurationStr = $durationStr
                ExitCode = 0
            }
        } else {
            Write-Host "    ❌ FAILED (exit code: $($process.ExitCode))" -ForegroundColor Red
            return @{
                Name = $Name
                Status = "FAILED"
                Duration = $duration
                DurationStr = $durationStr
                ExitCode = $process.ExitCode
            }
        }
    } catch {
        $duration = (Get-Date) - $jobStart
        Write-Host "    ❌ ERROR: $_" -ForegroundColor Red
        return @{
            Name = $Name
            Status = "ERROR"
            Duration = $duration
            DurationStr = $duration.ToString()
            ExitCode = 1
        }
    }
}

# ──────────────────────────────────────────────────────
# Stage 1: Quality Gates
# ──────────────────────────────────────────────────────

if ($Stage -in @("quality", "all")) {
    Write-Host "`n📋 Stage 1: Quality Gates" -ForegroundColor Magenta
    Write-Host "   Lint, format, and dependency checks`n" -ForegroundColor Gray

    $results += Invoke-Job "cargo-check" "cargo check --workspace --all-targets" 1800
    $results += Invoke-Job "cargo-clippy" "cargo clippy --workspace -- -D warnings" 1200
    $results += Invoke-Job "cargo-fmt" "cargo fmt --all -- --check" 300
    $results += Invoke-Job "cargo-audit" "cargo audit" 300

    $failed = $results | Where-Object { $_.Status -ne "PASSED" }
    if ($failed) {
        Write-Host "`n❌ Quality gates failed" -ForegroundColor Red
        if (-not $Report) {
            exit 1
        }
    }
}

# ──────────────────────────────────────────────────────
# Stage 2: Tests
# ──────────────────────────────────────────────────────

if ($Stage -in @("test", "all")) {
    Write-Host "`n🧪 Stage 2: Tests" -ForegroundColor Magenta
    Write-Host "   Unit, integration, and documentation tests`n" -ForegroundColor Gray

    $results += Invoke-Job "unit-tests" "cargo test --workspace --lib --release" 1800
    $results += Invoke-Job "integration-tests" "cargo test --workspace --test '*' --release" 2700
    $results += Invoke-Job "doc-tests" "cargo test --workspace --doc" 900

    $failed = $results | Where-Object { $_.Status -ne "PASSED" }
    if ($failed -and $Stage -eq "test") {
        Write-Host "`n❌ Tests failed" -ForegroundColor Red
        if (-not $Report) {
            exit 1
        }
    }
}

# ──────────────────────────────────────────────────────
# Stage 3: Build
# ──────────────────────────────────────────────────────

if ($Stage -in @("build", "all")) {
    Write-Host "`n🔨 Stage 3: Build" -ForegroundColor Magenta
    Write-Host "   Compile release binaries`n" -ForegroundColor Gray

    $results += Invoke-Job "release-build" "cargo build --release --workspace" 3600
}

# ──────────────────────────────────────────────────────
# Stage 4: Package
# ──────────────────────────────────────────────────────

if ($Stage -in @("package", "all")) {
    Write-Host "`n📦 Stage 4: Package" -ForegroundColor Magenta
    Write-Host "   Create installers for all platforms`n" -ForegroundColor Gray

    $results += Invoke-Job "windows-installer" "./scripts/installers/build-windows-installer.ps1" 1200
    $results += Invoke-Job "android-apk" "./scripts/installers/build-android-apk.ps1" 1800
}

# ──────────────────────────────────────────────────────
# Stage 5: Security
# ──────────────────────────────────────────────────────

if ($Stage -in @("security", "all")) {
    Write-Host "`n🛡️ Stage 5: Security" -ForegroundColor Magenta
    Write-Host "   Security and compliance checks`n" -ForegroundColor Gray

    $results += Invoke-Job "dependency-scan" "cargo deny check" 600
}

# ──────────────────────────────────────────────────────
# Summary Report
# ──────────────────────────────────────────────────────

$totalDuration = (Get-Date) - $startTime
$totalDurationStr = "{0:00}:{1:00}:{2:00}" -f [int]$totalDuration.TotalHours, $totalDuration.Minutes, $totalDuration.Seconds
$passedCount = ($results | Where-Object { $_.Status -eq "PASSED" }).Count
$failedCount = ($results | Where-Object { $_.Status -ne "PASSED" }).Count

Write-Host "`n════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "📊 CI/CD Pipeline Summary" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════" -ForegroundColor Cyan

if ($Report -or $Verbose) {
    Write-Host ""
    foreach ($result in $results) {
        $icon = if ($result.Status -eq "PASSED") { "✅" } else { "❌" }
        Write-Host "$icon $($result.Name) - $($result.Status) ($($result.DurationStr))" -ForegroundColor (if ($result.Status -eq "PASSED") { "Green" } else { "Red" })
    }
}

Write-Host ""
Write-Host "   Total Jobs: $($results.Count)" -ForegroundColor Cyan
Write-Host "   Passed: $passedCount" -ForegroundColor Green
Write-Host "   Failed: $failedCount" -ForegroundColor (if ($failedCount -gt 0) { "Red" } else { "Green" })
Write-Host "   Duration: $totalDurationStr" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════" -ForegroundColor Cyan

if ($failedCount -eq 0) {
    Write-Host "✅ All stages passed!" -ForegroundColor Green
    exit 0
} else {
    Write-Host "❌ Pipeline failed!" -ForegroundColor Red
    exit 1
}
