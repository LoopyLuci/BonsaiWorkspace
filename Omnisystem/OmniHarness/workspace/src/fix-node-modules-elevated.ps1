param(
    [string]$projectPath = "Z:\Projects\WorkspaceWorkspace\workspace\src"
)

Set-StrictMode -Version Latest
Write-Host "Project path: $projectPath"
if (-not (Test-Path $projectPath)) {
    Write-Host "Project path not found: $projectPath"
    exit 1
}
Set-Location $projectPath

Write-Host "Taking ownership of node_modules (requires admin)..."
cmd /c "takeown /f node_modules /r /d y"

$u = "$env:USERDOMAIN\$env:USERNAME"
Write-Host "Granting full control to $u ..."
cmd /c "icacls node_modules /grant \"$u:F\" /T /C"

Start-Sleep -Seconds 1

Write-Host "Removing node_modules..."
try {
    Remove-Item -LiteralPath node_modules -Recurse -Force -ErrorAction Stop
    Write-Host "Removed node_modules."
} catch {
    Write-Host "Remove-Item failed: $($_.Exception.Message)"
    Write-Host "Attempting robocopy mirror delete..."
    $temp = Join-Path $env:TEMP ("empty_" + [guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $temp | Out-Null
    robocopy $temp node_modules /MIR | Out-Null
    Remove-Item $temp -Recurse -Force
    if (Test-Path node_modules) {
        Write-Host "Failed to remove node_modules after robocopy trick."
        exit 2
    } else {
        Write-Host "node_modules removed via robocopy trick."
    }
}

Write-Host "Running npm ci --no-audit --no-fund ..."
npm ci --no-audit --no-fund

$exitCode = $LASTEXITCODE
Write-Host "npm exit code: $exitCode"
exit $exitCode
