#!/usr/bin/env pwsh
# Train Psychopathy Octopus in isolated Python venv with live output

$workspace = "Z:\Projects\BonsaiWorkspace"
$venv = "$workspace\.venv-ml"

Write-Host @"
════════════════════════════════════════════════════════════════════════════════
🐍 SETTING UP CLEAN PYTHON VENV FOR ML TRAINING
════════════════════════════════════════════════════════════════════════════════

Workspace: $workspace
Venv: $venv

"@ -ForegroundColor Cyan

# Step 1: Create venv
Write-Host "📦 Creating Python venv..." -ForegroundColor Green
python -m venv $venv

# Step 2: Activate venv
Write-Host "✅ Activating venv..." -ForegroundColor Green
& "$venv\Scripts\Activate.ps1"

# Step 3: Install dependencies with compatible versions
Write-Host @"

📥 Installing ML stack (this takes ~2-3 minutes)...
   - torch 2.1.0
   - transformers 4.35.0
   - datasets 2.14.0
   - peft 0.7.0
   - accelerate 0.24.0
   - blake3
"@ -ForegroundColor Yellow

pip install --upgrade pip
pip install torch==2.1.0
pip install transformers==4.35.0
pip install datasets==2.14.0
pip install peft==0.7.0
pip install accelerate==0.24.0
pip install blake3

Write-Host "`n✅ Python environment ready!`n" -ForegroundColor Green

# Step 4: Phase 3 - Data Preparation
Write-Host "════════════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "PHASE 3: PREPARING TRAINING DATA (1-2 hours, CPU)" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

Push-Location $workspace

python crates\octopus-ai\prepare_data.py --output ./training-data

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ PHASE 3 COMPLETE - Training data ready`n" -ForegroundColor Green
} else {
    Write-Host "`n❌ PHASE 3 FAILED`n" -ForegroundColor Red
    exit 1
}

# Step 5: Phase 4 - GPU Training
Write-Host "════════════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "PHASE 4: GPU TRAINING - PSYCHOPATHY OCTOPUS (4-6 hours, RX 7900 XTX)" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host @"
📊 WATCH GPU USAGE:
   - Open Task Manager (Ctrl+Shift+Esc)
   - Performance tab → GPU
   - Watch RX 7900 XTX climb to 85-95%

📈 TRAINING OUTPUT:
   - Step N/600: loss=X, lr=2e-04
   - Loss should decrease from 4.23 to <1.5
   - New checkpoint every 100 steps

⏱️  Expected duration: 4-6 hours on your RX 7900 XTX

`@ -ForegroundColor Yellow

python crates\octopus-ai\train_psychopathy.py

if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ PHASE 4 COMPLETE - Model trained`n" -ForegroundColor Green
} else {
    Write-Host "`n❌ PHASE 4 FAILED`n" -ForegroundColor Red
    exit 1
}

# Step 6: Phase 5 - Merge & Convert
Write-Host "════════════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "PHASE 5: MERGING LORA & CONVERTING TO GGUF (30 min)" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

if (-not (Test-Path "llama.cpp")) {
    Write-Host "📥 Cloning llama.cpp..." -ForegroundColor Blue
    git clone https://github.com/ggerganov/llama.cpp
}

python crates\octopus-ai\merge_and_convert.py

if (Test-Path "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf") {
    $size = (Get-Item "$workspace\psychopathy-octopus-v1.Q4_K_M.gguf").Length / 1MB
    Write-Host "`n✅ PHASE 5 COMPLETE`n" -ForegroundColor Green
    Write-Host "🎉 MODEL READY: psychopathy-octopus-v1.Q4_K_M.gguf ($([math]::Round($size, 1)) MB)`n" -ForegroundColor Green
} else {
    Write-Host "`n❌ PHASE 5 FAILED - Model not created`n" -ForegroundColor Red
    exit 1
}

Pop-Location

# Summary
Write-Host @"
════════════════════════════════════════════════════════════════════════════════
🐙 COMPLETE BONSAI ECOSYSTEM TRAINED & READY
════════════════════════════════════════════════════════════════════════════════

✅ Python venv: $venv
✅ Training data: $workspace\training-data\
✅ Trained model: $workspace\psychopathy-octopus-v1.Q4_K_M.gguf
✅ Knowledge module: $workspace\kdb-modules\psychopathy-octopus-knowledge.json
✅ Rust ecosystem: $workspace\target\release\

NEXT STEPS:
1. Copy model to ~/.bonsai/models/psychopathy-octopus-v1.Q4_K_M.gguf
2. Launch Bonsai Workspace IDE
3. Select octopus-v1 in model dropdown
4. Test with server management questions

DEPLOYMENT:
scp psychopathy-octopus-v1.Q4_K_M.gguf user@server:/var/lib/bonsai/models/

════════════════════════════════════════════════════════════════════════════════
"@ -ForegroundColor Green

Write-Host "To keep venv activated, type: & '$venv\Scripts\Activate.ps1'" -ForegroundColor Yellow
