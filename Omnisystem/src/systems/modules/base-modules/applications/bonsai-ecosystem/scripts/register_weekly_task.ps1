#Requires -RunAsAdministrator
<#
.SYNOPSIS
    Register a Windows Scheduled Task to run the BonsAI weekly training pipeline
    every Monday at 2:00 AM.

.DESCRIPTION
    Run this script once as Administrator. It creates a task that runs
    weekly_train.ps1 with the -SkipEval flag so it doesn't need a UI.

.NOTES
    To unregister: Unregister-ScheduledTask -TaskName "BonsAI Weekly Training" -Confirm:$false
    To view:       Get-ScheduledTask -TaskName "BonsAI Weekly Training"
    To run now:    Start-ScheduledTask -TaskName "BonsAI Weekly Training"
#>

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$TrainScript = Join-Path $ScriptDir "weekly_train.ps1"

if (-not (Test-Path $TrainScript)) {
    Write-Error "weekly_train.ps1 not found at $TrainScript"
    exit 1
}

$Action = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$TrainScript`" -SkipEval"

$Trigger = New-ScheduledTaskTrigger `
    -Weekly `
    -DaysOfWeek Monday `
    -At "02:00AM"

$Settings = New-ScheduledTaskSettingsSet `
    -StartWhenAvailable `
    -RunOnlyIfIdle `
    -IdleDuration (New-TimeSpan -Minutes 30) `
    -MultipleInstances IgnoreNew `
    -ExecutionTimeLimit (New-TimeSpan -Hours 6) `
    -Priority 7

$Principal = New-ScheduledTaskPrincipal `
    -UserId "SYSTEM" `
    -LogonType ServiceAccount `
    -RunLevel Highest

try {
    Register-ScheduledTask `
        -TaskName "BonsAI Weekly Training" `
        -Action $Action `
        -Trigger $Trigger `
        -Settings $Settings `
        -Principal $Principal `
        -Description "Runs the full BonsAI training pipeline every Monday at 2 AM. Managed by Bonsai Workspace." `
        -Force | Out-Null

    Write-Host "[bonsai] Scheduled task 'BonsAI Weekly Training' registered."
    Write-Host "         Runs: Every Monday at 02:00 AM"
    Write-Host "         Script: $TrainScript"
    Write-Host ""
    Write-Host "To test immediately:"
    Write-Host "  Start-ScheduledTask -TaskName 'BonsAI Weekly Training'"
} catch {
    Write-Error "Failed to register task: $_"
    Write-Host ""
    Write-Host "Try running this script as Administrator."
    exit 1
}
