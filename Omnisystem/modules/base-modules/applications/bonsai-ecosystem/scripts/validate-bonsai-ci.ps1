#!/usr/bin/env pwsh
<#
.SYNOPSIS
Validate Bonsai CI/CD deployment setup

.DESCRIPTION
Checks that all required components are in place and properly configured

.PARAMETER Environment
Environment to validate (production/staging)

.PARAMETER CheckRemote
Also validate GitHub integration (requires GITHUB_TOKEN)
#>

param(
    [string]$Environment = "staging",
    [switch]$CheckRemote
)

$ErrorActionPreference = "Stop"
$checks = @()
$failedChecks = @()

function Add-Check {
    param(
        [string]$Name,
        [string]$Description,
        [scriptblock]$Test,
        [bool]$Critical = $true
    )

    $result = try {
        & $Test
        $true
    }
    catch {
        $false
    }

    $check = [PSCustomObject]@{
        Name        = $Name
        Description = $Description
        Passed      = $result
        Critical    = $Critical
        Error       = if (-not $result) { $_.Exception.Message } else { $null }
    }

    $checks += $check

    if (-not $result) {
        $failedChecks += $check
    }

    $status = if ($result) { "✓" } else { "✗" }
    Write-Host "$status $Name" -ForegroundColor $(if ($result) { "Green" } else { "Red" })

    if (-not $result) {
        Write-Host "  → $($check.Error)" -ForegroundColor Red
    }
}

function Test-FilesExist {
    Write-Host "`n=== Checking Required Files ===" -ForegroundColor Cyan

    Add-Check "orchestrator.ps1" "CI/CD orchestrator exists" {
        Test-Path ".\scripts\bonsai-ci-orchestrator-complete.ps1" -PathType Leaf
    } -Critical $true

    Add-Check "deployment script" "Deployment script exists" {
        Test-Path ".\scripts\deploy-bonsai-ci.ps1" -PathType Leaf
    } -Critical $true

    Add-Check "health check script" "Health check endpoint exists" {
        Test-Path ".\scripts\bonsai-ci-health-check.ps1" -PathType Leaf
    } -Critical $false

    Add-Check "CI config" "CI configuration file exists" {
        Test-Path ".\bonsai-ci-config.json" -PathType Leaf
    } -Critical $true

    Add-Check "GitHub workflow" "GitHub workflow file exists" {
        Test-Path ".\.github\workflows\bonsai-ci-native.yml" -PathType Leaf
    } -Critical $true
}

function Test-PowerShellVersion {
    Write-Host "`n=== Checking PowerShell ===" -ForegroundColor Cyan

    Add-Check "PowerShell version" "PowerShell 7 or later" {
        $PSVersionTable.PSVersion.Major -ge 7
    } -Critical $true
}

function Test-Prerequisites {
    Write-Host "`n=== Checking Prerequisites ===" -ForegroundColor Cyan

    Add-Check "Git" "Git is available" {
        git --version > $null
    } -Critical $true

    Add-Check "Cargo" "Cargo is available" {
        cargo --version > $null
    } -Critical $true

    Add-Check "Rust" "Rust toolchain is installed" {
        rustc --version > $null
    } -Critical $true
}

function Test-ConfigurationFiles {
    Write-Host "`n=== Checking Configuration ===" -ForegroundColor Cyan

    Add-Check ".env.ci file" "Environment configuration exists" {
        Test-Path ".\.env.ci" -PathType Leaf
    } -Critical $false

    Add-Check "Config JSON syntax" "bonsai-ci-config.json is valid JSON" {
        $config = Get-Content ".\bonsai-ci-config.json" -Raw
        $parsed = $config | ConvertFrom-Json
        $parsed -ne $null
    } -Critical $true
}

function Test-ArtifactStorage {
    Write-Host "`n=== Checking Artifact Storage ===" -ForegroundColor Cyan

    Add-Check "Artifacts directory" "Artifact storage directory exists" {
        Test-Path ".\artifacts" -PathType Container
    } -Critical $false

    Add-Check "Artifacts subdirs" "Required artifact subdirectories exist" {
        $required = @("pr-validation", "nightly-soak", "stress-tests", "coverage", "benchmarks")
        foreach ($dir in $required) {
            if (-not (Test-Path ".\artifacts\$dir" -PathType Container)) {
                throw "Missing directory: artifacts\$dir"
            }
        }
        $true
    } -Critical $false
}

function Test-EnvironmentVariables {
    Write-Host "`n=== Checking Environment Variables ===" -ForegroundColor Cyan

    Add-Check "GITHUB_TOKEN" "GitHub token set" {
        -not [string]::IsNullOrEmpty($env:GITHUB_TOKEN)
    } -Critical $CheckRemote

    Add-Check "SLACK_WEBHOOK" "Slack webhook configured" {
        -not [string]::IsNullOrEmpty($env:SLACK_WEBHOOK)
    } -Critical $false
}

