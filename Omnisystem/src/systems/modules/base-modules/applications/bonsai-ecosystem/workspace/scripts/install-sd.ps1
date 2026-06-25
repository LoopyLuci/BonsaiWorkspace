<#
.SYNOPSIS
    Install the Stable Diffusion image generation pipeline for Bonsai (AMD/DirectML).

.DESCRIPTION
    1. Creates a dedicated Python venv at %LOCALAPPDATA%\com.bonsai.workspace\sd_venv.
    2. Installs torch-directml + diffusers + dependencies into that venv.
    3. Copies scripts/sd_generate.py to %LOCALAPPDATA%\com.bonsai.workspace\scripts\.
    4. Optionally downloads a model from HuggingFace (-DownloadModel).

    Supports: HuggingFace directories, .safetensors, .ckpt, and GGUF files.
    SDXL GGUF models (e.g. sdxl_base_1.0_Q*.gguf) are auto-detected and run at 1024x1024.

    The pipeline is 100% offline once the model is on disk.

.PARAMETER PythonExe
    Path to Python 3.10+. Default: auto-detected (py, python3, python).

.PARAMETER VenvDir
    SD venv directory. Default: %LOCALAPPDATA%\com.bonsai.workspace\sd_venv

.PARAMETER ScriptsDir
    Where to deploy sd_generate.py. Default: %LOCALAPPDATA%\com.bonsai.workspace\scripts

.PARAMETER DownloadModel
    Download model weights from HuggingFace. WARNING: several GB download.

.PARAMETER ModelId
    HuggingFace model ID (used only with -DownloadModel).
    Default: runwayml/stable-diffusion-v1-5

.PARAMETER ModelDir
    Local model path to use instead of HuggingFace (any supported format).

.EXAMPLE
    # Install packages only (no download):
    .\install-sd.ps1

    # Install and download SD 1.5 (~4 GB):
    .\install-sd.ps1 -DownloadModel

    # Use local SDXL GGUF already on disk:
    .\install-sd.ps1 -ModelDir "D:\Models\sdxl_base_1.0_Q5_K_S.gguf"
#>

param(
    [string]$PythonExe  = "",
    [string]$VenvDir    = "$env:LOCALAPPDATA\com.bonsai.workspace\sd_venv",
    [string]$ScriptsDir = "$env:LOCALAPPDATA\com.bonsai.workspace\scripts",
    [switch]$DownloadModel,
    [string]$ModelId    = "runwayml/stable-diffusion-v1-5",
    [string]$ModelDir   = ""
)

Set-StrictMode -Version 3
$ErrorActionPreference = "Stop"

# ── Find Python ───────────────────────────────────────────────────────────────

function Find-Python {
    foreach ($candidate in @($PythonExe, "py", "python3", "python")) {
        if ([string]::IsNullOrWhiteSpace($candidate)) { continue }
        try {
            $ver = & $candidate --version 2>&1
            if ($ver -match "Python 3\.(\d+)") {
                $minor = [int]$Matches[1]
                if ($minor -lt 10) { Write-Warning "$candidate is Python 3.$minor — need 3.10+"; continue }
                Write-Host "[sd] Found Python: $candidate ($ver)" -ForegroundColor Cyan
                return $candidate
            }
        } catch { }
    }
    Write-Error "Python 3.10+ not found. Install from https://www.python.org/downloads/"
    exit 1
}

$python = Find-Python

# ── Create / reuse venv ───────────────────────────────────────────────────────

$venvPy = "$VenvDir\Scripts\python.exe"
$pip    = "$VenvDir\Scripts\pip.exe"

if (-not (Test-Path $venvPy)) {
    Write-Host "[sd] Creating venv at $VenvDir ..."
    & $python -m venv $VenvDir
    if ($LASTEXITCODE -ne 0) { Write-Error "venv creation failed"; exit 1 }
}
Write-Host "[sd] Venv: $venvPy" -ForegroundColor Cyan

# ── Install packages ──────────────────────────────────────────────────────────

Write-Host "[sd] Installing torch-directml + diffusers (may take a few minutes)..."
# Upgrade pip silently (ignore the self-upgrade restriction notice)
& $pip install --upgrade pip --quiet 2>$null
& $pip install torch-directml diffusers transformers accelerate Pillow --quiet
if ($LASTEXITCODE -ne 0) { Write-Error "pip install failed"; exit 1 }
Write-Host "[sd] Packages installed." -ForegroundColor Green

# ── Deploy sd_generate.py ─────────────────────────────────────────────────────

New-Item -ItemType Directory -Force $ScriptsDir | Out-Null

# The canonical script lives in the repo at scripts/sd_generate.py.
# Copy it to AppData so image_generation.rs can find it at runtime.
$repoScript = Join-Path $PSScriptRoot "sd_generate.py"
$destScript  = "$ScriptsDir\sd_generate.py"

if (Test-Path $repoScript) {
    Copy-Item $repoScript $destScript -Force
    Write-Host "[sd] Deployed: $destScript" -ForegroundColor Green
} else {
    Write-Warning "[sd] sd_generate.py not found next to this script ($repoScript). The endpoint will not work until you deploy it manually."
}

# ── Optional model download ───────────────────────────────────────────────────

if ($DownloadModel) {
    Write-Host "`n[sd] Downloading $ModelId (several GB — Ctrl+C to cancel)..." -ForegroundColor Yellow
    & $venvPy -c "
from diffusers import StableDiffusionPipeline
import torch
print('[sd] Caching model from HuggingFace...')
StableDiffusionPipeline.from_pretrained('$ModelId', torch_dtype=torch.float16, safety_checker=None)
print('[sd] Done.')
"
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Download failed or cancelled. Re-run with -DownloadModel to retry."
    } else {
        Write-Host "[sd] Model $ModelId cached." -ForegroundColor Green
    }
} else {
    $modelHint = if ($ModelDir) { $ModelDir } else { "D:\Models\sdxl_base_1.0_Q5_K_S.gguf (or any .gguf/.safetensors)" }
    Write-Host "`n[sd] No model downloaded. Pass model_path in the API body, or run:" -ForegroundColor Yellow
    Write-Host "  .\install-sd.ps1 -ModelDir `"$modelHint`"" -ForegroundColor Yellow
}

# ── Smoke test ────────────────────────────────────────────────────────────────

Write-Host "`n[sd] Verifying imports..."
$ok = & $venvPy -c "import torch_directml, diffusers, transformers; print('OK')" 2>&1
if ($ok -match "OK") {
    Write-Host "[sd] All packages import successfully." -ForegroundColor Green
} else {
    Write-Warning "[sd] Import check failed: $ok"
}

Write-Host @"

[sd] Installation complete.
  Venv   : $VenvDir
  Script : $destScript

API endpoint (requires a model path):
  POST http://127.0.0.1:<port>/api/v1/images/generate
  Body : {
    "prompt": "a sunset over mountains",
    "model_path": "D:\\Models\\sdxl_base_1.0_Q5_K_S.gguf",
    "steps": 20,
    "output_path": "C:\\tmp\\out.png"
  }

Quick CLI test (if model is available):
  & '$venvPy' '$destScript' --model '$(if ($ModelDir) { $ModelDir } else { $ModelId })' --prompt 'a sunset' --output C:\tmp\test.png
"@ -ForegroundColor Cyan
