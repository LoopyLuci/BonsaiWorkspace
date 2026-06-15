#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Install Python 3.11.9 and all training dependencies
    Downloads directly from python.org if not already installed
#>

Write-Host "🐍 Python 3.11 Installation Script`n" -ForegroundColor Cyan

# Check if Python is already installed
$pythonPath = if (Test-Path "C:\Python311\python.exe") {
    "C:\Python311\python.exe"
} elseif (Test-Path "$env:USERPROFILE\AppData\Local\Programs\Python\Python311\python.exe") {
    "$env:USERPROFILE\AppData\Local\Programs\Python\Python311\python.exe"
} else {
    $null
}

if ($pythonPath) {
    Write-Host "✅ Python already installed at: $pythonPath" -ForegroundColor Green
    & $pythonPath --version
} else {
    Write-Host "Downloading Python 3.11.9 installer..." -ForegroundColor Yellow

    # Download Python installer
    $installerUrl = "https://www.python.org/ftp/python/3.11.9/python-3.11.9-amd64.exe"
    $installerPath = "$env:TEMP\python-3.11.9-amd64.exe"

    Write-Host "Downloading from: $installerUrl`n"

    try {
        Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing -ErrorAction Stop
        Write-Host "✅ Downloaded to: $installerPath`n" -ForegroundColor Green

        Write-Host "Installing Python 3.11.9..." -ForegroundColor Yellow
        Write-Host "  (This will open a GUI installer window)`n"

        # Run installer with options to add to PATH and install for all users
        & $installerPath /passive PrependPath=1 InstallAllUsers=1

        Write-Host "`n✅ Python installed!" -ForegroundColor Green
        Write-Host "Please wait 30 seconds for installation to complete, then run this script again.`n"

        Start-Sleep -Seconds 30

        # Refresh PATH and verify
        $env:PATH = [System.Environment]::GetEnvironmentVariable('PATH','Machine') + ';' + [System.Environment]::GetEnvironmentVariable('PATH','User')

        python --version

    } catch {
        Write-Host "❌ Download failed. Please download manually from:" -ForegroundColor Red
        Write-Host "   https://www.python.org/downloads/release/python-3119/" -ForegroundColor Yellow
        Write-Host "`nDuring installation, make sure to check:" -ForegroundColor Yellow
        Write-Host "   ✓ Add Python to PATH" -ForegroundColor Cyan
        Write-Host "   ✓ Install for all users (recommended)`n" -ForegroundColor Cyan
        exit 1
    }
}

# Now install Python packages
Write-Host "`n📦 Installing training dependencies...`n" -ForegroundColor Cyan

Write-Host "  → Upgrading pip..." -ForegroundColor Blue
python -m pip install --upgrade pip --quiet

Write-Host "  → Installing PyTorch with DirectML (AMD GPU)..." -ForegroundColor Blue
python -m pip install torch-directml --quiet

Write-Host "  → Installing Transformers..." -ForegroundColor Blue
python -m pip install transformers --quiet

Write-Host "  → Installing Datasets..." -ForegroundColor Blue
python -m pip install datasets --quiet

Write-Host "  → Installing PEFT (LoRA)..." -ForegroundColor Blue
python -m pip install peft --quiet

Write-Host "  → Installing Accelerate..." -ForegroundColor Blue
python -m pip install accelerate --quiet

Write-Host "  → Installing BitsAndBytes..." -ForegroundColor Blue
python -m pip install bitsandbytes --quiet

Write-Host "`n✅ All dependencies installed!" -ForegroundColor Green

# Verify installations
Write-Host "`n🧪 Verifying installations...`n" -ForegroundColor Cyan

python -c "import torch; print('  ✓ PyTorch:', torch.__version__)" 2>&1 | Write-Host -ForegroundColor Green
python -c "import transformers; print('  ✓ Transformers:', transformers.__version__)" 2>&1 | Write-Host -ForegroundColor Green
python -c "import peft; print('  ✓ PEFT:', peft.__version__)" 2>&1 | Write-Host -ForegroundColor Green
python -c "import datasets; print('  ✓ Datasets:', datasets.__version__)" 2>&1 | Write-Host -ForegroundColor Green

Write-Host "`n════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "✅ Python environment ready for GPU training!" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

Write-Host "Next step: Run the complete Bonsai build" -ForegroundColor Yellow
Write-Host "  cd Z:\Projects\BonsaiWorkspace" -ForegroundColor Cyan
Write-Host "  .\windows-full-setup.ps1 -LaunchStack`n" -ForegroundColor Cyan
