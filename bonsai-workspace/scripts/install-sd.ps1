<#
.SYNOPSIS
    Install the Stable Diffusion image generation pipeline for Bonsai (AMD/DirectML).

.DESCRIPTION
    1. Installs torch-directml + diffusers into the Bonsai SD virtual environment.
    2. Writes sd_generate.py to %LOCALAPPDATA%\com.bonsai.workspace\scripts\.
    3. Optionally downloads a small default model (Stable Diffusion 1.5 via diffusers
       from a user-specified local path, OR uses any already-cached HuggingFace model).

    The pipeline is 100% offline once the model is cached.
    This script only downloads the Python packages and the model weights on first run.

    IMPORTANT: The user must explicitly choose to download a model (--DownloadModel).
    By default this script only installs packages and writes the script.

.PARAMETER PythonExe
    Path to the Python 3.10+ executable. Default: auto-detected (py, python3, python).

.PARAMETER VenvDir
    Directory for the SD virtual environment.
    Default: %LOCALAPPDATA%\com.bonsai.workspace\sd_venv

.PARAMETER ScriptsDir
    Where to write sd_generate.py.
    Default: %LOCALAPPDATA%\com.bonsai.workspace\scripts

.PARAMETER DownloadModel
    If set, downloads the specified model from HuggingFace on first run.
    WARNING: This requires internet access and may download several GB.

.PARAMETER ModelId
    HuggingFace model ID to cache (used only with -DownloadModel).
    Default: runwayml/stable-diffusion-v1-5

.PARAMETER ModelDir
    Path to a local model directory or safetensors file to use instead of HF.
    Writes this path into sd_generate.py as the default model.

.EXAMPLE
    # Install packages + script only (no model download):
    .\install-sd.ps1

    # Install and download SD 1.5 weights (~4 GB):
    .\install-sd.ps1 -DownloadModel

    # Use a local model you already have:
    .\install-sd.ps1 -ModelDir "D:\Models\stable-diffusion-v1-5"
#>

