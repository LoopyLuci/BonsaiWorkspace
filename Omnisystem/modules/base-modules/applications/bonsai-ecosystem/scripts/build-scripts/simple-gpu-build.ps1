#!/usr/bin/env pwsh
# Simple GPU Build Script - Direct and straightforward

param([switch]$LaunchStack)

$workspace = "Z:\Projects\BonsaiEcosystem"

Write-Host @'
================================================================================
BONSAI ECOSYSTEM - COMPLETE GPU BUILD
================================================================================
'@ -ForegroundColor Cyan

# Set Python path
$env:PATH = "C:\Program Files\Python311;$env:PATH"

# Verify Python
python --version
Write-Host ""

# Phase 1: Kernel
Write-Host "PHASE 1: Building USOS Kernel" -ForegroundColor Green
Push-Location "$workspace\crates\kernel"
rustup target add x86_64-unknown-none
cargo build --release --target x86_64-unknown-none 2>&1 | Tee-Object -FilePath "$workspace\phase1.log"
Pop-Location
Write-Host "Kernel build complete`n" -ForegroundColor Green

# Phase 2: IDE
Write-Host "PHASE 2: Building Bonsai Workspace IDE" -ForegroundColor Green
Push-Location $workspace
cargo build --release -p bonsai-cli 2>&1 | Tee-Object -FilePath "$workspace\phase2.log"
cargo build --release -p bonsai-api-gateway 2>&1 | Tee-Object -Append -FilePath "$workspace\phase2.log"
cargo build --release -p bonsai-kdb 2>&1 | Tee-Object -Append -FilePath "$workspace\phase2.log"

Push-Location "$workspace\bonsai-workspace"
pnpm install 2>&1 | Tee-Object -FilePath "$workspace\phase2-frontend.log"
pnpm build 2>&1 | Tee-Object -Append -FilePath "$workspace\phase2-frontend.log"
pnpm tauri build 2>&1 | Tee-Object -Append -FilePath "$workspace\phase2-frontend.log"
Pop-Location
Pop-Location
Write-Host "IDE build complete`n" -ForegroundColor Green

# Phase 3: Data Preparation
Write-Host "PHASE 3: Preparing Training Data" -ForegroundColor Green
Push-Location $workspace
python crates\octopus-ai\prepare_data.py --output ./training-data 2>&1 | Tee-Object -FilePath "$workspace\phase3.log"
Pop-Location
Write-Host "Data preparation complete`n" -ForegroundColor Green

# Phase 4: GPU Training
Write-Host "PHASE 4: GPU Training (Watch the loss decrease!)" -ForegroundColor Yellow
Write-Host "Open Task Manager (Ctrl+Shift+Esc) and watch GPU utilization" -ForegroundColor Yellow
Write-Host "Performance > GPU > Should show 85-95% during training`n" -ForegroundColor Yellow

python -m pip install torch-directml transformers datasets peft accelerate bitsandbytes --quiet 2>&1 | Out-Null

Push-Location $workspace
python crates\octopus-ai\train_psychopathy.py 2>&1 | Tee-Object -FilePath "$workspace\training.log"
Pop-Location

if ($LASTEXITCODE -eq 0) {
    Write-Host "Training complete!`n" -ForegroundColor Green
} else {
    Write-Host "Training may have failed. Check training.log`n" -ForegroundColor Red
}

# Phase 5: Merge & Convert
Write-Host "PHASE 5: Merging LoRA and Converting to GGUF" -ForegroundColor Green

if (-not (Test-Path "llama.cpp")) {
    Write-Host "Cloning llama.cpp..." -ForegroundColor Blue
    git clone https://github.com/ggerganov/llama.cpp 2>&1 | Out-Null
}

if (Test-Path "llama.cpp/Makefile") {
    Write-Host "Building llama.cpp..." -ForegroundColor Blue
    Push-Location llama.cpp
    make 2>&1 | Out-Null
    Pop-Location
}

Push-Location $workspace
python crates\octopus-ai\merge_and_convert.py 2>&1 | Tee-Object -FilePath "$workspace\merge-convert.log"
Pop-Location

if (Test-Path "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf") {
    $size = (Get-Item "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf").Length / 1MB
    Write-Host "Model ready: psychopathy-octopus-v1.Q4_K_M.gguf ($([math]::Round($size, 1)) MB)`n" -ForegroundColor Green
} else {
    Write-Host "Model file not found - check merge-convert.log`n" -ForegroundColor Red
}

# Phase 6: Setup and optionally launch
Write-Host "PHASE 6: Setup Complete" -ForegroundColor Green

$model_dir = "$env:USERPROFILE\.bonsai\models"
if (-not (Test-Path $model_dir)) {
    New-Item -ItemType Directory -Path $model_dir -Force | Out-Null
}

$config_dir = "$env:USERPROFILE\.bonsai"
if (-not (Test-Path "$config_dir\config.toml")) {
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
"@ | Out-File -FilePath "$config_dir\config.toml"
}

Write-Host @"
================================================================================
SUCCESS! BONSAI ECOSYSTEM COMPLETE
================================================================================

PSYCHOPATHY OCTOPUS MODEL IS TRAINED AND READY!

Model location:
  $workspace\psychopathy-octopus-v1.Q4_K_M.gguf

Build logs:
  Phase 1 (Kernel):    $workspace\phase1.log
  Phase 2 (IDE):       $workspace\phase2.log
  Phase 3 (Data):      $workspace\phase3.log
  Phase 4 (Training):  $workspace\training.log
  Phase 5 (Merge):     $workspace\merge-convert.log

To test:
  1. cd bonsai-workspace
  2. pnpm tauri dev
  3. Select octopus-v1 in model selector
  4. Ask: "How do I restart a Docker container?"

================================================================================
"@ -ForegroundColor Green

if ($LaunchStack) {
    Write-Host "Launching IDE..." -ForegroundColor Green
    Push-Location "$workspace\bonsai-workspace"
    pnpm tauri dev
    Pop-Location
}
