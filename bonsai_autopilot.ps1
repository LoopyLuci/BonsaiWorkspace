<#
.SYNOPSIS
    Bonsai Autopilot — builds, launches, verifies, loads models, and orchestrates training.

.DESCRIPTION
    Automates the full Bonsai lifecycle:
      1. Build the workspace (cargo --release)
      2. Launch the Bonsai desktop app
      3. Verify API endpoints
      4. Cycle through candidate models (load best available)
      5. Start training services
      6. Monitor training for -MonitorMinutes (default 60)

.PARAMETER DryRun
    Print planned actions without executing any of them.

.PARAMETER Force
    Skip preflight checks and proceed even when optional components are absent.

.PARAMETER MonitorMinutes
    How long (in minutes) to poll training status after startup. Default: 60.

.EXAMPLE
    pwsh -NoProfile -ExecutionPolicy Bypass -File .\bonsai_autopilot.ps1 -DryRun
    pwsh -NoProfile -ExecutionPolicy Bypass -File .\bonsai_autopilot.ps1 -MonitorMinutes 120
#>
param(
    [switch]$DryRun,
    [switch]$Force,
    [int]$MonitorMinutes = 60
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$isDryRun = [bool]$DryRun
$isForce  = [bool]$Force

if ($isDryRun) { Write-Host '[AUTOPILOT] DRY-RUN MODE — no destructive actions will be taken.' -ForegroundColor Yellow }

# ── Cargo path resolution ─────────────────────────────────────────────────────

function Get-CargoExe {
    $candidates = @(
        "$env:USERPROFILE\.cargo\bin\cargo.exe",
        "$env:CARGO_HOME\bin\cargo.exe",
        (Get-Command cargo -ErrorAction SilentlyContinue)?.Source
    ) | Where-Object { $_ -and (Test-Path $_) }
    return $candidates | Select-Object -First 1
}

$cargoExe = Get-CargoExe
if (-not $cargoExe) {
    Write-Warning 'cargo not found. Install Rust from https://rustup.rs/ or add ~/.cargo/bin to PATH.'
    if (-not $isForce) { exit 1 }
}

# ── Step 1: Build ─────────────────────────────────────────────────────────────

function Step-1_Build {
    Write-Host '[STEP 1] Building bonsai-workspace (--release)...' -ForegroundColor Cyan

    $workspace = Join-Path $PSScriptRoot 'bonsai-workspace'
    if (-not (Test-Path $workspace)) {
        Write-Warning "Workspace not found: $workspace"
        if (-not $isForce) { return @{ success = $false; reason = 'workspace-missing' } }
    }

    if ($isDryRun) {
        Write-Host "  DryRun: would run: $cargoExe build --release in $workspace"
        return @{ success = $true; dryrun = $true }
    }

    try {
        $proc = Start-Process -FilePath $cargoExe `
            -ArgumentList 'build', '--release' `
            -WorkingDirectory $workspace `
            -Wait -PassThru -NoNewWindow
        if ($proc.ExitCode -eq 0) {
            Write-Host '  Build succeeded.' -ForegroundColor Green
            return @{ success = $true }
        }
        Write-Warning "  Build failed (exit $($proc.ExitCode))"
        return @{ success = $false; exit = $proc.ExitCode }
    } catch {
        Write-Warning "  Build exception: $($_.Exception.Message)"
        return @{ success = $false; reason = $_.Exception.Message }
    }
}

# ── Step 2: Launch ────────────────────────────────────────────────────────────

function Step-2_Launch {
    Write-Host '[STEP 2] Launching Bonsai...' -ForegroundColor Cyan

    $candidates = @(
        (Join-Path $PSScriptRoot 'bonsai-workspace\src-tauri\target\release\bonsai-workspace.exe'),
        (Join-Path $PSScriptRoot 'BonsaiWorkspace.exe'),
        (Join-Path $PSScriptRoot 'target\release\bonsai-workspace.exe')
    )

    foreach ($p in $candidates) {
        if (Test-Path $p) {
            if ($isDryRun) { Write-Host "  DryRun: would launch $p"; return @{ pid = 0 } }
            $proc = Start-Process -FilePath $p -WorkingDirectory $PSScriptRoot -PassThru
            Write-Host "  Launched $p (pid=$($proc.Id))" -ForegroundColor Green
            return $proc
        }
    }

    # Fallback: node launcher
    $launchMjs = Join-Path $PSScriptRoot 'bonsai-workspace\src\launch-all.mjs'
    if (Test-Path $launchMjs) {
        if ($isDryRun) { Write-Host "  DryRun: would run node $launchMjs"; return @{ pid = 0 } }
        $node = (Get-Command node -ErrorAction SilentlyContinue)?.Source
        if ($node) {
            $proc = Start-Process -FilePath $node -ArgumentList $launchMjs -WorkingDirectory (Split-Path $launchMjs) -PassThru
            Write-Host "  Launched via node (pid=$($proc.Id))" -ForegroundColor Green
            return $proc
        }
    }

    Write-Warning '  No launcher found — start Bonsai manually, then re-run verification.'
    return $null
}

# ── Config / API helpers ──────────────────────────────────────────────────────

function Wait-ForConfig([int]$TimeoutSec = 180) {
    $cfg = Join-Path $env:APPDATA 'com.bonsai.workspace\bonsai-config.json'
    $t0  = Get-Date
    while (-not (Test-Path $cfg)) {
        if ((Get-Date) - $t0 -gt [TimeSpan]::FromSeconds($TimeoutSec)) { throw "Config not found after ${TimeoutSec}s" }
        Start-Sleep -Seconds 2
    }
    return $cfg
}

function Load-ApiInfo {
    if ($isDryRun) { return @{ base = 'http://127.0.0.1:11369/api/v1'; token = 'dryrun-token' } }
    $cfgPath = Wait-ForConfig -TimeoutSec 180
    $json    = Get-Content $cfgPath -Raw | ConvertFrom-Json
    $port    = if ($json.api_port -and $json.api_port -ne 0) { $json.api_port } else { 11369 }
    return @{ base = "http://127.0.0.1:${port}/api/v1"; token = $json.pair_token }
}

function Invoke-ApiCall([string]$Base, [string]$Token, [string]$Path, [string]$Method = 'GET', $Body = $null) {
    $url     = $Base + $Path
    $headers = @{ Authorization = "Bearer $Token" }
    if ($isDryRun) { Write-Host "  DryRun: $Method $url"; return $true }
    try {
        if ($Method -eq 'GET') {
            $null = Invoke-RestMethod -Uri $url -Headers $headers -TimeoutSec 10
        } else {
            $null = Invoke-RestMethod -Uri $url -Method $Method -Headers $headers `
                -Body ($Body | ConvertTo-Json -Depth 10) -ContentType 'application/json' -TimeoutSec 30
        }
        Write-Host "  PASS: $Path" -ForegroundColor Green
        return $true
    } catch {
        Write-Warning "  FAIL: $Path -> $($_.Exception.Message)"
        return $false
    }
}