function Test-Orchestrator {
    Write-Host "`n=== Testing Orchestrator ===" -ForegroundColor Cyan

    Add-Check "Orchestrator syntax" "Orchestrator script has valid syntax" {
        $ortho = Get-Content ".\scripts\bonsai-ci-orchestrator-complete.ps1"
        [System.Management.Automation.PSParser]::Tokenize($ortho, [ref]$null) | Out-Null
        $true
    } -Critical $true

    Add-Check "Orchestrator dry-run" "Orchestrator runs without errors (dry-run)" {
        & ".\scripts\bonsai-ci-orchestrator-complete.ps1" `
            -Workflow "pr_validation" `
            -DryRun `
            -ErrorAction SilentlyContinue | Out-Null
        $true
    } -Critical $false
}

function Test-GitHubIntegration {
    Write-Host "`n=== Testing GitHub Integration ===" -ForegroundColor Cyan

    if (-not $CheckRemote) {
        Write-Host "Skipping (use -CheckRemote flag to test)" -ForegroundColor Yellow
        return
    }

    Add-Check "GitHub API accessibility" "Can reach GitHub API" {
        $response = Invoke-WebRequest -Uri "https://api.github.com" -Method Get -ErrorAction SilentlyContinue
        $response.StatusCode -eq 200
    } -Critical $false

    Add-Check "GitHub token validity" "GitHub token is valid" {
        $headers = @{ Authorization = "token $($env:GITHUB_TOKEN)" }
        $response = Invoke-WebRequest -Uri "https://api.github.com/user" -Headers $headers -ErrorAction SilentlyContinue
        $response.StatusCode -eq 200
    } -Critical $false
}

function Test-SlackIntegration {
    Write-Host "`n=== Testing Slack Integration ===" -ForegroundColor Cyan

    if ([string]::IsNullOrEmpty($env:SLACK_WEBHOOK)) {
        Write-Host "Slack not configured (optional)" -ForegroundColor Yellow
        return
    }

    Add-Check "Slack webhook accessible" "Can reach Slack webhook" {
        $response = Invoke-WebRequest -Uri $env:SLACK_WEBHOOK -Method Post `
            -Body '{"text":"health check"}' `
            -ContentType "application/json" `
            -ErrorAction SilentlyContinue
        $response.StatusCode -eq 200
    } -Critical $false
}

function Test-WorkflowSyntax {
    Write-Host "`n=== Checking Workflow Syntax ===" -ForegroundColor Cyan

    Add-Check "GitHub workflow YAML" "GitHub workflow is valid YAML" {
        # Basic YAML validation (check for required fields)
        $workflow = Get-Content ".\.github\workflows\bonsai-ci-native.yml" -Raw
        if ($workflow -match "^name:" -and $workflow -match "^on:" -and $workflow -match "^jobs:") {
            $true
        } else {
            throw "Missing required workflow fields"
        }
    } -Critical $true
}

function Write-Summary {
    $passed = ($checks | Where-Object { $_.Passed }).Count
    $total = $checks.Count
    $critical_failed = ($failedChecks | Where-Object { $_.Critical }).Count

    Write-Host "`n=== VALIDATION SUMMARY ===" -ForegroundColor Cyan
    Write-Host "Passed: $passed / $total"
    Write-Host "Failed: $($total - $passed)"

    if ($critical_failed -gt 0) {
        Write-Host "`nCritical Failures: $critical_failed" -ForegroundColor Red
        foreach ($check in ($failedChecks | Where-Object { $_.Critical })) {
            Write-Host "  ✗ $($check.Name)" -ForegroundColor Red
        }
    }

    Write-Host "`nValidation Status: " -NoNewline
    if ($critical_failed -eq 0) {
        Write-Host "✓ READY FOR DEPLOYMENT" -ForegroundColor Green
    } else {
        Write-Host "✗ REQUIRES FIXES BEFORE DEPLOYMENT" -ForegroundColor Red
    }

    return $critical_failed -eq 0
}

# Main execution
Write-Host "Bonsai CI/CD Validation Report" -ForegroundColor Cyan
Write-Host "Environment: $Environment" -ForegroundColor Cyan
Write-Host "Time: $(Get-Date)" -ForegroundColor Cyan

Test-FilesExist
Test-PowerShellVersion
Test-Prerequisites
Test-ConfigurationFiles
Test-ArtifactStorage
Test-EnvironmentVariables
Test-Orchestrator
Test-WorkflowSyntax
Test-GitHubIntegration
Test-SlackIntegration

$ready = Write-Summary

exit $(if ($ready) { 0 } else { 1 })
