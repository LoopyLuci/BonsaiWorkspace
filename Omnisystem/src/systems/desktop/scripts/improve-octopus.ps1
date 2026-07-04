#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Nightly Octopus AI Improvement Script

.DESCRIPTION
    Runs the EternalTrainingLoop to continuously improve Octopus AI from user feedback.
    Collects feedback, fine-tunes LoRA adapters, validates, and hot-swaps improved models.

.NOTES
    Runs on CPU - no GPU required for inference
    Scheduled to run nightly at 3 AM (configurable)
#>

param(
    [string]$ModelPath = "$PSScriptRoot\..\models\octopus-v1.bkp",
    [string]$FeedbackPath = "$PSScriptRoot\..\data\feedback",
    [string]$OutputPath = "$PSScriptRoot\..\models\octopus-v1-improved"
)

$workspace = Split-Path -Parent $PSScriptRoot
$logFile = "$workspace\logs\octopus-improvement-$(Get-Date -Format 'yyyy-MM-dd').log"

function Write-Log {
    param([string]$Message)
    $timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    $logMessage = "[$timestamp] $Message"
    Write-Host $logMessage -ForegroundColor Cyan
    Add-Content -Path $logFile -Value $logMessage
}

try {
    Write-Log "🐙 Octopus AI - Nightly Improvement Starting"
    Write-Log "Workspace: $workspace"
    Write-Log "Model: $ModelPath"

    # Check if feedback exists
    if (-not (Test-Path $FeedbackPath)) {
        Write-Log "⚠️  No feedback directory found. Creating..."
        New-Item -ItemType Directory -Path $FeedbackPath -Force | Out-Null
    }

    $feedbackFiles = Get-ChildItem -Path $FeedbackPath -Filter "*.jsonl" -ErrorAction SilentlyContinue
    $feedbackCount = if ($feedbackFiles) { ($feedbackFiles | Measure-Object -Line).Lines } else { 0 }

    if ($feedbackCount -eq 0) {
        Write-Log "ℹ️  No feedback collected yet. Skipping training."
        Write-Log "🐙 Octopus AI - Nightly Improvement Completed (no changes)"
        exit 0
    }

    Write-Log "📊 Found feedback: $feedbackCount entries"

    # Step 1: Fine-tune LoRA adapters on new feedback
    Write-Log "🔄 Fine-tuning LoRA adapters..."

    $trainCommand = @(
        "python",
        "$workspace\crates\octopus-ai\train.py",
        "--model", $ModelPath,
        "--feedback", $FeedbackPath,
        "--method", "lora",
        "--rank", "16",
        "--epochs", "1",
        "--output", $OutputPath,
        "--validate"
    )

    & $trainCommand[0] $trainCommand[1..($trainCommand.Length-1)] 2>&1 | Tee-Object -FilePath $logFile -Append

    if ($LASTEXITCODE -eq 0) {
        Write-Log "✅ Fine-tuning successful"

        # Step 2: Validate the improved model
        Write-Log "🧪 Validating improved model..."

        $validateCommand = @(
            "python",
            "$workspace\crates\octopus-ai\test_suite.py",
            "--model", $OutputPath,
            "--quick-validation"
        )

        & $validateCommand[0] $validateCommand[1..($validateCommand.Length-1)] 2>&1 | Tee-Object -FilePath $logFile -Append

        if ($LASTEXITCODE -eq 0) {
            Write-Log "✅ Validation passed"

            # Step 3: Hot-swap if validation passes
            Write-Log "🔄 Hot-swapping improved model..."

            # Backup current model
            Copy-Item -Path $ModelPath -Destination "$ModelPath.backup" -Force

            # Move improved model to production
            Copy-Item -Path "$OutputPath\model.bkp" -Destination $ModelPath -Force

            Write-Log "✅ Model successfully improved and hot-swapped"

            # Clean up old improvement
            Remove-Item -Path $OutputPath -Recurse -Force -ErrorAction SilentlyContinue

            # Clear processed feedback
            Get-ChildItem -Path $FeedbackPath -Filter "*.jsonl" | Remove-Item -Force
            Write-Log "📝 Cleared processed feedback"

        } else {
            Write-Log "❌ Validation failed. Keeping current model."
            Remove-Item -Path $OutputPath -Recurse -Force -ErrorAction SilentlyContinue
        }
    } else {
        Write-Log "❌ Fine-tuning failed"
        Remove-Item -Path $OutputPath -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Log "🐙 Octopus AI - Nightly Improvement Completed"

} catch {
    Write-Log "❌ Error: $_"
    Write-Log "Stack trace: $($_.ScriptStackTrace)"
    exit 1
}
