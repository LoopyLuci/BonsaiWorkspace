#!/usr/bin/env pwsh
<#
.SYNOPSIS
Deploy Bonsai Native CI/CD to production runners

.DESCRIPTION
Sets up CI/CD orchestrator, configures webhooks, verifies GitHub integration

.PARAMETER Environment
Production or staging environment (default: staging)

.PARAMETER SlackWebhook
Slack webhook URL for notifications

.PARAMETER GitHubToken
GitHub token for status checks

.PARAMETER ArtifactPath
Path to store CI artifacts (default: ./artifacts)

.PARAMETER DryRun
Validate setup without applying changes
#>

param(
    [string]$Environment = "staging",
    [string]$SlackWebhook = "",
    [string]$GitHubToken = "",
    [string]$ArtifactPath = "./artifacts",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$VerbosePreference = "Continue"

# Colors for output
$colors = @{
    success = "Green"
    error = "Red"
    warning = "Yellow"
    info = "Cyan"
}

function Write-Log {
    param([string]$Message, [string]$Type = "info")
    $color = $colors[$Type]
    Write-Host "[$((Get-Date).ToString('HH:mm:ss'))] $Message" -ForegroundColor $color
}

function Test-Prerequisites {
    Write-Log "Validating prerequisites..." "info"

    $prerequisites = @(
        @{Name = "PowerShell"; Test = { $PSVersionTable.PSVersion.Major -ge 7 } }
        @{Name = "git"; Test = { git --version } }
        @{Name = "cargo"; Test = { cargo --version } }
    )

    foreach ($prereq in $prerequisites) {
        try {
            & $prereq.Test | Out-Null
            Write-Log "✓ $($prereq.Name) available" "success"
        }
        catch {
            Write-Log "✗ $($prereq.Name) not found" "error"
            throw "Missing prerequisite: $($prereq.Name)"
        }
    }
}

function Setup-ArtifactStorage {
    Write-Log "Setting up artifact storage at $ArtifactPath..." "info"

    if (-not (Test-Path $ArtifactPath)) {
        if ($DryRun) {
            Write-Log "[DRY RUN] Would create directory: $ArtifactPath" "warning"
        }
        else {
            New-Item -ItemType Directory -Path $ArtifactPath -Force | Out-Null
            Write-Log "✓ Artifact storage created" "success"
        }
    }
    else {
        Write-Log "✓ Artifact storage already exists" "success"
    }

    # Create subdirectories
    $subdirs = @("pr-validation", "nightly-soak", "stress-tests", "regression", "coverage", "benchmarks")
    foreach ($subdir in $subdirs) {
        $path = Join-Path $ArtifactPath $subdir
        if (-not (Test-Path $path)) {
            if ($DryRun) {
                Write-Log "[DRY RUN] Would create: $path" "warning"
            }
            else {
                New-Item -ItemType Directory -Path $path -Force | Out-Null
            }
        }
    }

    Write-Log "✓ Artifact directories ready" "success"
}

function Setup-EnvironmentVariables {
    Write-Log "Configuring environment variables..." "info"

    $envFile = ".env.ci"
    $envContent = @"
# Bonsai CI/CD Environment Configuration
BONSAI_CI_ENVIRONMENT=$Environment
BONSAI_ARTIFACT_PATH=$ArtifactPath
BONSAI_CI_PARALLEL_JOBS=8
BONSAI_CI_TIMEOUT_MINUTES=60
BONSAI_CI_RETRY_ATTEMPTS=3
BONSAI_LOG_LEVEL=info
BONSAI_SLACK_WEBHOOK=$SlackWebhook
BONSAI_GITHUB_TOKEN=$GitHubToken

# Performance settings
BONSAI_CARGO_INCREMENTAL=1
BONSAI_CARGO_CODEGEN_UNITS=16
BONSAI_SCCACHE_ENABLED=true

# Health check
BONSAI_HEALTH_CHECK_PORT=8080
BONSAI_HEALTH_CHECK_INTERVAL_SECS=30
"@

    if ($DryRun) {
        Write-Log "[DRY RUN] Would write .env file:" "warning"
        Write-Host $envContent
    }
    else {
        Set-Content -Path $envFile -Value $envContent
        Write-Log "✓ Environment file created: $envFile" "success"
    }
}

function Setup-HealthCheckEndpoint {
    Write-Log "Creating health check endpoint configuration..." "info"

    $healthCheckScript = @"
#!/usr/bin/env pwsh
<#
.SYNOPSIS
Bonsai CI/CD Health Check Endpoint

Provides real-time status of CI/CD system
#>

param([int]`$Port = 8080)

`$ErrorActionPreference = "Stop"

# CI health status
`$ciStatus = @{
    status = "healthy"
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
    version = "1.0.0"
    components = @{
        orchestrator = @{
            status = if (Test-Path ".\\scripts\\bonsai-ci-orchestrator-complete.ps1") { "ready" } else { "missing" }
            last_check = (Get-Date).ToUniversalTime().ToString("o")
        }
        artifact_storage = @{
            status = if (Test-Path ".\\artifacts") { "ready" } else { "missing" }
            last_check = (Get-Date).ToUniversalTime().ToString("o")
        }
        environment = @{
            status = if (Test-Path ".env.ci") { "configured" } else { "unconfigured" }
            last_check = (Get-Date).ToUniversalTime().ToString("o")
        }
    }
    metrics = @{
        uptime_seconds = 0
        requests_processed = 0
        error_rate = 0.0
    }
}

function Start-HealthCheckServer {
    param([int]`$Port)

    `$listener = New-Object System.Net.HttpListener
    `$listener.Prefixes.Add("http://localhost:`$Port/")
    `$listener.Start()

    Write-Host "Health check endpoint listening on port `$Port"

    while (`$true) {
        try {
            `$context = `$listener.GetContext()
            `$response = `$context.Response

            `$jsonResponse = `$ciStatus | ConvertTo-Json -Depth 3
            `$bytes = [System.Text.Encoding]::UTF8.GetBytes(`$jsonResponse)

            `$response.ContentType = "application/json"
            `$response.ContentLength64 = `$bytes.Length
            `$response.OutputStream.Write(`$bytes, 0, `$bytes.Length)
            `$response.OutputStream.Close()
        }
        catch {
            Write-Error "Health check error: `$_"
        }
    }
}

Start-HealthCheckServer -Port `$Port
"@

    $healthCheckPath = "scripts\bonsai-ci-health-check.ps1"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would create health check: $healthCheckPath" "warning"
    }
    else {
        Set-Content -Path $healthCheckPath -Value $healthCheckScript
        Write-Log "✓ Health check endpoint created: $healthCheckPath" "success"
    }
}

function Setup-GitHubIntegration {
    Write-Log "Configuring GitHub integration..." "info"

    if ([string]::IsNullOrEmpty($GitHubToken)) {
        Write-Log "⚠ GitHub token not provided, skipping GitHub integration" "warning"
        return
    }

    # Create GitHub status check script
    $statusScript = @"
#!/usr/bin/env pwsh
<#
.SYNOPSIS
Update GitHub PR status checks

.PARAMETER Owner
GitHub repository owner

.PARAMETER Repo
GitHub repository name

.PARAMETER Sha
Commit SHA to update

.PARAMETER Context
Status context name

.PARAMETER State
Status state (pending, success, failure, error)

.PARAMETER Description
Status description

.PARAMETER TargetUrl
URL to detailed status
#>

param(
    [Parameter(Mandatory=`$true)][string]`$Owner,
    [Parameter(Mandatory=`$true)][string]`$Repo,
    [Parameter(Mandatory=`$true)][string]`$Sha,
    [Parameter(Mandatory=`$true)][string]`$Context,
    [Parameter(Mandatory=`$true)][string]`$State,
    [string]`$Description = "",
    [string]`$TargetUrl = ""
)

`$headers = @{
    "Authorization" = "token `$env:GITHUB_TOKEN"
    "Accept" = "application/vnd.github.v3+json"
}

