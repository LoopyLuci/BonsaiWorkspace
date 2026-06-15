<#
.SYNOPSIS
    Full BonsAI training cycle for Windows 10, Ryzen 9 5900X, RX 7900 XTX.

.DESCRIPTION
    Runs the complete pipeline: data export → DPO → distillation → evaluation
    → hot model reload.  All steps are 100% offline — no HuggingFace downloads.

    Training happens on CPU (PyTorch/DirectML backward pass is unsupported on AMD
    Windows; ROCm requires Linux).  The RX 7900 XTX is used for llama-server
    inference only (teacher distillation via API).

.PARAMETER SkipExport
    Skip the data export step (use existing ~/.bonsai/training_export/).

.PARAMETER SkipDpo
    Skip the DPO training step.

.PARAMETER SkipDistill
    Skip knowledge distillation (requires Qwen3-35B in D:\Models\general\).

.PARAMETER SkipEval
    Skip evaluation (requires the Bonsai Workspace app to be running).

.PARAMETER SkipHotReload
    Skip the final hot-reload step.

.PARAMETER TeacherModel
    Path to the teacher GGUF for distillation.
    Default: D:\Models\general\Qwen3-35B-A22B-Q4_K_M.gguf

.PARAMETER StudentHfDir
    Path to the student HF model directory (needed for gradient training).
    Default: auto-detected from HuggingFace cache.

.EXAMPLE
    .\scripts\train-windows.ps1
    .\scripts\train-windows.ps1 -SkipExport -SkipDistill
