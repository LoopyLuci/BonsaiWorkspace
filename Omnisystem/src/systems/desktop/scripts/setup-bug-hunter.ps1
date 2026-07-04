# Bug Hunter Setup Script
# Purpose: Build MCP server and prepare environment for Bug Hunter
# Usage: .\scripts\setup-bug-hunter.ps1

param(
    [switch]$BuildOnly = $false,
    [switch]$StartServer = $false,
    [switch]$ScheduleTasks = $false
)

$ErrorActionPreference = "Stop"

function Write-Status($message, $type = "Info") {
    $colors = @{
        Success = "Green"
        Warning = "Yellow"
        Error = "Red"
        Info = "Cyan"
    }
    $color = $colors[$type]
    Write-Host "[$([DateTime]::Now.ToString("HH:mm:ss"))] $message" -ForegroundColor $color
}

function Test-Cargo {
    try {
        $version = & cargo --version
        Write-Status "✓ Cargo found: $version" Success
        return $true
    }
    catch {
        Write-Status "✗ Cargo not found. Install Rust from https://rustup.rs/" Error
        return $false
    }
}

function Build-McpServer {
    Write-Status "Building mcp-server..." Info

    try {
        Push-Location

        # Go to workspace root
        $workspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
        Set-Location $workspaceRoot

        Write-Status "Workspace: $workspaceRoot"

        # Build the MCP server
        Write-Status "Running: cargo build --package mcp-server --release" Info
        & cargo build --package mcp-server --release

        if ($LASTEXITCODE -ne 0) {
            Write-Status "Build failed. See errors above." Error
            return $false
        }

        $binaryPath = "target/release/mcp-server.exe"
        if (Test-Path $binaryPath) {
            Write-Status "✓ Binary created: $binaryPath" Success
            return $binaryPath
        }
        else {
            Write-Status "✗ Binary not found at expected location" Error
            return $false
        }
    }
    finally {
        Pop-Location
    }
}

function Start-McpServer($binaryPath) {
    Write-Status "Starting MCP server..." Info

    if (-not (Test-Path $binaryPath)) {
        Write-Status "Binary not found: $binaryPath" Error
        return $false
    }

    try {
        # Check if port 3000 is already in use
        $port = Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue
        if ($port) {
            Write-Status "Port 3000 already in use. Kill existing process?" Warning
            $response = Read-Host "Kill process? (y/n)"
            if ($response -eq 'y') {
                Stop-Process -Id $port.OwningProcess -Force
                Start-Sleep -Seconds 2
            }
            else {
                Write-Status "Cannot start server with port in use" Error
                return $false
            }
        }

        # Start server in background
        Start-Process -FilePath $binaryPath -NoNewWindow -PassThru

        Write-Status "Waiting for server to start..." Info
        Start-Sleep -Seconds 3

        # Verify it's running
        try {
            $response = Invoke-WebRequest -Uri "http://127.0.0.1:3000/tools" `
                -Method GET `
                -ErrorAction SilentlyContinue -TimeoutSec 5

            $toolCount = ($response.Content | ConvertFrom-Json).tools.Count
            Write-Status "✓ Server running with $toolCount tools" Success
            return $true
        }
        catch {
            Write-Status "⚠ Server started but not responding yet. Give it a moment..." Warning
            return $true
        }
    }
    catch {
        Write-Status "Error starting server: $_" Error
        return $false
    }
}

function Create-ScheduledTasks {
    Write-Status "Setting up scheduled tasks..." Info

    $workspaceRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
    $scriptPath = Join-Path $workspaceRoot "scripts\run-bug-hunter.ps1"

    # Task 1: Daily scan at 2 AM
    Write-Status "Creating daily scan task..." Info
    $trigger = New-ScheduledTaskTrigger -Daily -At 2am
    $action = New-ScheduledTaskAction -Execute "powershell.exe" `
        -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" -Mode full"

    Register-ScheduledTask -TaskName "BugHunter-DailyScan" `
        -Trigger $trigger `
        -Action $action `
        -Description "Daily Bug Hunter scan" `
        -Force | Out-Null

    Write-Status "✓ Daily scan scheduled for 2 AM" Success

    # Task 2: Weekly deep scan at Sunday 1 AM
    Write-Status "Creating weekly deep scan task..." Info
    $trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek Sunday -At 1am
    $action = New-ScheduledTaskAction -Execute "powershell.exe" `
        -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`" -Mode full -SaveToSurvival -SaveToKDB"

    Register-ScheduledTask -TaskName "BugHunter-WeeklyScan" `
        -Trigger $trigger `
        -Action $action `
        -Description "Weekly deep scan with KDB sync" `
        -Force | Out-Null

    Write-Status "✓ Weekly scan scheduled for Sunday 1 AM" Success
}

function Main {
    Write-Host ""
    Write-Status "╔════════════════════════════════════════╗" Info
    Write-Status "║   Bug Hunter Setup                    ║" Info
    Write-Status "╚════════════════════════════════════════╝" Info
    Write-Host ""

    # Step 1: Check Cargo
    Write-Status "Step 1: Checking prerequisites..." Info
    if (-not (Test-Cargo)) {
        exit 1
    }

    Write-Host ""

    # Step 2: Build MCP Server
    Write-Status "Step 2: Building MCP server..." Info
    $binaryPath = Build-McpServer

    if (-not $binaryPath) {
        Write-Status "Setup failed" Error
        exit 1
    }

    Write-Host ""

    if ($BuildOnly) {
        Write-Status "Build complete. Binary: $binaryPath" Success
        exit 0
    }

    # Step 3: Start Server
    if ($StartServer) {
        Write-Status "Step 3: Starting MCP server..." Info
        if (-not (Start-McpServer $binaryPath)) {
            exit 1
        }
        Write-Host ""
    }

    # Step 4: Schedule Tasks
    if ($ScheduleTasks) {
        Write-Status "Step 4: Setting up scheduled tasks..." Info
        Create-ScheduledTasks
        Write-Host ""
    }

    # Success
    Write-Status "╔════════════════════════════════════════╗" Info
    Write-Status "║     Setup Complete!                  ║" Info
    Write-Status "╚════════════════════════════════════════╝" Info

    Write-Host ""
    Write-Status "Next steps:" Info
    Write-Status "1. Start MCP server:" Info
    Write-Status "   .\target\release\mcp-server.exe"
    Write-Host ""
    Write-Status "2. Connect Claude Code to the MCP server"
    Write-Host ""
    Write-Status "3. Run your first scan:" Info
    Write-Status "   .\scripts\run-bug-hunter.ps1 -Path 'Z:\Projects\BonsaiEcosystem' -Mode full"
    Write-Host ""
}

Main