# ── Step 3: Verify endpoints ──────────────────────────────────────────────────

function Step-3_Verify {
    Write-Host '[STEP 3] Verifying API endpoints...' -ForegroundColor Cyan
    $info = Load-ApiInfo
    $b = $info.base; $t = $info.token
    return @{
        health  = Invoke-ApiCall $b $t '/health'
        agents  = Invoke-ApiCall $b $t '/agents/list'
        models  = Invoke-ApiCall $b $t '/models/list'
        bonsai  = Invoke-ApiCall $b $t '/bonsai/process' -Method 'POST' -Body @{ prompt = 'ping' }
    }
}

# ── Step 4: Model cycling ─────────────────────────────────────────────────────

function Step-4_LoadBestModel {
    Write-Host '[STEP 4] Selecting and loading model...' -ForegroundColor Cyan

    # Ordered preference list — first existing file wins
    $models = @(
        @{ path = 'D:\Models\general\Qwen3.6-35B-A3B-Compact\Qwen3.6-35B-A3B-Compact.gguf'; gpu_layers = 20 },
        @{ path = 'D:\Models\general\Qwen2.5-14B-Instruct-Q4_K_M\Qwen2.5-14B-Instruct-Q4_K_M.gguf'; gpu_layers = 30 },
        @{ path = 'D:\Models\general\Mistral-7B-Instruct-v0.3-Q4_K_M\Mistral-7B-Instruct-v0.3-Q4_K_M.gguf'; gpu_layers = 40 },
        @{ path = 'D:\Models\general\Bonsai-1.7B-Q4_K_M\Bonsai-1.7B-Q4_K_M.gguf'; gpu_layers = 99 },
        @{ path = 'D:\Models\general\Bonsai-1.7B-Q2_K\Bonsai-1.7B-Q2_K.gguf'; gpu_layers = 99 }
    )

    $selected = $models | Where-Object { (Test-Path $_.path) -or $isDryRun } | Select-Object -First 1
    if (-not $selected) {
        Write-Warning '  No model files found on D:\Models — skipping model load.'
        return $false
    }

    Write-Host "  Selected: $($selected.path) (gpu_layers=$($selected.gpu_layers))"
    $info = Load-ApiInfo
    return Invoke-ApiCall $info.base $info.token '/models/load' -Method 'POST' -Body $selected
}

