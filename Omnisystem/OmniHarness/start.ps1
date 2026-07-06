#!/usr/bin/env pwsh
# OmniHarness — One-command startup
# Usage: .\start.ps1 [--no-kernel] [--no-gui] [--port 8080]

param(
    [switch]$NoKernel,
    [switch]$NoGui,
    [switch]$NoClj,
    [switch]$NoAutoBuild,
    [int]$Port = 8080,
    [int]$GuiPort = 3000,
    [int]$CljPort = 8090,
    [switch]$Help
)

if ($Help) {
    Write-Host @"
OmniHarness Startup Script
Usage: .\start.ps1 [options]

Options:
  --no-kernel      Skip the Rust gRPC kernel (orchestrator runs standalone)
  --no-gui         Skip the ClojureScript GUI (API only)
  --no-clj         Skip the Clojure HTN planner/policy API
  --no-auto-build  Don't auto-build the kernel / auto-install Python deps
                   when missing — just warn and skip them, like before
  --port N         Python orchestrator port (default: 8080)
  --gui-port N     GUI dev server port (default: 3000)
  --clj-port N     Clojure orchestrator HTTP API port (default: 8090)
  --help           Show this help

For individuals: the default behavior needs nothing pre-built. Missing
pieces (kernel binary, orchestrator deps) are built/installed automatically
on first run; a real health check at the end reports what's actually up.

Prerequisites:
  - Python 3.11+  (pip install -e "orchestrator/.[all]")
  - Rust 1.78+    (cargo build --release in kernel/)
  - Node 20+      (npm install in gui/)
  - Leiningen     (cd clj-orchestrator && lein deps) — HTN planner/policy API
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
    if (-not (Test-Path $kernelBin) -and -not $NoAutoBuild) {
        $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
        if ($cargoCmd) {
            Write-Step "Kernel not built yet — building it now (cargo build --release, one-time, ~1-2 min)..."
            Push-Location (Join-Path $Root "kernel")
            & cargo build --release
            $built = $LASTEXITCODE -eq 0
            Pop-Location
            if ($built) { Write-OK "Kernel built." } else { Write-Warn "Kernel build failed — see output above." }
        } else {
            Write-Warn "Rust (cargo) not found on PATH. Install from https://rustup.rs to enable the kernel."
        }
    }
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
        Write-Warn "Rust kernel not built. Run: cd kernel && cargo build --release (or drop --no-auto-build)"
        Write-Warn "Continuing without kernel (orchestrator runs standalone)"
    }
}

# Install orchestrator Python dependencies if missing (mirrors the VS Code
# extension's own auto-install logic — individuals shouldn't need to know
# this is a separate pip install step at all).
if (-not $NoAutoBuild) {
    Push-Location (Join-Path $Root "orchestrator")
    $importCheck = & python -c "import uvicorn, fastapi, pydantic, httpx, dotenv, sse_starlette, aiosqlite, aiofiles" 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Dependency check failed: $importCheck"
        Write-Step "Orchestrator dependencies missing — installing (pip install -r requirements.txt)..."
        & python -m pip install -r requirements.txt
        if ($LASTEXITCODE -eq 0) { Write-OK "Orchestrator dependencies installed." }
        else { Write-Warn "pip install failed — see output above." }
    }
    Pop-Location
}

# Start Python orchestrator
Write-Step "Starting Python orchestrator on :$Port..."
$orchJob = Start-Job -ScriptBlock {
    param($root, $port)
    Set-Location (Join-Path $root "orchestrator")
    & python -m uvicorn omniharness.server:app --host 0.0.0.0 --port $port --log-level info
} -ArgumentList $Root, $Port
$Jobs += $orchJob
Write-OK "Orchestrator starting at http://localhost:$Port"
Write-OK "  API docs: http://localhost:$Port/docs"
Write-OK "  Health:   http://localhost:$Port/api/health"
Write-OK "  Models:   http://localhost:$Port/api/models"

