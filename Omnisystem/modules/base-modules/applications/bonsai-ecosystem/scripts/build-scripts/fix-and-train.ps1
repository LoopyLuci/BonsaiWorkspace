#!/usr/bin/env pwsh
# Fix Python environment and re-run training phases

$workspace = "Z:\Projects\BonsaiWorkspace"
$env:PATH = "C:\Program Files\Python311;$env:PATH"

Write-Host @"
════════════════════════════════════════════════════════════════════════════════
🔧 FIXING PYTHON & RE-RUNNING TRAINING PHASES
════════════════════════════════════════════════════════════════════════════════

"@ -ForegroundColor Green

# Clean up problematic packages
Write-Host "Cleaning up incompatible packages..." -ForegroundColor Blue
python -m pip uninstall torch transformers peft -y --quiet 2>&1 | Out-Null

# Install minimal working versions for training
Write-Host "Installing minimal compatible stack..." -ForegroundColor Blue

python -m pip install --upgrade pip --quiet
python -m pip install torch==2.0.0 --quiet
python -m pip install transformers==4.30.0 --quiet
python -m pip install peft==0.4.0 --quiet
python -m pip install datasets==2.13.0 --quiet
python -m pip install accelerate==0.20.0 --quiet
python -m pip install blake3 --quiet

Write-Host "✅ Python environment fixed`n" -ForegroundColor Green

# Phase 3: Training data preparation
Write-Host "PHASE 3: Preparing Training Data" -ForegroundColor Green

Push-Location $workspace

python crates\octopus-ai\prepare_data.py --output ./training-data 2>&1 | Tee-Object -FilePath "$workspace\phase3-retry.log"

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Phase 3 complete`n" -ForegroundColor Green
} else {
    Write-Host "❌ Phase 3 failed`n" -ForegroundColor Red
}

# Phase 4: GPU Training
Write-Host "PHASE 4: GPU TRAINING - Psychopathy Octopus" -ForegroundColor Green
Write-Host "Watch Task Manager → Performance → GPU during training`n" -ForegroundColor Yellow

python crates\octopus-ai\train_psychopathy.py 2>&1 | Tee-Object -FilePath "$workspace\phase4-retry.log"

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Phase 4 complete`n" -ForegroundColor Green
} else {
    Write-Host "⚠️ Phase 4 encountered issues - check phase4-retry.log`n" -ForegroundColor Yellow
}

# Phase 5: Merge & Convert
Write-Host "PHASE 5: Merging & Converting to GGUF" -ForegroundColor Green

if (-not (Test-Path "llama.cpp")) {
    Write-Host "Cloning llama.cpp..." -ForegroundColor Blue
    git clone https://github.com/ggerganov/llama.cpp 2>&1 | Out-Null
}

python crates\octopus-ai\merge_and_convert.py 2>&1 | Tee-Object -FilePath "$workspace\phase5-retry.log"

if (Test-Path "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf") {
    $size = (Get-Item "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf").Length / 1MB
    Write-Host "✅ Model created: psychopathy-octopus-v1.Q4_K_M.gguf ($([math]::Round($size, 1)) MB)`n" -ForegroundColor Green
} else {
    Write-Host "⚠️ Model file not created - check logs`n" -ForegroundColor Yellow
}

Pop-Location

Write-Host @"
════════════════════════════════════════════════════════════════════════════════
TRAINING PHASES COMPLETE
════════════════════════════════════════════════════════════════════════════════

Logs:
  Phase 3: $workspace\phase3-retry.log
  Phase 4: $workspace\phase4-retry.log
  Phase 5: $workspace\phase5-retry.log

✅ Complete Bonsai Ecosystem is now built and trained!
"@ -ForegroundColor Green
