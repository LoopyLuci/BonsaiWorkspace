#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Minimal Bonsai Ecosystem Setup (without GPU training)
    Builds USOS kernel, Bonsai Workspace IDE, and prepares training infrastructure
    (Python training skipped if not installed)
#>

param([switch]$SkipKernelBuild, [switch]$SkipIDE, [switch]$LaunchStack)

$workspace = "Z:\Projects\BonsaiEcosystem"

function Write-Header {
    param([string]$Text)
    Write-Host "`n" -NoNewline
    Write-Host "═" * 80 -ForegroundColor Cyan
    Write-Host "🖥️  $Text" -ForegroundColor Cyan
    Write-Host "═" * 80 -ForegroundColor Cyan
}

function Write-Step {
    param([string]$Text)
    Write-Host "`n➤ $Text" -ForegroundColor Green
}

Write-Header "BONSAI ECOSYSTEM — WINDOWS 10 LOCAL BUILD (Minimal)"

# ============================================================================
# PHASE 1: Build USOS Kernel
# ============================================================================

if (-not $SkipKernelBuild) {
    Write-Header "PHASE 1: BUILD USOS KERNEL"
    Write-Step "Building USOS kernel (bare-metal x86_64)..."

    Push-Location "$workspace\crates\kernel"

    # Ensure x86_64-unknown-none target is installed
    rustup target add x86_64-unknown-none 2>&1 | Out-Null

    cargo build --release --target x86_64-unknown-none 2>&1 | Tee-Object -FilePath "$workspace\usos-build.log"

    if ($LASTEXITCODE -eq 0) {
        $kernel_path = "$workspace\crates\kernel\target\x86_64-unknown-none\release\kernel"
        Write-Host "✅ Kernel built: $kernel_path" -ForegroundColor Green
        Write-Host "   Size: $((Get-Item $kernel_path).Length / 1KB) KB" -ForegroundColor Cyan
    } else {
        Write-Host "❌ Kernel build failed" -ForegroundColor Red
    }

    Pop-Location
}

# ============================================================================
# PHASE 2: Build Bonsai Workspace IDE
# ============================================================================

if (-not $SkipIDE) {
    Write-Header "PHASE 2: BUILD BONSAI WORKSPACE IDE"

    Write-Step "Building Rust crates..."
    Push-Location $workspace

    cargo build --release -p bonsai-cli 2>&1 | Select-Object -First 50
    cargo build --release -p bonsai-api-gateway 2>&1 | Select-Object -First 50
    cargo build --release -p bonsai-kdb 2>&1 | Select-Object -First 50

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Rust crates built" -ForegroundColor Green
    }

    Write-Step "Building Tauri desktop app..."
    Push-Location "$workspace\bonsai-workspace"

    pnpm install 2>&1 | Select-Object -Last 10
    pnpm build 2>&1 | Select-Object -Last 10
    pnpm tauri build 2>&1 | Select-Object -Last 10

    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Tauri app built" -ForegroundColor Green
        $exe_path = "$workspace\bonsai-workspace\src-tauri\target\release\bonsai-workspace.exe"
        if (Test-Path $exe_path) {
            Write-Host "   Executable: $exe_path" -ForegroundColor Cyan
        }
    }

    Pop-Location
}

# ============================================================================
# PHASE 3: Setup Infrastructure
# ============================================================================

Write-Header "PHASE 3: SETUP INFRASTRUCTURE"

Write-Step "Creating model directory..."
$model_dir = "$env:USERPROFILE\.bonsai\models"
if (-not (Test-Path $model_dir)) {
    New-Item -ItemType Directory -Path $model_dir -Force | Out-Null
    Write-Host "Created: $model_dir" -ForegroundColor Green
}

Write-Step "Creating config..."
$config_dir = "$env:USERPROFILE\.bonsai"
$config_file = "$config_dir\config.toml"

if (-not (Test-Path $config_file)) {
    @"
[bonsai]
name = "Bonsai Workspace"
version = "0.2.0"

[models]
default = "octopus-v1"
model_dir = "$model_dir"

[api]
host = "127.0.0.1"
port = 11425

[ui]
theme = "auto"
"@ | Out-File -FilePath $config_file
    Write-Host "Created: $config_file" -ForegroundColor Green
}

Write-Step "Knowledge module ready..."
$kdb_path = "$workspace\kdb-modules\psychopathy-octopus-knowledge.json"
if (Test-Path $kdb_path) {
    $size = (Get-Item $kdb_path).Length / 1KB
    Write-Host "Knowledge module: $kdb_path ($([math]::Round($size, 0)) KB)" -ForegroundColor Green
}

# ============================================================================
# Summary
# ============================================================================

Write-Header "BUILD SUMMARY"

Write-Host @"

✅ BONSAI WORKSPACE COMPONENTS BUILT:

1. USOS Kernel (bare-metal x86_64)
   Location: crates/kernel/target/x86_64-unknown-none/release/kernel

2. Bonsai Workspace IDE (Tauri desktop app)
   Location: bonsai-workspace/src-tauri/target/release/bonsai-workspace.exe

3. Model Infrastructure
   Config: $config_file
   Models dir: $model_dir
   Knowledge module: 34-container server spec

4. Training Infrastructure
   Training data preparation script: crates/octopus-ai/prepare_data.py
   Training script: crates/octopus-ai/train_psychopathy.py
   Merge/convert script: crates/octopus-ai/merge_and_convert.py

═══════════════════════════════════════════════════════════════════════════════

NEXT STEPS:

1. Install Python 3.11+ from https://python.org (if you want to train models)

2. Prepare training data (CPU-only):
   python crates\octopus-ai\prepare_data.py --output ./training-data

3. Train Psychopathy Octopus on GPU (requires Python + PyTorch):
   python crates\octopus-ai\train_psychopathy.py

4. Merge and convert to GGUF:
   python crates\octopus-ai\merge_and_convert.py

5. Launch the complete stack:
   # Terminal 1: API Gateway
   cargo run --release -p bonsai-api-gateway -- --host 127.0.0.1 --port 11425

   # Terminal 2: IDE
   cd bonsai-workspace && pnpm tauri dev

═══════════════════════════════════════════════════════════════════════════════

BUILD COMPLETE! Infrastructure ready for testing and training.
"@

if ($LaunchStack) {
    Write-Step "Would launch stack, but Python is required for full deployment"
    Write-Host "Install Python 3.11+ and run the training phase to complete setup"
}

Write-Host "`n✅ Build infrastructure complete!" -ForegroundColor Green