#>
[CmdletBinding()]
param(
    [switch]$SkipExport,
    [switch]$SkipDpo,
    [switch]$SkipDistill,
    [switch]$SkipEval,
    [switch]$SkipHotReload,
    [string]$TeacherModel  = "D:\Models\general\Qwen3-35B-A22B-Q4_K_M.gguf",
    [string]$StudentHfDir  = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$WorkspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$TrainerDir    = Join-Path $WorkspaceRoot "bonsai-workspace\runtimes\bonsai-trainer"
$ExportDir     = Join-Path $env:USERPROFILE ".bonsai\training_export"
$AdaptersDir   = Join-Path $env:USERPROFILE ".bonsai\adapters"
$ModelsDir     = Join-Path $env:USERPROFILE ".bonsai\models"
$Timestamp     = Get-Date -Format "yyyyMMdd_HHmmss"

# ── Enforce offline mode ────────────────────────────────────────────────────────
$env:TRANSFORMERS_OFFLINE       = "1"
$env:HF_HUB_OFFLINE             = "1"
$env:HF_DATASETS_OFFLINE        = "1"
$env:HF_HUB_DISABLE_TELEMETRY  = "1"

function Write-Step { param([string]$Msg) Write-Host "`n==> $Msg" -ForegroundColor Cyan }
function Write-Ok   { param([string]$Msg) Write-Host "    $Msg"   -ForegroundColor Green }
function Write-Warn { param([string]$Msg) Write-Host "    WARN: $Msg" -ForegroundColor Yellow }

# ── Auto-detect student HF dir ─────────────────────────────────────────────────
if (-not $StudentHfDir) {
    $HfCache = Join-Path $env:USERPROFILE ".cache\huggingface\hub"
    $Configs = Get-ChildItem -Path $HfCache -Recurse -Filter "config.json" -ErrorAction SilentlyContinue |
               Where-Object { $_.FullName -match "models--Qwen" } |
               Sort-Object LastWriteTime -Descending
    if ($Configs) {
        $StudentHfDir = Split-Path $Configs[0].FullName -Parent
        Write-Ok "Auto-detected student model: $StudentHfDir"
    } else {
        Write-Warn "No local HF model found in $HfCache"
        Write-Warn "Skipping gradient-based training (DPO/distill require a local HF snapshot)."
        Write-Warn "Download once: huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct"
        $SkipDpo     = $true
        $SkipDistill = $true
    }
}

New-Item -ItemType Directory -Force -Path $AdaptersDir | Out-Null
New-Item -ItemType Directory -Force -Path $ModelsDir   | Out-Null

# ── Step 1: Export training data ───────────────────────────────────────────────
if (-not $SkipExport) {
    Write-Step "Step 1 — Exporting training data"
    & powershell -NoProfile -ExecutionPolicy Bypass `
        -File (Join-Path $WorkspaceRoot "scripts\export_training_data.ps1")
    Write-Ok "Export complete → $ExportDir"
} else {
    Write-Warn "Skipping export (--SkipExport)"
}

# ── Step 2: DPO preference optimisation ───────────────────────────────────────
if (-not $SkipDpo) {
    Write-Step "Step 2 — DPO preference optimisation"
    $DpoData  = Join-Path $ExportDir "bonsai_dpo_latest.jsonl"
    $DpoOut   = Join-Path $AdaptersDir "bonsai-dpo-$Timestamp"

    if (-not (Test-Path $DpoData)) {
        Write-Warn "No DPO data at $DpoData — skipping DPO."
    } elseif (-not (Test-Path $StudentHfDir)) {
        Write-Warn "Student HF dir not found — skipping DPO."
    } else {
        python3 (Join-Path $TrainerDir "dpo_train.py") `
            --base-model $StudentHfDir `
            --data $DpoData `
            --output $DpoOut `
            --device cpu
        Write-Ok "DPO adapter → $DpoOut"
    }
} else {
    Write-Warn "Skipping DPO (--SkipDpo)"
}

# ── Step 3: Knowledge distillation ────────────────────────────────────────────
if (-not $SkipDistill) {
    Write-Step "Step 3 — Knowledge distillation (teacher → student)"
    $DistillOut = Join-Path $AdaptersDir "bonsai-distilled-$Timestamp"
    $Prompts    = Join-Path $ExportDir "distill_prompts.txt"

    if (-not (Test-Path $TeacherModel)) {
        Write-Warn "Teacher model not found: $TeacherModel"
        Write-Warn "Skipping distillation. Set -TeacherModel to your largest GGUF."
    } elseif (-not (Test-Path $StudentHfDir)) {
        Write-Warn "Student HF dir not found — skipping distillation."
    } else {
        # Start the teacher as a llama-server sidecar (GPU inference on 7900 XTX)
        Write-Ok "Starting teacher sidecar: $TeacherModel"
        $LlamaServer = Get-Command "llama-server" -ErrorAction SilentlyContinue
        if (-not $LlamaServer) {
            $LlamaServer = Get-Command "llama-server.exe" -ErrorAction SilentlyContinue
        }
        if (-not $LlamaServer) {
            Write-Warn "llama-server not found in PATH. Skipping distillation."
            Write-Warn "Install llama.cpp: https://github.com/ggerganov/llama.cpp/releases"
        } else {
            $TeacherProc = Start-Process -FilePath $LlamaServer.Source `
                -ArgumentList "-m `"$TeacherModel`" -ngl 99 --port 8080" `
                -PassThru -WindowStyle Minimized
            Write-Ok "Teacher sidecar PID=$($TeacherProc.Id) — waiting for readiness..."
            Start-Sleep -Seconds 15   # give the server time to load

            try {
                python3 (Join-Path $TrainerDir "distill.py") `
                    --student-model $StudentHfDir `
                    --teacher-api   "http://127.0.0.1:8080" `
                    --prompts       $Prompts `
                    --output        $DistillOut `
                    --alpha         0.5
                Write-Ok "Distillation adapter → $DistillOut"
            } finally {
                Stop-Process -Id $TeacherProc.Id -Force -ErrorAction SilentlyContinue
                Write-Ok "Teacher sidecar stopped."
            }
        }
    }
} else {
    Write-Warn "Skipping distillation (--SkipDistill)"
}

# ── Step 4: Deploy best adapter ───────────────────────────────────────────────
Write-Step "Step 4 — Deploying latest adapter"
$LatestAdapter = Get-ChildItem -Path $AdaptersDir -Directory |
                 Where-Object { $_.Name -match "^bonsai-(dpo|distilled|sft)" } |
                 Sort-Object LastWriteTime -Descending |
                 Select-Object -First 1

if ($LatestAdapter) {
    Write-Ok "Latest adapter: $($LatestAdapter.FullName)"
    # Copy the fused model to bonsai-latest.gguf if a fused GGUF exists
    $FusedGguf = Get-ChildItem -Path $LatestAdapter.FullName -Filter "*.gguf" -ErrorAction SilentlyContinue |
                 Select-Object -First 1
    if ($FusedGguf) {
        $TargetPath = Join-Path $ModelsDir "bonsai-latest.gguf"
        Copy-Item -Path $FusedGguf.FullName -Destination $TargetPath -Force
        Write-Ok "Deployed GGUF → $TargetPath"
        Write-Ok "The hot-reload watcher will detect the new file automatically."
    } else {
        Write-Warn "No fused GGUF found in adapter dir. Run llama.cpp convert_hf_to_gguf.py manually."
        Write-Warn "Then copy the output to $ModelsDir\bonsai-latest.gguf"
    }
} else {
    Write-Warn "No adapter found to deploy."
}

# ── Step 5: Evaluate ──────────────────────────────────────────────────────────
if (-not $SkipEval) {
    Write-Step "Step 5 — Evaluation"
    try {
        $EvalResult = Invoke-RestMethod -Uri "http://127.0.0.1:11369/api/training/evaluate" `
            -Method Post `
            -ContentType "application/json" `
            -Body '{"run_full": true}' `
            -TimeoutSec 300
        Write-Ok "Evaluation complete: $($EvalResult | ConvertTo-Json -Depth 3)"
    } catch {
        Write-Warn "Could not reach Bonsai Workspace at :11369 — launch the app and re-run evaluation:"
        Write-Warn "  just evaluate"
    }
} else {
    Write-Warn "Skipping evaluation (--SkipEval)"
}

# ── Step 6: Trigger hot reload via Tauri API ──────────────────────────────────
if (-not $SkipHotReload) {
    $TargetGguf = Join-Path $ModelsDir "bonsai-latest.gguf"
    if (Test-Path $TargetGguf) {
        Write-Step "Step 6 — Hot reload"
        try {
            $ReloadResult = Invoke-RestMethod -Uri "http://127.0.0.1:11369/api/model/hot-reload" `
                -Method Post `
                -ContentType "application/json" `
                -Body (ConvertTo-Json @{ model_path = $TargetGguf }) `
                -TimeoutSec 120
            Write-Ok "Hot reload triggered: $ReloadResult"
        } catch {
            Write-Warn "Hot reload API not reachable. The file watcher will pick up the change automatically."
            Write-Warn "Or restart Bonsai Workspace to load the new model."
        }
    }
}

Write-Host ""
Write-Host "Training cycle complete." -ForegroundColor Green
Write-Host "  Adapters: $AdaptersDir"
Write-Host "  Model:    $(Join-Path $ModelsDir 'bonsai-latest.gguf')"
Write-Host ""
Write-Host "If the app is running, the hot-reload watcher will swap the model"
Write-Host "automatically within a few seconds.  No restart required."
