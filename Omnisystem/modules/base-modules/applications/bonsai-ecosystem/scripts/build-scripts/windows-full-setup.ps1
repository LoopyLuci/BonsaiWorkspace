#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Complete Bonsai Ecosystem Setup for Windows 10
    Builds USOS kernel, Bonsai Workspace, and trains Psychopathy Octopus

.DESCRIPTION
    One-stop script to:
    1. Set up dependencies (Rust, Python, Node.js)
    2. Build USOS bare-metal kernel
    3. Build Bonsai Workspace IDE
    4. Prepare training data
    5. Train Psychopathy Octopus on GPU (RX 7900 XTX)
    6. Merge and convert model to GGUF
    7. Launch complete local stack

.PARAMETER SkipKernelBuild
    Skip USOS kernel compilation

.PARAMETER SkipTraining
    Skip model training (use existing LoRA adapter if available)

.PARAMETER SkipIDE
    Skip Tauri IDE build

.PARAMETER OnlyPrepareData
    Only prepare training data, don't train
#>

param(
    [switch]$SkipKernelBuild,
    [switch]$SkipTraining,
    [switch]$SkipIDE,
    [switch]$OnlyPrepareData,
    [switch]$LaunchStack
)

$workspace = "Z:\Projects\BonsaiEcosystem"
$timestamp = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'

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

function Write-Error {
    param([string]$Text)
    Write-Host "❌ $Text" -ForegroundColor Red
}

function Test-Dependency {
    param([string]$Command, [string]$Name)
    $exists = $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
    if ($exists) {
        Write-Host "  ✅ $Name found" -ForegroundColor Green
    } else {
        Write-Error "$Name not found"
        return $false
    }
    return $true
}

# ============================================================================
# PHASE 1: Check Prerequisites
# ============================================================================

Write-Header "BONSAI ECOSYSTEM — WINDOWS 10 LOCAL BUILD"
Write-Host "Hardware: Ryzen 9 5900X (12C/24T), 64GB RAM, RX 7900 XTX (24GB VRAM)`n"

Write-Step "Checking dependencies..."

$deps_ok = $true
$deps_ok = (Test-Dependency "rustc" "Rust") -and $deps_ok
$deps_ok = (Test-Dependency "cargo" "Cargo") -and $deps_ok
$deps_ok = (Test-Dependency "python" "Python 3.11+") -and $deps_ok
$deps_ok = (Test-Dependency "node" "Node.js") -and $deps_ok
$deps_ok = (Test-Dependency "pnpm" "pnpm") -and $deps_ok

if (-not $deps_ok) {
    Write-Error "Missing dependencies. Please install:"
    Write-Host "  Rust: https://rustup.rs"
    Write-Host "  Python 3.11: https://python.org"
    Write-Host "  Node.js: https://nodejs.org"
    Write-Host "  pnpm: npm install -g pnpm"
    exit 1
}

# Check GPU
Write-Step "Checking GPU..."
$gpu = Get-WmiObject Win32_VideoController | Where-Object { $_.Name -like "*7900*" }
if ($gpu) {
    Write-Host "  ✅ Found: $($gpu.Name) - $($gpu.AdapterRam / 1GB) GB VRAM" -ForegroundColor Green
} else {
    Write-Host "  ⚠️  RX 7900 XTX not detected. GPU training will be slower." -ForegroundColor Yellow
}

# ============================================================================
# PHASE 2: Build USOS Kernel
# ============================================================================

if (-not $SkipKernelBuild) {
    Write-Header "PHASE 2: BUILD USOS KERNEL"

    Write-Step "Building USOS kernel (bare-metal x86_64)..."

    Push-Location "$workspace\crates\kernel"

    # Add x86_64-unknown-none target
    cargo build --release --target x86_64-unknown-none 2>&1 | Tee-Object -FilePath "$workspace\usos-build.log"

    if ($LASTEXITCODE -eq 0) {
        $kernel_path = "$workspace\crates\kernel\target\x86_64-unknown-none\release\kernel"
        Write-Host "  ✅ Kernel built: $kernel_path" -ForegroundColor Green

        Write-Step "Testing kernel with QEMU..."
        Write-Host "  (QEMU must be installed: choco install qemu or download from qemu.org)" -ForegroundColor Yellow
        Write-Host "  Test command: qemu-system-x86_64 -kernel $kernel_path" -ForegroundColor Cyan
    } else {
        Write-Error "Kernel build failed"
    }

    Pop-Location
}