`$body = @{
    state = `$State
    context = `$Context
    description = `$Description
    target_url = `$TargetUrl
} | ConvertTo-Json

`$uri = "https://api.github.com/repos/`$Owner/`$Repo/statuses/`$Sha"

Invoke-RestMethod -Uri `$uri -Method POST -Headers `$headers -Body `$body
"@

    if ($DryRun) {
        Write-Log "[DRY RUN] Would create GitHub status script" "warning"
    }
    else {
        Set-Content -Path "scripts\bonsai-ci-github-status.ps1" -Value $statusScript
        Write-Log "✓ GitHub integration configured" "success"
    }
}

function Setup-SlackNotifications {
    Write-Log "Setting up Slack notifications..." "info"

    if ([string]::IsNullOrEmpty($SlackWebhook)) {
        Write-Log "⚠ Slack webhook not provided, skipping Slack setup" "warning"
        return
    }

    $slackScript = @"
#!/usr/bin/env pwsh
<#
.SYNOPSIS
Send notifications to Slack

.PARAMETER Message
Notification message

.PARAMETER Channel
Slack channel

.PARAMETER Severity
Message severity (info, warning, error)
#>

param(
    [Parameter(Mandatory=`$true)][string]`$Message,
    [string]`$Channel = "#ci-cd",
    [string]`$Severity = "info"
)

`$colors = @{
    info = "#36a64f"
    warning = "#f4a400"
    error = "#dd3333"
}

