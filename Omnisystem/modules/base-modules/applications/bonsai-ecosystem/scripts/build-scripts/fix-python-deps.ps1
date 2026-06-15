#!/usr/bin/env pwsh
# Fix Python dependencies for training

$env:PATH = "C:\Program Files\Python311;$env:PATH"

Write-Host "Fixing Python dependencies..." -ForegroundColor Green

# Ensure pip is up to date
python -m pip install --upgrade pip --quiet

# Install specific compatible versions
Write-Host "Installing PyTorch with DirectML..." -ForegroundColor Blue
python -m pip install torch==2.0.1 torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu --quiet

Write-Host "Installing HuggingFace Transformers (compatible version)..." -ForegroundColor Blue
python -m pip install transformers==4.30.0 --quiet

Write-Host "Installing PEFT (LoRA)..." -ForegroundColor Blue
python -m pip install peft==0.4.0 --quiet

Write-Host "Installing Datasets..." -ForegroundColor Blue
python -m pip install datasets --quiet

Write-Host "Installing Accelerate..." -ForegroundColor Blue
python -m pip install accelerate --quiet

Write-Host "Installing blake3 (for data dedup)..." -ForegroundColor Blue
python -m pip install blake3 --quiet

Write-Host "Installing bitsandbytes (quantization)..." -ForegroundColor Blue
python -m pip install bitsandbytes --quiet

# Verify
Write-Host "`nVerifying installations..." -ForegroundColor Cyan
python -c "import torch; print(f'PyTorch: {torch.__version__}')"
python -c "import transformers; print(f'Transformers: {transformers.__version__}')"
python -c "import peft; print(f'PEFT: {peft.__version__}')"
python -c "import blake3; print('blake3: OK')"

Write-Host "`n✅ Python environment fixed!" -ForegroundColor Green
