#!/usr/bin/env pwsh
# OmniHarness — One-command startup
# Usage: .\start.ps1 [--no-kernel] [--no-gui] [--port 8080]

param(
    [switch]$NoKernel,
    [switch]$NoGui,
    [int]$Port = 8080,
    [int]$GuiPort = 3000,
    [switch]$Help
)

if ($Help) {
    Write-Host @"
OmniHarness Startup Script
Usage: .\start.ps1 [options]

Options:
  --no-kernel    Skip the Rust gRPC kernel (orchestrator runs standalone)
  --no-gui       Skip the ClojureScript GUI (API only)
  --port N       Python orchestrator port (default: 8080)
  --gui-port N   GUI dev server port (default: 3000)
  --help         Show this help

Prerequisites:
  - Python 3.11+  (pip install -e "orchestrator/.[all]")
  - Rust 1.78+    (cargo build --release in kernel/)
  - Node 20+      (npm install in gui/)
  - .env file     (copy .env.example to .env and fill in keys)
"@
    exit 0
}

$Root = $PSScriptRoot
$Jobs = @()

function Write-Step($msg) {
    Write-Host "[OmniHarness] $msg" -ForegroundColor Cyan
}

function Write-OK($msg) {
    Write-Host "[OK] $msg" -ForegroundColor Green
}

function Write-Warn($msg) {
    Write-Host "[WARN] $msg" -ForegroundColor Yellow
}

# Load .env into environment
$envFile = Join-Path $Root ".env"
if (Test-Path $envFile) {
    Get-Content $envFile | ForEach-Object {
        if ($_ -match '^\s*([A-Z_]+)\s*=\s*(.+)\s*$') {
            $name = $matches[1]; $val = $matches[2].Trim('"').Trim("'")
            if ($val -ne "" -and -not [System.Environment]::GetEnvironmentVariable($name)) {
                [System.Environment]::SetEnvironmentVariable($name, $val, "Process")
            }
        }
    }
    Write-OK "Loaded .env"
} else {
    Write-Warn ".env not found — copy .env.example to .env and add your API keys"
    Write-Warn "Running with environment variables only"
}

# Check Python
try {
    $pyVersion = & python --version 2>&1
    Write-OK "Python: $pyVersion"
} catch {
    Write-Host "[ERROR] Python not found. Install Python 3.11+ from https://python.org" -ForegroundColor Red
    exit 1
}

# Start Rust kernel (optional)
if (-not $NoKernel) {
    $kernelBin = Join-Path $Root "kernel\target\release\omniharness-kernel.exe"
    if (Test-Path $kernelBin) {
        Write-Step "Starting Rust kernel on :50051..."
        $kernelJob = Start-Job -ScriptBlock {
            param($bin, $root)
            Set-Location $root
            & $bin
        } -ArgumentList $kernelBin, $Root
        $Jobs += $kernelJob
        Start-Sleep 1
        Write-OK "Kernel starting (Job $($kernelJob.Id))"
    } else {
        Write-Warn "Rust kernel not built. Run: cd kernel && cargo build --release"
        Write-Warn "Continuing without kernel (orchestrator runs standalone)"
    }
}

# Start Python orchestrator
Write-Step "Starting Python orchestrator on :$Port..."
$orchJob = Start-Job -ScriptBlock {
    param($root, $port)
    Set-Location (Join-Path $root "orchestrator")
    & python -m uvicorn omniharness.server:app --host 0.0.0.0 --port $port --log-level info
} -ArgumentList $Root, $Port
$Jobs += $orchJob
Start-Sleep 2
Write-OK "Orchestrator starting at http://localhost:$Port"
Write-OK "  API docs: http://localhost:$Port/docs"
Write-OK "  Health:   http://localhost:$Port/api/health"
Write-OK "  Models:   http://localhost:$Port/api/models"

# Start ClojureScript GUI (optional)
if (-not $NoGui) {
    $guiPath = Join-Path $Root "gui"
    $nodeModules = Join-Path $guiPath "node_modules"
    if (Test-Path $nodeModules) {
        Write-Step "Starting GUI dev server on :$GuiPort..."
        $guiJob = Start-Job -ScriptBlock {
            param($guiPath)
            Set-Location $guiPath
            & npm run dev
        } -ArgumentList $guiPath
        $Jobs += $guiJob
        Start-Sleep 2
        Write-OK "GUI starting at http://localhost:$GuiPort"
    } else {
        Write-Warn "GUI dependencies not installed. Run: cd gui && npm install"
        Write-Warn "Continuing without GUI"
    }
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "  OmniHarness is running" -ForegroundColor White
Write-Host "  Orchestrator:  http://localhost:$Port" -ForegroundColor White
if (-not $NoGui) {
    Write-Host "  GUI:           http://localhost:$GuiPort" -ForegroundColor White
}
Write-Host "  CLI:           omniharness chat `"Hello!`"" -ForegroundColor White
Write-Host "  Press Ctrl+C to stop all services" -ForegroundColor Dim
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""

# Keep alive and relay output
try {
    while ($true) {
        foreach ($job in $Jobs) {
            $output = Receive-Job $job
            if ($output) { $output | ForEach-Object { Write-Host $_ } }
            if ($job.State -eq "Failed") {
                Write-Host "[ERROR] Job $($job.Id) failed" -ForegroundColor Red
                Receive-Job $job -ErrorAction SilentlyContinue
            }
        }
        Start-Sleep 2
    }
} finally {
    Write-Host "`nShutting down..." -ForegroundColor Yellow
    $Jobs | Stop-Job
    $Jobs | Remove-Job -Force
    Write-Host "All services stopped." -ForegroundColor Yellow
}