# ============================================================================
# PHASE 3: Build Bonsai Workspace IDE
# ============================================================================

if (-not $SkipIDE) {
    Write-Header "PHASE 3: BUILD BONSAI WORKSPACE IDE"

    Write-Step "Building Rust crates..."
    Push-Location $workspace

    cargo build --release -p bonsai-cli 2>&1 | Tee-Object -FilePath "$workspace\rust-build.log"
    cargo build --release -p bonsai-api-gateway 2>&1 | Tee-Object -Append -FilePath "$workspace\rust-build.log"
    cargo build --release -p bonsai-kdb 2>&1 | Tee-Object -Append -FilePath "$workspace\rust-build.log"

    if ($LASTEXITCODE -ne 0) {
        Write-Error "Rust build failed"
        Pop-Location
        exit 1
    }

    Write-Host "  ✅ Rust crates built" -ForegroundColor Green

    Write-Step "Building Tauri desktop app..."
    Push-Location "$workspace\bonsai-workspace"

    pnpm install 2>&1 | Tee-Object -FilePath "$workspace\frontend-build.log"
    pnpm build 2>&1 | Tee-Object -Append -FilePath "$workspace\frontend-build.log"
    pnpm tauri build 2>&1 | Tee-Object -Append -FilePath "$workspace\frontend-build.log"

    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✅ Tauri app built" -ForegroundColor Green
        $exe_path = "$workspace\bonsai-workspace\src-tauri\target\release\bonsai-workspace.exe"
        Write-Host "  Executable: $exe_path" -ForegroundColor Cyan
    } else {
        Write-Error "Frontend build failed"
    }

    Pop-Location
}

# ============================================================================
# PHASE 4: Prepare Training Data
# ============================================================================

Write-Header "PHASE 4: PREPARE TRAINING DATA"

Write-Step "Generating 1.6M training examples..."

Push-Location $workspace
python crates\octopus-ai\prepare_data.py `
    --output ./training-data `
    --count 1600000 `
    2>&1 | Tee-Object -FilePath "$workspace\prepare-data.log"

if ($LASTEXITCODE -eq 0) {
    Write-Host "  ✅ Training data prepared" -ForegroundColor Green
} else {
    Write-Error "Data preparation failed"
}

# If only data prep requested, exit here
if ($OnlyPrepareData) {
    Write-Host "`n✅ Training data ready at $workspace\training-data" -ForegroundColor Green
    exit 0
}

# ============================================================================
# PHASE 5: Train Psychopathy Octopus
# ============================================================================

if (-not $SkipTraining) {
    Write-Header "PHASE 5: TRAIN PSYCHOPATHY OCTOPUS ON GPU"

    Write-Step "Installing PyTorch with GPU support..."

    # For AMD GPU on Windows, use DirectML (native Windows ML acceleration)
    pip install torch-directml 2>&1 | Out-Null

    Write-Step "Installing training dependencies..."
    pip install transformers datasets peft accelerate bitsandbytes 2>&1 | Out-Null

    Write-Step "Starting GPU training (RX 7900 XTX, 24GB VRAM)..."
    Write-Host "  Expected duration: 4-6 hours" -ForegroundColor Yellow
    Write-Host "  Model: TinyLlama 1.1B (QLoRA with rank-16)" -ForegroundColor Cyan

    python crates\octopus-ai\train_psychopathy.py `
        2>&1 | Tee-Object -FilePath "$workspace\training.log"

    if ($LASTEXITCODE -eq 0) {
        Write-Host "  ✅ Training complete" -ForegroundColor Green

        Write-Step "Merging LoRA adapter and converting to GGUF..."

        # Install llama.cpp if not present
        if (-not (Test-Path "llama.cpp")) {
            Write-Host "  Cloning llama.cpp..." -ForegroundColor Blue
            git clone https://github.com/ggerganov/llama.cpp
        }

        python crates\octopus-ai\merge_and_convert.py `
            2>&1 | Tee-Object -FilePath "$workspace\merge-convert.log"

        if ($LASTEXITCODE -eq 0) {
            $gguf_path = "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf"
            if (Test-Path $gguf_path) {
                $size_mb = (Get-Item $gguf_path).Length / 1MB
                Write-Host "  ✅ GGUF model ready: $gguf_path ($([math]::Round($size_mb, 1)) MB)" -ForegroundColor Green
            }
        }
    } else {
        Write-Error "Training failed - check $workspace\training.log"
    }
}

