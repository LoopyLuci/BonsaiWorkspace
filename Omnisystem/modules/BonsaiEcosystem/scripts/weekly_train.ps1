<#
.SYNOPSIS
    BonsAI Weekly Full Training Cycle — Windows 10 + RX 7900 XTX

.DESCRIPTION
    Runs the complete 7-phase training pipeline in sequence:
      Phase 1 – Safety DPO           (no teacher, CPU, ~2 h)
      Phase 2 – Survival distillation (Qwen3-35B teacher, CPU student, ~3 h)
      Phase 3 – Tool-use DPO         (Qwen3-35B teacher, CPU, ~2 h)
      Phase 4 – Code distillation     (DeepSeek-R1-32B teacher, CPU student, ~3 h)
      Phase 5 – Chat distillation     (Qwen3-35B teacher, CPU, ~2 h)
      Phase 6 – Reasoning distill.    (DeepSeek-R1-14B teacher, CPU, ~2 h)
      Phase 7 – Final SFT merge       (CPU, ~3 h)
      Phase 8 – GGUF convert + hot-reload

    Training is CPU-only (AMD DirectML cannot do backward passes).
    GPU (RX 7900 XTX) is used ONLY for teacher inference via llama-server.

.PARAMETER SkipSafety
    Skip Phase 1.
.PARAMETER SkipSurvival
    Skip Phase 2.
.PARAMETER SkipToolUse
    Skip Phase 3.
.PARAMETER SkipCode
    Skip Phase 4.
.PARAMETER SkipChat
    Skip Phase 5.
.PARAMETER SkipReason
    Skip Phase 6.
.PARAMETER SkipFinal
    Skip Phase 7 (combine + SFT merge).
.PARAMETER SkipConvert
    Skip Phase 8 (GGUF conversion).
.PARAMETER TeacherGeneral
    Path to the general-purpose teacher GGUF.
    Default: D:/Models/general/Qwen3-35B-A22B-Q4_K_M.gguf
.PARAMETER TeacherCode
    Path to the code-specialist teacher GGUF.
    Default: D:/Models/coding/DeepSeek-R1-Distill-Qwen-32B-Q4_K_M.gguf
.PARAMETER TeacherReason
    Path to the reasoning-specialist teacher GGUF.
    Default: D:/Models/reasoning/DeepSeek-R1-Distill-Qwen-14B-Q4_K_M.gguf
.PARAMETER LlamaCppDir
    Path to llama.cpp checkout (needs convert_hf_to_gguf.py).
    Default: C:/tools/llama.cpp
.PARAMETER MaxPhases
    Stop after this many phases (0 = run all). Useful for testing.

.EXAMPLE
    # Full weekly run
    .\scripts\weekly_train.ps1

    # Skip safety (already done), start from survival
    .\scripts\weekly_train.ps1 -SkipSafety

    # Only run safety + tool-use, skip everything else
    .\scripts\weekly_train.ps1 -SkipSurvival -SkipCode -SkipChat -SkipReason -SkipFinal -SkipConvert
#>