# Start Clojure orchestrator — HTN planner + policy engine HTTP API (optional)
if (-not $NoClj) {
    # Don't just check `lein` is *found* on PATH — verify it actually runs.
    # A `lein` shim can exist on PATH but point at a broken/incomplete
    # self-install (seen in the wild: a Scoop-installed shim referencing a
    # leiningen-*.jar that was never downloaded), which would otherwise
    # silently hang the background job with zero useful output.
    $leinCmd = Get-Command lein -ErrorAction SilentlyContinue
    $leinWorks = $false
    if ($leinCmd) {
        & lein --version *> $null
        $leinWorks = $LASTEXITCODE -eq 0
    }
    if ($leinWorks) {
        Write-Step "Starting Clojure orchestrator (planner/policy API) on :$CljPort..."
        $cljJob = Start-Job -ScriptBlock {
            param($root, $cljPort)
            Set-Location (Join-Path $root "clj-orchestrator")
            $env:CLJ_HTTP_PORT = $cljPort
            & lein run serve
        } -ArgumentList $Root, $CljPort
        $Jobs += $cljJob
        Start-Sleep 2
        Write-OK "Clojure orchestrator starting at http://localhost:$CljPort (health: /health)"
    } elseif ($leinCmd) {
        Write-Warn "Found 'lein' on PATH at $($leinCmd.Source) but it doesn't run (broken/incomplete install)."
        Write-Warn "Try: lein self-install — or fix PATH so a working lein is found first. Skipping Clojure orchestrator."
    } else {
        Write-Warn "Leiningen (lein) not found on PATH. Skipping Clojure orchestrator."
        Write-Warn "The Python orchestrator degrades gracefully without it (/api/planner/* returns 503)."
    }
}

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

# Real health check — poll each service instead of assuming a fixed sleep
# was long enough, and report what's ACTUALLY up rather than what we hoped
# would start.
function Wait-Http($url, $seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $r = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 3 -ErrorAction Stop
            if ($r.StatusCode -eq 200) { return $true }
        } catch {}
        Start-Sleep -Milliseconds 500
    }
    return $false
}

Write-Step "Waiting for services to come up (first boot can be slow — several cloud-provider SDKs import at module load)..."
# 90s for the orchestrator: cold Python startup importing every provider SDK
# (anthropic/openai/cohere/mistralai/groq/google-generativeai/...) can
# genuinely take a while on a loaded machine, especially the very first run.
# 127.0.0.1, not "localhost": these servers bind 0.0.0.0/IPv4 only, but
# Invoke-WebRequest resolves "localhost" to the IPv6 ::1 first and — unlike
# curl — doesn't fall back to IPv4 within one call, so every attempt would
# time out even though the service is genuinely up and reachable.
$orchOk = Wait-Http "http://127.0.0.1:$Port/api/health" 90
$cljOk  = if (-not $NoClj)  { Wait-Http "http://127.0.0.1:$CljPort/health" 90 } else { $null }
$guiOk  = if (-not $NoGui)  { Wait-Http "http://127.0.0.1:$GuiPort" 20 }        else { $null }
$kernelOk = if ($orchOk) {
    try { ((Invoke-WebRequest -Uri "http://127.0.0.1:$Port/api/health" -UseBasicParsing).Content | ConvertFrom-Json).kernel } catch { $false }
} else { $false }

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host "  OmniHarness status" -ForegroundColor White
Write-Host "  Orchestrator:  http://localhost:$Port  $(if ($orchOk) {'[UP]'} else {'[NOT RESPONDING]'})" -ForegroundColor $(if ($orchOk) {'Green'} else {'Red'})
if (-not $NoKernel) {
    Write-Host "  Kernel:        (via orchestrator)      $(if ($kernelOk) {'[UP]'} else {'[not connected]'})" -ForegroundColor $(if ($kernelOk) {'Green'} else {'Yellow'})
}
if (-not $NoClj) {
    Write-Host "  Clj Planner:   http://localhost:$CljPort  $(if ($cljOk) {'[UP]'} else {'[not running]'})" -ForegroundColor $(if ($cljOk) {'Green'} else {'Yellow'})
}
if (-not $NoGui) {
    Write-Host "  GUI:           http://localhost:$GuiPort  $(if ($guiOk) {'[UP]'} else {'[not running]'})" -ForegroundColor $(if ($guiOk) {'Green'} else {'Yellow'})
}
Write-Host "  CLI:           omniharness chat `"Hello!`"" -ForegroundColor White
Write-Host "  Press Ctrl+C to stop all services" -ForegroundColor DarkGray
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor Cyan
Write-Host ""
if (-not $orchOk) {
    Write-Host "[ERROR] Orchestrator never became healthy — check the log output below." -ForegroundColor Red
}

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