# ── Step 5: Training ──────────────────────────────────────────────────────────

function Step-5_StartTraining {
    Write-Host '[STEP 5] Starting training services...' -ForegroundColor Cyan
    $info = Load-ApiInfo
    $b = $info.base; $t = $info.token
    $sp = Invoke-ApiCall $b $t '/training/self-play/start' -Method 'POST' -Body @{ rounds = 20 }
    $lp = Invoke-ApiCall $b $t '/training/loop/start'      -Method 'POST' -Body @{ strategy = 'dpo' }
    return @{ selfplay = $sp; loop = $lp }
}

# ── Step 6: Monitor ───────────────────────────────────────────────────────────

function Step-6_Monitor([int]$PollSec = 60, [int]$TotalMinutes = 60) {
    if ($isDryRun) { Write-Host '[STEP 6] DryRun: skipping monitoring.'; return }
    $iterations = [int][math]::Ceiling(($TotalMinutes * 60) / $PollSec)
    Write-Host "[STEP 6] Monitoring training every ${PollSec}s for ${TotalMinutes}min ($iterations polls)..." -ForegroundColor Cyan
    $info = Load-ApiInfo
    $b = $info.base; $t = $info.token
    $headers = @{ Authorization = "Bearer $t" }

    for ($i = 0; $i -lt $iterations; $i++) {
        try {
            $sp = Invoke-RestMethod -Uri "$b/training/self-play/status" -Headers $headers -TimeoutSec 10
            $lp = Invoke-RestMethod -Uri "$b/training/loop/status"      -Headers $headers -TimeoutSec 10
            $ts = Get-Date -Format 'HH:mm:ss'
            Write-Host "  [$ts] SelfPlay: rounds=$($sp.rounds) gaps=$($sp.gaps_found) | Loop: rounds=$($lp.rounds) examples=$($lp.examples)"
        } catch {
            Write-Warning "  Monitor poll $i failed: $($_.Exception.Message)"
        }
        if ($i -lt ($iterations - 1)) { Start-Sleep -Seconds $PollSec }
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────

try {
    $build = Step-1_Build
    if (-not $build.success) {
        Write-Warning 'Build step failed or was skipped.'
        if (-not $isForce) { exit 1 }
    }

    $proc = Step-2_Launch
    if ($null -eq $proc) {
        Write-Warning 'Launch step produced no process handle. Continuing in case app is already running.'
    }

    Write-Host 'Waiting for Bonsai config file...' -ForegroundColor DarkGray
    $info = Load-ApiInfo
    Write-Host "  API base: $($info.base)"

    $verify = Step-3_Verify
    Write-Host 'Endpoint results:'
    $verify | Format-List

    Step-4_LoadBestModel | Out-Null
    Step-5_StartTraining | Out-Null
    Step-6_Monitor -PollSec 60 -TotalMinutes $MonitorMinutes

    Write-Host '[AUTOPILOT] Sequence complete.' -ForegroundColor Green
} catch {
    Write-Error "[AUTOPILOT] Fatal: $($_.Exception.Message)"
    exit 2
}
