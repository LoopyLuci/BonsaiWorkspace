#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Build and Launch Bonsai Workspace with Octopus AI

.DESCRIPTION
    Complete build script for compiling Rust crates, frontend, Tauri app, and launching.
    Automatically handles dependencies, compilation, and runs the desktop IDE.

.NOTES
    Requires: Rust, Node.js/pnpm, Tauri CLI
#>

param(
    [switch]$SkipBuild,
    [switch]$SkipFrontend,
    [switch]$LaunchDev,
    [switch]$ReleaseMode
)

$workspace = Get-Location
$timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'

function Write-Header {
    param([string]$Text)
    Write-Host "`n" -NoNewline
    Write-Host "═" * 80 -ForegroundColor Cyan
    Write-Host "🧬 $Text" -ForegroundColor Cyan
    Write-Host "═" * 80 -ForegroundColor Cyan
}

function Write-Step {
    param([string]$Text)
    Write-Host "`n➤ $Text" -ForegroundColor Green
}

function Write-Error {
    param([string]$Text)
    Write-Host "❌ $Text" -ForegroundColor Red
}

function Test-Dependency {
    param([string]$Command, [string]$Name)
    $exists = $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
    if ($exists) {
        Write-Host "✅ $Name found" -ForegroundColor Green
    } else {
        Write-Error "$Name not found. Please install it first."
        exit 1
    }
    return $exists
}

Write-Header "Bonsai Workspace Build & Launch"

# Verify dependencies
Write-Step "Checking dependencies..."
Test-Dependency "cargo" "Rust/Cargo"
Test-Dependency "node" "Node.js"
Test-Dependency "pnpm" "pnpm" | Out-Null
if ($null -eq (Get-Command "pnpm" -ErrorAction SilentlyContinue)) {
    Write-Step "Installing pnpm..."
    npm install -g pnpm
}

# Build Rust workspace
if (-not $SkipBuild) {
    Write-Step "Building Rust crates (release mode)..."
    Write-Host "This may take 10-30 minutes on first build..."

    if ($ReleaseMode) {
        cargo build --release --all 2>&1 | Tee-Object -FilePath build.log
    } else {
        cargo build --release 2>&1 | Tee-Object -FilePath build.log
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Rust build failed. Check build.log for details."
        exit 1
    }
    Write-Host "✅ Rust build successful" -ForegroundColor Green
} else {
    Write-Host "⊘ Skipping Rust build" -ForegroundColor Yellow
}

# Build frontend
if (-not $SkipFrontend) {
    Write-Step "Building Bonsai Workspace frontend..."
    Set-Location "$workspace\bonsai-workspace"

    Write-Host "Installing dependencies..." -ForegroundColor Blue
    pnpm install 2>&1 | Tee-Object -FilePath "..\frontend-build.log"

    if ($LASTEXITCODE -ne 0) {
        Write-Error "pnpm install failed. Check frontend-build.log for details."
        exit 1
    }

    Write-Host "Building frontend assets..." -ForegroundColor Blue
    pnpm build 2>&1 | Tee-Object -Append -FilePath "..\frontend-build.log"

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Frontend build failed. Check frontend-build.log for details."
        exit 1
    }

    Write-Host "✅ Frontend build successful" -ForegroundColor Green
    Set-Location $workspace
} else {
    Write-Host "⊘ Skipping frontend build" -ForegroundColor Yellow
}

# Setup models directory
Write-Step "Setting up models directory..."
$modelsDir = "$env:USERPROFILE\.bonsai\models"
if (-not (Test-Path $modelsDir)) {
    New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
    Write-Host "Created: $modelsDir" -ForegroundColor Green
}

# Create default config
Write-Step "Creating Bonsai configuration..."
$configDir = "$env:USERPROFILE\.bonsai"
$configFile = "$configDir\config.toml"

if (-not (Test-Path $configFile)) {
    $config = @"
[bonsai]
name = "Bonsai Workspace"
version = "0.2.0"

[models]
default = "octopus-v1"
model_dir = "$modelsDir"
auto_load = true

[api]
host = "127.0.0.1"
port = 11425
inference_port = 4000

[ui]
theme = "auto"
default_model_selector = true
show_model_descriptions = true

[performance]
max_cached_models = 3
gpu_acceleration = false
cpu_threads = 8
"@
    Add-Content -Path $configFile -Value $config
    Write-Host "Created: $configFile" -ForegroundColor Green
}

Write-Header "Build Complete!"

# Launch options
Write-Host "`n" -NoNewline
Write-Host "Ready to launch! Choose an option:" -ForegroundColor Cyan
Write-Host "`n1. Launch development mode (hot-reload, logs visible)"
Write-Host "2. Launch production build"
Write-Host "3. Test CLI only (no UI)"
Write-Host "4. Exit`n"

if ($LaunchDev -or -not $LaunchDev) {
    Write-Host "Launching development mode..." -ForegroundColor Green
    Write-Host "`n" -NoNewline
    Write-Host "🧬 Starting Bonsai Workspace..." -ForegroundColor Cyan
    Write-Host "The IDE should open in your default browser/window." -ForegroundColor Yellow
    Write-Host "Model selector available in the sidebar. Default: Octopus AI v1`n" -ForegroundColor Yellow

    Set-Location "$workspace\bonsai-workspace"
    pnpm tauri dev
} else {
    Write-Host "Skipping launch. To start:" -ForegroundColor Yellow
    Write-Host "cd bonsai-workspace && pnpm tauri dev" -ForegroundColor Yellow
}
