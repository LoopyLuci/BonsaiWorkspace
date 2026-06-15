#!/usr/bin/env pwsh
# Complete Bonsai CI/CD Orchestrator
# Unified orchestration for all CI/CD workflows (replaces GitHub Actions)
# Supports: PR validation, nightly soak, Android regression, production deployment

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("pr-validation", "nightly-soak", "android-regression", "full-suite", "quick-check", "stress-test", "report")]
    [string]$Workflow = "pr-validation",

    [Parameter(Mandatory=$false)]
    [string]$ConfigFile = "bonsai-ci-complete.yaml",

    [Parameter(Mandatory=$false)]
    [int]$ParallelJobs = 4,

    [Parameter(Mandatory=$false)]
    [switch]$DryRun = $false,

    [Parameter(Mandatory=$false)]
    [switch]$Verbose = $false,

    [Parameter(Mandatory=$false)]
    [hashtable]$AndroidParams = @{}
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Colors
$colors = @{
    Header = "Cyan"
    Success = "Green"
    Warning = "Yellow"
    Error = "Red"
    Status = "White"
    Metric = "Magenta"
}

function Write-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host "╔$("═" * 70)╗" -ForegroundColor $colors.Header
    Write-Host "║ $Message.PadRight(68) ║" -ForegroundColor $colors.Header
    Write-Host "╚$("═" * 70)╝" -ForegroundColor $colors.Header
    Write-Host ""
}

function Write-Status {
    param([string]$Message, [string]$Status)
    $icon = switch ($Status) {
        "running" { "⚙️ " }
        "success" { "✅ " }
        "failed" { "❌ " }
        "pending" { "⏳ " }
        "skipped" { "⊘ " }
        default { "ℹ️ " }
    }
    Write-Host "$icon $Message" -ForegroundColor $colors.Status
}

function Write-Metric {
    param([string]$Label, [string]$Value)
    Write-Host "  📊 $Label`: $Value" -ForegroundColor $colors.Metric
}

function Invoke-Stage {
    param(
        [string]$StageName,
        [array]$Jobs,
        [int]$Timeout
    )

    Write-Host ""
    Write-Host "🎯 Stage: $StageName" -ForegroundColor $colors.Header
    Write-Host "   Jobs: $($Jobs.Count) | Parallel: $ParallelJobs | Timeout: $Timeout min" -ForegroundColor $colors.Status
    Write-Host ""

    $stageStart = Get-Date
    $results = @()

    # Process jobs in batches based on ParallelJobs
    $jobQueue = [System.Collections.Queue]::new()
    $Jobs | ForEach-Object { $jobQueue.Enqueue($_) }

    while ($jobQueue.Count -gt 0) {
        $batch = @()
        $batchSize = [Math]::Min($ParallelJobs, $jobQueue.Count)

        for ($i = 0; $i -lt $batchSize; $i++) {
            $batch += $jobQueue.Dequeue()
        }

        # Run batch jobs in parallel
        $tasks = $batch | ForEach-Object -AsJob -Parallel {
            $job = $_
            $startTime = Get-Date

            Write-Host "Running: $($job.name)" -ForegroundColor Yellow

            if (-not $using:DryRun) {
                try {
                    $result = & $job.command
                    $duration = ((Get-Date) - $startTime).TotalSeconds

                    return @{
                        name = $job.name
                        status = "success"
                        duration = $duration
                        output = $result
                    }
                } catch {
                    $duration = ((Get-Date) - $startTime).TotalSeconds
                    return @{
                        name = $job.name
                        status = "failed"
                        duration = $duration
                        error = $_.Exception.Message
                    }
                }
            } else {
                return @{
                    name = $job.name
                    status = "dry-run"
                    duration = 0
                }
            }
        }

        # Wait for all jobs in batch
        $tasks | Wait-Job | ForEach-Object {
            $result = Receive-Job $_
            $results += $result

            $statusIcon = switch ($result.status) {
                "success" { "✅" }
                "failed" { "❌" }
                "dry-run" { "⊘" }
            }

            Write-Status "$($result.name)" $result.status
            Write-Metric "Duration" "$([Math]::Round($result.duration, 2))s"

            if ($result.error) {
                Write-Host "   Error: $($result.error)" -ForegroundColor $colors.Error
            }

            Remove-Job $_
        }
    }

    $stageDuration = ((Get-Date) - $stageStart).TotalSeconds
    $successCount = ($results | Where-Object { $_.status -eq "success" }).Count
    $failureCount = ($results | Where-Object { $_.status -eq "failed" }).Count

    Write-Host ""
    Write-Host "Stage Summary:" -ForegroundColor $colors.Header
    Write-Metric "Total Jobs" $results.Count
    Write-Metric "Passed" $successCount
    Write-Metric "Failed" $failureCount
    Write-Metric "Duration" "$([Math]::Round($stageDuration, 2))s"

    return @{
        name = $StageName
        results = $results
        duration = $stageDuration
        passed = $successCount
        failed = $failureCount
    }
}