param(
    [switch] $SkipSafety,
    [switch] $SkipSurvival,
    [switch] $SkipToolUse,
    [switch] $SkipCode,
    [switch] $SkipChat,
    [switch] $SkipReason,
    [switch] $SkipFinal,
    [switch] $SkipConvert,
    [string] $TeacherGeneral = "D:/Models/general/Qwen3-35B-A22B-Q4_K_M.gguf",
    [string] $TeacherCode    = "D:/Models/coding/DeepSeek-R1-Distill-Qwen-32B-Q4_K_M.gguf",
    [string] $TeacherReason  = "D:/Models/reasoning/DeepSeek-R1-Distill-Qwen-14B-Q4_K_M.gguf",
    [string] $LlamaCppDir    = "C:/tools/llama.cpp",
    [int]    $MaxPhases      = 0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Offline enforcement ──────────────────────────────────────────────────────
$env:TRANSFORMERS_OFFLINE       = "1"
$env:HF_HUB_OFFLINE             = "1"
$env:HF_DATASETS_OFFLINE        = "1"
$env:HF_HUB_DISABLE_TELEMETRY  = "1"

$WorkspaceRoot = Split-Path $PSScriptRoot -Parent
$TrainerDir    = "$WorkspaceRoot\bonsai-workspace\runtimes\bonsai-trainer"
$TrainingData  = "$WorkspaceRoot\training_data"
$ExportDir     = "$env:USERPROFILE\.bonsai\training_export"
$AdapterDir    = "$env:USERPROFILE\.bonsai\adapters"
$ModelsDir     = "$env:USERPROFILE\.bonsai\models"

# Create output dirs
@($ExportDir, $AdapterDir, $ModelsDir) | ForEach-Object {
    New-Item -ItemType Directory -Force -Path $_ | Out-Null
}

# ── Helpers ──────────────────────────────────────────────────────────────────

function Find-StudentHfDir {
    $result = python -c "
import os, glob
h = os.path.expanduser('~/.cache/huggingface/hub')
snaps = glob.glob(h + '/models--Qwen--**/snapshots/*/config.json', recursive=True)
if snaps:
    print(os.path.dirname(snaps[0]))
else:
    print('')
" 2>$null
    $result = $result.Trim()
    if (-not $result) {
        Write-Error "No local HuggingFace model snapshot found.`nDownload with: huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct"
    }
    return $result
}

$LlamaServerProcess = $null

function Start-Teacher {
    param([string]$ModelPath)
    if (-not (Test-Path $ModelPath)) {
        Write-Warning "[teacher] GGUF not found at $ModelPath — skipping teacher-dependent phase"
        return $false
    }
    Write-Host "[teacher] Starting llama-server: $ModelPath"
    $script:LlamaServerProcess = Start-Process llama-server `
        -ArgumentList "-m `"$ModelPath`" -ngl 99 --port 8080 --ctx-size 8192" `
        -PassThru -WindowStyle Minimized
    Write-Host "[teacher] Waiting 30 s for model to load…"
    Start-Sleep -Seconds 30
    # Health check
    try {
        $r = Invoke-WebRequest -Uri "http://127.0.0.1:8080/health" -TimeoutSec 5 -UseBasicParsing
        if ($r.StatusCode -eq 200) { Write-Host "[teacher] Ready."; return $true }
    } catch {}
    Write-Warning "[teacher] Health check failed after 30 s — teacher may still be loading. Continuing anyway."
    return $true
}

function Stop-Teacher {
    if ($script:LlamaServerProcess -and -not $script:LlamaServerProcess.HasExited) {
        Write-Host "[teacher] Stopping llama-server (PID $($script:LlamaServerProcess.Id))"
        Stop-Process -Id $script:LlamaServerProcess.Id -Force -ErrorAction SilentlyContinue
        $script:LlamaServerProcess = $null
        Start-Sleep -Seconds 3
    }
}

function Invoke-Phase {
    param([string]$Name, [scriptblock]$Body, [switch]$Skip)
    if ($Skip) { Write-Host "`n[skip] Phase: $Name"; return }
    Write-Host "`n$('─' * 70)"
    Write-Host "[phase] $Name  $(Get-Date -Format 'HH:mm:ss')"
    Write-Host "$('─' * 70)"
    & $Body
    Write-Host "[phase] $Name complete  $(Get-Date -Format 'HH:mm:ss')"

    $script:PhasesRun++
    if ($MaxPhases -gt 0 -and $script:PhasesRun -ge $MaxPhases) {
        Write-Host "`n[weekly_train] MaxPhases=$MaxPhases reached — stopping early."
        exit 0
    }
}

$script:PhasesRun = 0
$StudentHfDir = Find-StudentHfDir
Write-Host "[weekly_train] Student model: $StudentHfDir"
Write-Host "[weekly_train] Started: $(Get-Date)"

# ── Phase 1: Safety DPO ───────────────────────────────────────────────────────
Invoke-Phase -Name "Phase 1: Safety DPO" -Skip:$SkipSafety -Body {
    python "$WorkspaceRoot\scripts\generate_safety_data.py"
    python "$TrainerDir\dpo_train.py" `
        --base-model $StudentHfDir `
        --data       "$ExportDir\safety_dpo.jsonl" `
        --output     "$AdapterDir\bonsai-safety-v1" `
        --beta 0.15 --epochs 3 --device cpu
}

# ── Phase 2: Survival distillation ───────────────────────────────────────────
Invoke-Phase -Name "Phase 2: Survival Distillation" -Skip:$SkipSurvival -Body {
    $ok = Start-Teacher -ModelPath $TeacherGeneral
    python "$TrainerDir\distill.py" `
        --student-model $StudentHfDir `
        --teacher-api   http://127.0.0.1:8080 `
        --prompts       "$TrainingData\distill_prompts.txt" `
        --output        "$AdapterDir\bonsai-survival-v1" `
        --alpha 0.5 --device cpu
    Stop-Teacher
    # DPO on accumulated survival pairs (if they exist)
    if (Test-Path "$ExportDir\bonsai_dpo_latest.jsonl") {
        python "$TrainerDir\dpo_train.py" `
            --base-model $StudentHfDir `
            --data       "$ExportDir\bonsai_dpo_latest.jsonl" `
            --output     "$AdapterDir\bonsai-survival-dpo-v1" `
            --beta 0.1 --epochs 2 --device cpu
    }
}

# ── Phase 3: Tool-use ─────────────────────────────────────────────────────────
Invoke-Phase -Name "Phase 3: Tool-Use DPO" -Skip:$SkipToolUse -Body {
    Start-Teacher -ModelPath $TeacherGeneral | Out-Null
    python "$WorkspaceRoot\scripts\generate_tool_data.py" --teacher-url http://127.0.0.1:8080
    Stop-Teacher
    python "$TrainerDir\dpo_train.py" `
        --base-model $StudentHfDir `
        --data       "$ExportDir\tool_use_synthetic.jsonl" `
        --output     "$AdapterDir\bonsai-tooluse-v1" `
        --beta 0.08 --epochs 2 --device cpu
}

# ── Phase 4: Code distillation ────────────────────────────────────────────────
Invoke-Phase -Name "Phase 4: Code Distillation" -Skip:$SkipCode -Body {
    Start-Teacher -ModelPath $TeacherCode | Out-Null
    python "$TrainerDir\distill.py" `
        --student-model $StudentHfDir `
        --teacher-api   http://127.0.0.1:8080 `
        --prompts       "$TrainingData\code_prompts.txt" `
        --output        "$AdapterDir\bonsai-code-v1" `
        --alpha 0.6 --epochs 2 --device cpu
    Stop-Teacher
}

# ── Phase 5: General conversation ────────────────────────────────────────────
Invoke-Phase -Name "Phase 5: Chat Distillation" -Skip:$SkipChat -Body {
    Start-Teacher -ModelPath $TeacherGeneral | Out-Null
    python "$TrainerDir\distill.py" `
        --student-model $StudentHfDir `
        --teacher-api   http://127.0.0.1:8080 `
        --prompts       "$TrainingData\distill_prompts.txt" `
        --output        "$AdapterDir\bonsai-chat-v1" `
        --alpha 0.4 --epochs 2 --device cpu
    Stop-Teacher
}

# ── Phase 6: Reasoning ────────────────────────────────────────────────────────
Invoke-Phase -Name "Phase 6: Reasoning Distillation" -Skip:$SkipReason -Body {
    Start-Teacher -ModelPath $TeacherReason | Out-Null
    python "$TrainerDir\distill.py" `
        --student-model $StudentHfDir `
        --teacher-api   http://127.0.0.1:8080 `
        --prompts       "$TrainingData\reasoning_prompts.txt" `
        --output        "$AdapterDir\bonsai-reason-v1" `
        --alpha 0.7 --epochs 2 --device cpu
    Stop-Teacher
}

# ── Phase 7: Final multi-task SFT merge ──────────────────────────────────────
Invoke-Phase -Name "Phase 7: Final SFT Merge" -Skip:$SkipFinal -Body {
    # Combine all accumulated JSONL data
    python -c "
import json, pathlib, random
export = pathlib.Path(r'$ExportDir')
all_data = []
for f in export.glob('*.jsonl'):
    if 'combined' in f.name or 'final' in f.name:
        continue
    with open(f, encoding='utf-8') as fh:
        for line in fh:
            line = line.strip()
            if line:
                try:
                    all_data.append(json.loads(line))
                except: pass
random.shuffle(all_data)
out = export / 'bonsai_combined_final.jsonl'
with open(out, 'w', encoding='utf-8') as f:
    for r in all_data[:20000]:
        f.write(json.dumps(r) + '\n')
print(f'Combined {min(len(all_data),20000)} / {len(all_data)} examples -> {out}')
"
    python "$TrainerDir\finetune_sft.py" `
        --base-model $StudentHfDir `
        --data       "$ExportDir\bonsai_combined_final.jsonl" `
        --output     "$AdapterDir\bonsai-final-v1" `
        --device cpu --epochs 2
}

# ── Phase 8: GGUF convert + hot-reload ───────────────────────────────────────
Invoke-Phase -Name "Phase 8: GGUF Convert + Hot Reload" -Skip:$SkipConvert -Body {
    $adapter = "$AdapterDir\bonsai-final-v1"
    if (-not (Test-Path $adapter)) {
        # Fall back to the most recent adapter
        $adapter = Get-ChildItem "$AdapterDir\bonsai-*" -Directory |
            Sort-Object LastWriteTime -Descending |
            Select-Object -First 1 -ExpandProperty FullName
    }
    if (-not $adapter) { Write-Error "No adapter found. Run Phase 7 first." }

    Write-Host "[convert] Adapter: $adapter"
    python "$WorkspaceRoot\scripts\convert_to_gguf.py" `
        --base-model  $StudentHfDir `
        --adapter     $adapter `
        --output      "$ModelsDir\bonsai-latest.gguf" `
        --llama-cpp-dir $LlamaCppDir `
        --quant-type  q4_k_m
    Write-Host "[convert] Hot-reload watcher will pick up the new GGUF within 2 s."
}

Write-Host ""
Write-Host "$('═' * 70)"
Write-Host "[weekly_train] All phases complete  $(Get-Date)"
Write-Host "[weekly_train] Run 'just evaluate' to measure adapter quality."
Write-Host "$('═' * 70)"