param(
    [string]$PythonExe   = "",
    [string]$VenvDir     = "$env:LOCALAPPDATA\com.bonsai.workspace\sd_venv",
    [string]$ScriptsDir  = "$env:LOCALAPPDATA\com.bonsai.workspace\scripts",
    [switch]$DownloadModel,
    [string]$ModelId     = "runwayml/stable-diffusion-v1-5",
    [string]$ModelDir    = ""
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

if (-not (Test-Path $venvPy)) {
    Write-Host "[sd] Creating virtual environment at $VenvDir ..."
    & $python -m venv $VenvDir
    if ($LASTEXITCODE -ne 0) { Write-Error "venv creation failed"; exit 1 }
}

Write-Host "[sd] Using venv: $venvPy" -ForegroundColor Cyan
$pip = "$VenvDir\Scripts\pip.exe"

# ── Install packages ──────────────────────────────────────────────────────────

Write-Host "[sd] Installing torch-directml + diffusers (this may take a few minutes)..."

# torch-directml wheels are on PyPI — no special index needed
& $pip install --upgrade pip --quiet
& $pip install torch-directml diffusers transformers accelerate Pillow --quiet
if ($LASTEXITCODE -ne 0) { Write-Error "pip install failed"; exit 1 }

Write-Host "[sd] Packages installed." -ForegroundColor Green

# ── Write sd_generate.py ──────────────────────────────────────────────────────

New-Item -ItemType Directory -Force $ScriptsDir | Out-Null

$defaultModel = if ($ModelDir) { $ModelDir.Replace('\', '/') } else { $ModelId }

$scriptContent = @"
#!/usr/bin/env python3
"""Bonsai Stable Diffusion generator — AMD DirectML backend.

Called by image_generation.rs with arguments:
    --model <path_or_hf_id>
    --prompt <text>
    --output <path.png>
    --width  <int>
    --height <int>
    --steps  <int>
    --guidance <float>
    [--negative_prompt <text>]

Outputs the PNG to --output. Exits 0 on success, non-zero on error.
"""

import argparse
import sys
import os

def parse_args():
    p = argparse.ArgumentParser()
    p.add_argument("--model",           default="$defaultModel")
    p.add_argument("--prompt",          required=True)
    p.add_argument("--output",          required=True)
    p.add_argument("--width",           type=int,   default=512)
    p.add_argument("--height",          type=int,   default=512)
    p.add_argument("--steps",           type=int,   default=20)
    p.add_argument("--guidance",        type=float, default=7.5)
    p.add_argument("--negative_prompt", default="")
    return p.parse_args()


def main():
    args = parse_args()

    try:
        import torch_directml
        import torch
        from diffusers import StableDiffusionPipeline, DPMSolverMultistepScheduler
    except ImportError as e:
        print(f"ERROR: Missing dependency: {e}", file=sys.stderr)
        print("Run: pip install torch-directml diffusers transformers accelerate Pillow", file=sys.stderr)
        sys.exit(1)

    dml_device = torch_directml.device()
    print(f"[sd] Device: {dml_device}", file=sys.stderr)
    print(f"[sd] Model: {args.model}", file=sys.stderr)
    print(f"[sd] Prompt: {args.prompt[:80]}", file=sys.stderr)

    # Load pipeline — uses HuggingFace cache if model is an HF id
    pipe = StableDiffusionPipeline.from_pretrained(
        args.model,
        torch_dtype=torch.float16,
        safety_checker=None,
        requires_safety_checker=False,
    )
    pipe.scheduler = DPMSolverMultistepScheduler.from_config(pipe.scheduler.config)
    pipe = pipe.to(dml_device)

    print(f"[sd] Generating ({args.width}x{args.height}, {args.steps} steps)...", file=sys.stderr)

    result = pipe(
        prompt=args.prompt,
        negative_prompt=args.negative_prompt or None,
        width=args.width,
        height=args.height,
        num_inference_steps=args.steps,
        guidance_scale=args.guidance,
        num_images_per_prompt=1,
    )

    image = result.images[0]

    # Ensure output directory exists
    os.makedirs(os.path.dirname(os.path.abspath(args.output)), exist_ok=True)
    image.save(args.output, "PNG")
    print(f"[sd] Saved: {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
"@

$scriptPath = "$ScriptsDir\sd_generate.py"
Set-Content -Path $scriptPath -Value $scriptContent -Encoding UTF8
Write-Host "[sd] Script written: $scriptPath" -ForegroundColor Green

# ── Optional model download ───────────────────────────────────────────────────

if ($DownloadModel) {
    Write-Host "`n[sd] Downloading model $ModelId (this will be several GB — Ctrl+C to cancel)..." -ForegroundColor Yellow
    & $venvPy -c @"
from diffusers import StableDiffusionPipeline
import torch
print('[sd] Caching model weights from HuggingFace...')
pipe = StableDiffusionPipeline.from_pretrained('$ModelId', torch_dtype=torch.float16, safety_checker=None)
print('[sd] Model cached successfully.')
"@
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Model download failed or was cancelled. Run with -DownloadModel again to retry."
    } else {
        Write-Host "[sd] Model $ModelId cached to HuggingFace cache directory." -ForegroundColor Green
    }
} else {
    Write-Host "`n[sd] No model downloaded (use -DownloadModel to fetch $ModelId, or -ModelDir to point at a local model)." -ForegroundColor Yellow
}

# ── Smoke test ────────────────────────────────────────────────────────────────

Write-Host "`n[sd] Verifying package imports..." -ForegroundColor Cyan
$importTest = & $venvPy -c "import torch_directml, diffusers, transformers; print('OK')" 2>&1
if ($importTest -match "OK") {
    Write-Host "[sd] All packages import successfully." -ForegroundColor Green
} else {
    Write-Warning "[sd] Import check failed: $importTest"
}

Write-Host @"

[sd] Installation complete.
  Venv   : $VenvDir
  Script : $scriptPath
  Model  : $defaultModel

The Bonsai image generation endpoint is now active (if a model is cached):
  POST http://127.0.0.1:<port>/api/v1/images/generate
  Body : {"prompt":"a sunset over mountains","steps":20,"output_path":"C:/tmp/out.png"}

To generate your first image:
  & '$venvPy' '$scriptPath' --model '$defaultModel' --prompt 'a sunset' --output C:\tmp\test.png
"@ -ForegroundColor Cyan