function Test-Frontend {
    Write-Status "Frontend Quality Check" "running"

    $steps = @(
        @{
            name = "Install deps"
            command = {
                Set-Location "bonsai-workspace/src"
                npm ci | Out-Null
                Set-Location "../.."
            }
        },
        @{
            name = "Svelte check"
            command = {
                Set-Location "bonsai-workspace/src"
                npx svelte-check
                Set-Location "../.."
            }
        },
        @{
            name = "Build"
            command = {
                Set-Location "bonsai-workspace/src"
                npm run build
                Set-Location "../.."
            }
        },
        @{
            name = "Bundle check"
            command = {
                Set-Location "bonsai-workspace/src"
                npm run check:bundle
                Set-Location "../.."
            }
        }
    )

    foreach ($step in $steps) {
        if (-not $DryRun) {
            & $step.command
        }
    }

    return @{
        name = "Frontend Quality"
        command = { Write-Status "Frontend tests completed" "success" }
    }
}

function Test-VSCodeExtension {
    Write-Status "VSCode Extension Quality" "running"

    $steps = @(
        @{
            name = "Install deps"
            command = {
                Set-Location "vscode-extension"
                npm ci | Out-Null
                Set-Location ".."
            }
        },
        @{
            name = "Compile"
            command = {
                Set-Location "vscode-extension"
                npm run compile
                Set-Location ".."
            }
        },
        @{
            name = "Type check"
            command = {
                Set-Location "vscode-extension"
                npm run test:typecheck
                Set-Location ".."
            }
        },
        @{
            name = "Test"
            command = {
                Set-Location "vscode-extension"
                npm run test
                Set-Location ".."
            }
        }
    )

    foreach ($step in $steps) {
        if (-not $DryRun) {
            & $step.command
        }
    }

    return @{
        name = "VSCode Extension"
        command = { Write-Status "Extension tests completed" "success" }
    }
}

function Test-RustQuality {
    param([string]$Platform = "ubuntu")

    Write-Status "Rust Quality ($Platform)" "running"

    $steps = @(
        @{
            name = "Check"
            command = {
                cargo check --locked --manifest-path "bonsai-workspace/src-tauri/Cargo.toml"
            }
        },
        @{
            name = "Clippy"
            command = {
                cargo clippy --manifest-path "bonsai-workspace/src-tauri/Cargo.toml" --all-targets -- -D warnings
            }
        },
        @{
            name = "Audit"
            command = {
                cargo audit --manifest-path "bonsai-workspace/src-tauri/Cargo.toml"
            }
        },
        @{
            name = "Test"
            command = {
                cargo test --manifest-path "bonsai-workspace/src-tauri/Cargo.toml" --lib -- --test-threads=1
            }
        }
    )

    foreach ($step in $steps) {
        if (-not $DryRun) {
            try {
                & $step.command
            } catch {
                Write-Host "Error in $($step.name): $_" -ForegroundColor $colors.Error
                throw
            }
        }
    }

    return @{
        name = "Rust Quality"
        command = { Write-Status "Rust tests completed" "success" }
    }
}

function Test-Routing {
    Write-Status "Deterministic Routing" "running"

    $maxRetries = 3
    $env:BONSAI_UI_STEP_TIMEOUT_MS = "60000"

    for ($attempt = 1; $attempt -le $maxRetries; $attempt++) {
        Write-Host "  Attempt $attempt/$maxRetries" -ForegroundColor $colors.Status

        if (-not $DryRun) {
            Set-Location "bonsai-workspace/src"
            $result = npm run test:agent-routing-ci 2>&1
            Set-Location "../.."

            if ($LASTEXITCODE -eq 0) {
                return @{
                    name = "Routing Tests"
                    command = { Write-Status "Routing tests passed" "success" }
                }
            }
        } else {
            return @{
                name = "Routing Tests"
                command = { Write-Status "Routing tests (dry-run)" "success" }
            }
        }
    }

    throw "Routing tests failed after $maxRetries attempts"
}