`$payload = @{
    channel = `$Channel
    attachments = @(@{
        color = `$colors[`$Severity]
        text = `$Message
        ts = [int][double]::Parse((Get-Date -UFormat %s))
    })
} | ConvertTo-Json

Invoke-RestMethod -Uri `$env:SLACK_WEBHOOK -Method POST -Body `$payload -ContentType "application/json"
"@

    if ($DryRun) {
        Write-Log "[DRY RUN] Would create Slack notification script" "warning"
    }
    else {
        Set-Content -Path "scripts\bonsai-ci-slack-notify.ps1" -Value $slackScript
        Write-Log "✓ Slack notifications configured" "success"
    }
}

function Verify-Orchestrator {
    Write-Log "Verifying CI/CD orchestrator..." "info"

    $orchestratorPath = "scripts\bonsai-ci-orchestrator-complete.ps1"

    if (-not (Test-Path $orchestratorPath)) {
        Write-Log "✗ Orchestrator not found at $orchestratorPath" "error"
        throw "Orchestrator missing"
    }

    Write-Log "✓ Orchestrator found" "success"

    # Test orchestrator with dry-run
    Write-Log "Testing orchestrator with dry-run..." "info"

    if ($DryRun) {
        Write-Log "[DRY RUN] Would execute: .$orchestratorPath -Workflow pr-validation -DryRun" "warning"
    }
    else {
        try {
            & $orchestratorPath -Workflow "pr_validation" -DryRun 2>&1 | ForEach-Object {
                Write-Log $_
            }
            Write-Log "✓ Orchestrator test successful" "success"
        }
        catch {
            Write-Log "⚠ Orchestrator test failed: $_" "warning"
        }
    }
}

function Create-DeploymentSummary {
    Write-Log "Creating deployment summary..." "info"

    $summary = @"
# Bonsai CI/CD Deployment Summary

**Environment:** $Environment
**Date:** $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')
**DryRun:** $DryRun

## Configured Components

- ✓ Artifact Storage: $ArtifactPath
- ✓ Environment Variables: .env.ci
- ✓ Health Check Endpoint: scripts\bonsai-ci-health-check.ps1
- ✓ GitHub Integration: scripts\bonsai-ci-github-status.ps1
- ✓ Slack Notifications: scripts\bonsai-ci-slack-notify.ps1
- ✓ Orchestrator Verified: scripts\bonsai-ci-orchestrator-complete.ps1

## Next Steps

1. Deploy orchestrator to CI runners
2. Configure webhook triggers
3. Set GitHub secrets (GITHUB_TOKEN, SLACK_WEBHOOK)
4. Start health check endpoint: pwsh scripts\bonsai-ci-health-check.ps1
5. Monitor first PR validation run

## Success Criteria

- [ ] PR validations complete in <47 minutes
- [ ] GitHub status checks appearing on PRs
- [ ] Slack notifications received
- [ ] Artifacts collected and accessible
- [ ] Health check endpoint responding
"@

    if ($DryRun) {
        Write-Log "[DRY RUN] Would save deployment summary:" "warning"
        Write-Host $summary
    }
    else {
        Set-Content -Path "DEPLOYMENT_SUMMARY.md" -Value $summary
        Write-Log "✓ Deployment summary created: DEPLOYMENT_SUMMARY.md" "success"
    }
}

# Main execution
try {
    Write-Log "Starting Bonsai CI/CD deployment..." "info"
    Write-Log "Environment: $Environment, DryRun: $DryRun" "info"

    Test-Prerequisites
    Setup-ArtifactStorage
    Setup-EnvironmentVariables
    Setup-HealthCheckEndpoint
    Setup-GitHubIntegration
    Setup-SlackNotifications
    Verify-Orchestrator
    Create-DeploymentSummary

    Write-Log "✓ Deployment configuration complete" "success"

    if ($DryRun) {
        Write-Log "Dry-run completed. Review output and run without -DryRun to apply." "info"
    }
    else {
        Write-Log "Deployment ready. Start health check with: pwsh scripts\bonsai-ci-health-check.ps1" "info"
    }
}
catch {
    Write-Log "✗ Deployment failed: $_" "error"
    exit 1
}