# ============================================================================
# PHASE 6: Setup Model Directory
# ============================================================================

Write-Header "PHASE 6: SETUP MODEL DIRECTORY"

Write-Step "Creating model directory..."

$model_dir = "$env:USERPROFILE\.bonsai\models"
if (-not (Test-Path $model_dir)) {
    New-Item -ItemType Directory -Path $model_dir -Force | Out-Null
    Write-Host "  Created: $model_dir" -ForegroundColor Green
}

# Copy GGUF model if available
$gguf_src = "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf"
if (Test-Path $gguf_src) {
    Copy-Item -Path $gguf_src -Destination "$model_dir\psychopathy-octopus-v1.Q4_K_M.gguf" -Force
    Write-Host "  ✅ Model copied to $model_dir" -ForegroundColor Green
}

# ============================================================================
# PHASE 7: Launch Local Stack
# ============================================================================

Write-Header "COMPLETE LOCAL STACK"

Write-Host @"

🎯 YOUR BONSAI ECOSYSTEM IS READY

Location:         $workspace
GPU Model:        RX 7900 XTX (24 GB)
CPU:              Ryzen 9 5900X (12C/24T)
RAM:              64 GB
Training Data:    1.6M examples
Kernel:           USOS x86_64 bare-metal
Model:            Psychopathy Octopus (TinyLlama 1.1B LoRA)
IDE:              Tauri (native Windows app)

═══════════════════════════════════════════════════════════════════════════════

TO LAUNCH THE COMPLETE STACK:

1. Start the inference API:
   cargo run --release -p bonsai-api-gateway -- --host 127.0.0.1 --port 11425

2. Start the MCP tools server (in another terminal):
   cargo run --release -p bonsai-cli -- mcp serve --port 7780

3. Start the KDB server (in another terminal):
   cargo run --release -p bonsai-kdb -- serve --port 8089

4. Launch the Bonsai Workspace IDE (in another terminal):
   cd bonsai-workspace && pnpm tauri dev

5. Test the model:
   curl http://127.0.0.1:11425/v1/chat/completions `
     -H "Content-Type: application/json" `
     -d '{"model":"psychopathy-octopus-v1","messages":[{"role":"user","content":"How do I restart a Docker container?"}]}'

═══════════════════════════════════════════════════════════════════════════════

TO EMULATE NIXOS:

Option 1: Docker (lightweight)
  docker run -d --name nixos-test nixos/nix:latest

Option 2: QEMU full VM (realistic)
  - Download: https://channels.nixos.org/nixos-24.11/latest-nixos-minimal-x86_64-linux.iso
  - Create disk: qemu-img create -f qcow2 nixos.qcow2 40G
  - Boot: qemu-system-x86_64 -m 16G -smp 8 -drive file=nixos.qcow2 -cdrom nixos.iso

═══════════════════════════════════════════════════════════════════════════════

NIGHTLY IMPROVEMENT (Optional):

Edit windows-improve-octopus.ps1 and schedule it daily:
  powershell -File Z:\Projects\BonsaiEcosystem\windows-improve-octopus.ps1

Or run manually anytime to fine-tune from feedback.

═══════════════════════════════════════════════════════════════════════════════

📊 BUILD LOGS:
  Kernel:       $workspace\usos-build.log
  Rust crates:  $workspace\rust-build.log
  Frontend:     $workspace\frontend-build.log
  Training:     $workspace\training.log
  Merge/Conv:   $workspace\merge-convert.log

✅ Setup complete at $timestamp
"@

if ($LaunchStack) {
    Write-Step "Launching local stack..."
    Write-Host "  Opening IDE..."
    Start-Process "$workspace\bonsai-workspace\src-tauri\target\release\bonsai-workspace.exe"
}

Write-Host "`n✅ Ready to deploy to your friend's NixOS server when testing is complete!" -ForegroundColor Green