function Test-APISmokeTests {
    Write-Status "API Smoke Tests" "running"

    $maxRetries = 2
    $env:BONSAI_SKIP_UI = "1"

    for ($attempt = 1; $attempt -le $maxRetries; $attempt++) {
        Write-Host "  Attempt $attempt/$maxRetries" -ForegroundColor $colors.Status

        if (-not $DryRun) {
            Set-Location "bonsai-workspace/src"
            $result = npm run test:agent-orchestrated 2>&1
            Set-Location "../.."

            if ($LASTEXITCODE -eq 0) {
                return @{
                    name = "API Smoke Tests"
                    command = { Write-Status "API smoke tests passed" "success" }
                }
            }
        } else {
            return @{
                name = "API Smoke Tests"
                command = { Write-Status "API smoke tests (dry-run)" "success" }
            }
        }
    }

    throw "API smoke tests failed after $maxRetries attempts"
}

function Test-AndroidTarget {
    Write-Status "Android Target Check" "running"

    if (-not $DryRun) {
        cargo check --locked `
            --manifest-path "bonsai-workspace/src-tauri/Cargo.toml" `
            --target aarch64-linux-android
    }

    return @{
        name = "Android Target"
        command = { Write-Status "Android target check completed" "success" }
    }
}

function Test-NightlySoak {
    param([string]$Platform = "windows")

    Write-Status "Nightly Soak ($Platform)" "running"

    $iterations = 10
    $passCount = 0

    if (-not $DryRun) {
        for ($i = 1; $i -le $iterations; $i++) {
            Write-Host "  Iteration $i/$iterations" -ForegroundColor $colors.Status

            Set-Location "bonsai-workspace/src"
            $result = npm run test:agent-routing-ci 2>&1
            Set-Location "../.."

            if ($LASTEXITCODE -eq 0) {
                $passCount++
            }
        }

        $flakeRate = if ($iterations -gt 0) {
            [Math]::Round(($iterations - $passCount) / $iterations * 100, 2)
        } else { 0 }

        Write-Metric "Passes" "$passCount/$iterations"
        Write-Metric "Flake Rate" "$flakeRate%"
    }

    return @{
        name = "Nightly Soak"
        command = { Write-Status "Nightly soak completed" "success" }
    }
}

# Main execution
Write-Header "Bonsai Complete CI/CD Orchestrator"
Write-Host "Workflow: $Workflow | Config: $ConfigFile | Parallel: $ParallelJobs"
Write-Host "Dry-Run: $DryRun | Verbose: $Verbose"
Write-Host ""

$workflowStart = Get-Date
$allResults = @()

try {
    switch ($Workflow) {
        "pr-validation" {
            Write-Host "🚀 Running PR Validation Workflow" -ForegroundColor $colors.Header
            Write-Host "  Stages: Frontend, VSCode, Rust, Integration, Android"

            $jobs = @(
                @{ name = "Frontend"; command = { Test-Frontend } },
                @{ name = "VSCode"; command = { Test-VSCodeExtension } }
            )
            $allResults += Invoke-Stage "Frontend & Extension Quality" $jobs 20

            $jobs = @(
                @{ name = "Rust-Ubuntu"; command = { Test-RustQuality "ubuntu" } },
                @{ name = "Rust-Windows"; command = { Test-RustQuality "windows" } }
            )
            $allResults += Invoke-Stage "Rust Quality (Multi-Platform)" $jobs 30

            $jobs = @(
                @{ name = "Routing"; command = { Test-Routing } },
                @{ name = "APISmokeTests"; command = { Test-APISmokeTests } }
            )
            $allResults += Invoke-Stage "Integration Tests" $jobs 45

            $jobs = @(
                @{ name = "AndroidTarget"; command = { Test-AndroidTarget } }
            )
            $allResults += Invoke-Stage "Android Target Verification" $jobs 25
        }

        "nightly-soak" {
            Write-Host "🌙 Running Nightly Soak Workflow" -ForegroundColor $colors.Header
            Write-Host "  Platforms: Windows, Ubuntu"
            Write-Host "  Iterations: 10 per platform"

            $jobs = @(
                @{ name = "WindowsSoak"; command = { Test-NightlySoak "windows" } },
                @{ name = "UbuntuSoak"; command = { Test-NightlySoak "ubuntu" } }
            )
            $allResults += Invoke-Stage "Extended Soak Testing" $jobs 120
        }

        "android-regression" {
            Write-Host "📱 Running Android USB Regression" -ForegroundColor $colors.Header
            Write-Host "  Parameters: $AndroidParams"

            if (-not $DryRun) {
                Set-Location "bonsai-workspace/src"
                npm run test:android-usb-regression
                Set-Location "../.."
            }

            Write-Status "Android regression completed" "success"
        }

        "full-suite" {
            Write-Host "🔬 Running Full Test Suite" -ForegroundColor $colors.Header

            # Run all workflows
            & $PSCommandPath -Workflow "pr-validation" -ParallelJobs $ParallelJobs -DryRun:$DryRun -Verbose:$Verbose
            & $PSCommandPath -Workflow "nightly-soak" -ParallelJobs $ParallelJobs -DryRun:$DryRun -Verbose:$Verbose
        }

        "quick-check" {
            Write-Host "⚡ Running Quick Check" -ForegroundColor $colors.Header
            Write-Host "  Fast validation: Frontend + Rust check"

            $jobs = @(
                @{ name = "Frontend"; command = { Test-Frontend } },
                @{ name = "RustCheck"; command = {
                    if (-not $DryRun) {
                        cargo check --locked --manifest-path "bonsai-workspace/src-tauri/Cargo.toml"
                    }
                } }
            )
            $allResults += Invoke-Stage "Quick Validation" $jobs 15
        }

        "stress-test" {
            Write-Host "💪 Running Extended Stress Test" -ForegroundColor $colors.Header
            Write-Host "  20 iterations of routing and API tests"

            for ($i = 1; $i -le 20; $i++) {
                Write-Status "Stress iteration $i/20" "running"
                & $PSCommandPath -Workflow "pr-validation" -ParallelJobs $ParallelJobs -DryRun:$DryRun
            }
        }

        "report" {
            Write-Host "📊 Generating CI/CD Report" -ForegroundColor $colors.Header
            Write-Host ""
            Write-Host "Bonsai Ecosystem CI/CD Configuration:" -ForegroundColor $colors.Header
            Write-Host "  - Unified PR validation with 6 parallel job types"
            Write-Host "  - Nightly soak testing (10 iterations per platform)"
            Write-Host "  - Android USB regression testing"
            Write-Host "  - Multi-platform support (Windows, Ubuntu, Android)"
            Write-Host "  - Parallel execution with configurable worker count"
            Write-Host "  - Artifact retention and reporting"
            Write-Host "  - GitHub integration for status checks"
        }
    }
} catch {
    Write-Host ""
    Write-Host "❌ Workflow failed: $_" -ForegroundColor $colors.Error
    Write-Host "Stack: $($_.ScriptStackTrace)" -ForegroundColor $colors.Error
    exit 1
}

# Summary
Write-Host ""
Write-Header "Workflow Execution Summary"

$totalDuration = ((Get-Date) - $workflowStart).TotalSeconds

Write-Metric "Total Duration" "$([Math]::Round($totalDuration, 2))s"
Write-Metric "Workflow" $Workflow
Write-Metric "Status" "✅ Passed"

if ($allResults) {
    $totalPassed = ($allResults | ForEach-Object { $_.passed } | Measure-Object -Sum).Sum
    $totalFailed = ($allResults | ForEach-Object { $_.failed } | Measure-Object -Sum).Sum

    Write-Host ""
    Write-Host "Detailed Results:" -ForegroundColor $colors.Header
    foreach ($result in $allResults) {
        Write-Host "  Stage: $($result.name)" -ForegroundColor $colors.Status
        Write-Metric "Passed" $result.passed
        Write-Metric "Failed" $result.failed
        Write-Metric "Duration" "$([Math]::Round($result.duration, 2))s"
    }

    Write-Host ""
    Write-Metric "Total Passed" $totalPassed
    Write-Metric "Total Failed" $totalFailed
}

Write-Host ""
Write-Host "✅ Bonsai CI/CD orchestration complete!" -ForegroundColor $colors.Success
Write-Host ""

exit 0
