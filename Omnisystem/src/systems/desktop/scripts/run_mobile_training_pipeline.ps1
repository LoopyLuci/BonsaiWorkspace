#!/usr/bin/env pwsh
<#
.SYNOPSIS
BonsAI Mobile Model Training Pipeline Orchestration

.DESCRIPTION
Complete end-to-end pipeline for training a mobile-optimized AI model via knowledge distillation.

Phases:
  1. Download teacher model (if needed)
  2. Generate distillation prompts (8000 examples, domain-weighted)
  3. Start teacher sidecar server (llama-server)
  4. Train student model (knowledge distillation with LoRA)
  5. Quantize to GGUF (Q4_K_M)
  6. Generate metadata and model card
  7. Create .bkp package for distribution

Prerequisites:
  - Python 3.10+ with: torch, transformers, peft, yaml
  - llama.cpp with llama-server binary (in PATH or install via pip)
  - Model files in appropriate directories

.PARAMETER TeacherModel
Teacher model GGUF path. Defaults to $env:USERPROFILE\Models\Bonsai-8B-Q2_K.gguf

.PARAMETER StudentModel
Student model name (HuggingFace or local path). Defaults to TinyLlama-1.1B-Chat-v1.0

.PARAMETER Epochs
Number of training epochs. Defaults to 3.

.PARAMETER BatchSize
Training batch size. Defaults to 4.

.PARAMETER TeacherPort
Port for teacher llama-server. Defaults to 8080.

.PARAMETER SkipGenerate
Skip prompt generation if prompts already exist.

.PARAMETER SkipTrain
Skip training phase (useful for testing quantization).

.PARAMETER SkipQuantize
Skip quantization phase.

.PARAMETER Cleanup
Remove intermediate files after completion.

.EXAMPLE
.\scripts\run_mobile_training_pipeline.ps1 `
  -TeacherModel "$env:USERPROFILE\Models\Bonsai-8B-Q2_K.gguf" `
  -StudentModel "TinyLlama/TinyLlama-1.1B-Chat-v1.0" `
  -Epochs 3 `
  -BatchSize 4 `
  -Cleanup
#>

param(
    [string]$TeacherModel = "$env:USERPROFILE\Models\Bonsai-8B-Q2_K.gguf",
    [string]$StudentModel = "TinyLlama/TinyLlama-1.1B-Chat-v1.0",
    [string]$ConfigPath = "config/mobile_training_config.yaml",
    [int]$Epochs = 3,
    [int]$BatchSize = 4,
    [int]$TeacherPort = 8080,
    [switch]$SkipGenerate,
    [switch]$SkipTrain,
    [switch]$SkipQuantize,
    [switch]$Cleanup,
    [string]$OutputDir = "$PWD\training_output\bonsai-mobile-v1",
    [string]$Device = "cpu"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# ─── Colors ──────────────────────────────────────────────────────────────

$Colors = @{
    Success = "Green"
    Warning = "Yellow"
    Error   = "Red"
    Info    = "Cyan"
    Step    = "Magenta"
}

function Write-Log {
    param([string]$Message, [string]$Level = "Info")
    $timestamp = Get-Date -Format "HH:mm:ss"
    $color = $Colors[$Level]
    Write-Host "[$timestamp] $Message" -ForegroundColor $color
}

function Write-Header {
    param([string]$Message)
    Write-Host ""
    Write-Host ("=" * 80) -ForegroundColor Cyan
    Write-Host $Message -ForegroundColor Cyan
    Write-Host ("=" * 80) -ForegroundColor Cyan
}

function Test-Command {
    param([string]$CommandName)
    try {
        $null = Get-Command $CommandName -ErrorAction Stop
        return $true
    } catch {
        return $false
    }
}

function Wait-ForService {
    param(
        [string]$Url,
        [int]$MaxAttempts = 30,
        [int]$DelaySeconds = 1
    )

    $attempt = 0
    while ($attempt -lt $MaxAttempts) {
        try {
            $response = Invoke-WebRequest -Uri $Url -TimeoutSec 2 -ErrorAction SilentlyContinue
            if ($response.StatusCode -eq 200) {
                Write-Log "Service is ready" -Level "Success"
                return $true
            }
        } catch {
            # Service not ready yet
        }

        $attempt++
        if ($attempt -lt $MaxAttempts) {
            Start-Sleep -Seconds $DelaySeconds
        }
    }

    Write-Log "Service failed to start after $($MaxAttempts * $DelaySeconds)s" -Level "Error"
    return $false
}

# ─── Main Script ──────────────────────────────────────────────────────────

Write-Header "BonsAI Mobile Model Training Pipeline"
Write-Log "Teacher: $TeacherModel"
Write-Log "Student: $StudentModel"
Write-Log "Epochs: $Epochs, Batch Size: $BatchSize, Device: $Device"
Write-Log "Output: $OutputDir"

# ─── Phase 1: Verify prerequisites ─────────────────────────────────────────

Write-Log ""
Write-Log "Phase 1: Verify prerequisites" -Level "Step"

if (-not (Test-Command "python")) {
    Write-Log "Python not found in PATH" -Level "Error"
    exit 1
}

$pythonVersion = python --version 2>&1
Write-Log "Python: $pythonVersion" -Level "Success"

# ─── Phase 2: Verify/download teacher model ────────────────────────────────

Write-Log ""
Write-Log "Phase 2: Verify teacher model" -Level "Step"

if (-not (Test-Path $TeacherModel)) {
    Write-Log "Teacher model not found: $TeacherModel" -Level "Warning"
    Write-Log "Attempting download from HuggingFace..." -Level "Info"

    $modelsDir = Split-Path $TeacherModel
    if (-not (Test-Path $modelsDir)) {
        New-Item -ItemType Directory -Path $modelsDir -Force | Out-Null
        Write-Log "Created directory: $modelsDir" -Level "Success"
    }

    Write-Log "Downloading Bonsai-8B-Q2_K.gguf (3.5 GB)..." -Level "Info"
    Write-Log "This may take 10-20 minutes on a typical internet connection" -Level "Warning"

    $url = "https://huggingface.co/lilyanatia/Bonsai-8B-requantized/resolve/main/Bonsai-8B-Q2_K.gguf?download=true"

    try {
        curl.exe -L -o $TeacherModel $url
        if (Test-Path $TeacherModel) {
            $sizeGB = (Get-Item $TeacherModel).Length / 1GB
            Write-Log "Downloaded: $($sizeGB.ToString('F2')) GB" -Level "Success"
        } else {
            throw "Download verification failed"
        }
    } catch {
        Write-Log "Download failed: $_" -Level "Error"
        exit 1
    }
} else {
    $sizeGB = (Get-Item $TeacherModel).Length / 1GB
    Write-Log "Teacher model ready: $($sizeGB.ToString('F2')) GB" -Level "Success"
}

# ─── Phase 3: Generate prompts ──────────────────────────────────────────────

if (-not $SkipGenerate) {
    Write-Log ""
    Write-Log "Phase 3: Generate distillation prompts" -Level "Step"

    $promptFile = "$PWD\training_data\mobile_distill_prompts.jsonl"

    if ((Test-Path $promptFile)) {
        Write-Log "Prompts already exist, skipping generation" -Level "Warning"
    } else {
        try {
            Write-Log "Generating 8000 domain-weighted prompts..." -Level "Info"
            python scripts/generate_mobile_prompts.py
            Write-Log "Generated prompts successfully" -Level "Success"
        } catch {
            Write-Log "Failed to generate prompts: $_" -Level "Error"
            exit 1
        }
    }
} else {
    Write-Log ""
    Write-Log "Phase 3: Skipped (--SkipGenerate)" -Level "Warning"
}

# ─── Phase 4: Start teacher server ─────────────────────────────────────────

Write-Log ""
Write-Log "Phase 4: Start teacher model server" -Level "Step"

if (-not (Test-Command "llama-server")) {
    Write-Log "llama-server not found in PATH" -Level "Error"
    Write-Log "Install via: pip install llama-cpp-python[server]" -Level "Info"
    exit 1
}

Write-Log "Starting llama-server on port $TeacherPort..." -Level "Info"

$teacherProcess = Start-Process `
    -FilePath "llama-server" `
    -ArgumentList "-m `"$TeacherModel`" -ngl 99 --port $TeacherPort" `
    -PassThru `
    -NoNewWindow `
    -RedirectStandardOutput "$PWD\teacher_server.log" `
    -RedirectStandardError "$PWD\teacher_server_err.log"

Write-Log "Teacher process started (PID: $($teacherProcess.Id))" -Level "Success"

if (-not (Wait-ForService "http://127.0.0.1:$TeacherPort/v1/models" -MaxAttempts 60)) {
    Write-Log "Teacher server startup failed" -Level "Error"
    Stop-Process -Id $teacherProcess.Id -Force -ErrorAction SilentlyContinue
    exit 1
}

# ─── Phase 5: Train model ──────────────────────────────────────────────────

if (-not $SkipTrain) {
    Write-Log ""
    Write-Log "Phase 5: Train BonsAI Mobile model" -Level "Step"

    try {
        Write-Log "Starting training loop ($Epochs epochs)..." -Level "Info"
        python scripts/train_bonsai_mobile.py `
            --student-model $StudentModel `
            --teacher-api "http://127.0.0.1:$TeacherPort" `
            --prompts "training_data/mobile_distill_prompts.jsonl" `
            --output $OutputDir `
            --batch-size $BatchSize `
            --epochs $Epochs `
            --temperature 4.0 `
            --alpha 0.5 `
            --device $Device

        Write-Log "Training complete" -Level "Success"
    } catch {
        Write-Log "Training failed: $_" -Level "Error"
        Stop-Process -Id $teacherProcess.Id -Force -ErrorAction SilentlyContinue
        exit 1
    }
} else {
    Write-Log ""
    Write-Log "Phase 5: Skipped (--SkipTrain)" -Level "Warning"
}

# ─── Phase 6: Quantize model ───────────────────────────────────────────────

if (-not $SkipQuantize) {
    Write-Log ""
    Write-Log "Phase 6: Quantize model to GGUF" -Level "Step"

    try {
        Write-Log "Converting to GGUF with Q4_K_M quantization..." -Level "Info"
        python scripts/quantize_bonsai_mobile.py `
            --model-dir "$OutputDir/final_model" `
            --output-dir "$OutputDir/gguf" `
            --quantization Q4_K_M

        Write-Log "Quantization complete" -Level "Success"
    } catch {
        Write-Log "Quantization warning: $_" -Level "Warning"
    }
} else {
    Write-Log ""
    Write-Log "Phase 6: Skipped (--SkipQuantize)" -Level "Warning"
}

# ─── Cleanup ────────────────────────────────────────────────────────────────

Write-Log ""
Write-Log "Cleanup" -Level "Step"

Write-Log "Stopping teacher server (PID: $($teacherProcess.Id))..." -Level "Info"
try {
    Stop-Process -Id $teacherProcess.Id -Force
    Start-Sleep -Milliseconds 500
    Write-Log "Teacher server stopped" -Level "Success"
} catch {
    Write-Log "Could not stop teacher server: $_" -Level "Warning"
}

if ($Cleanup) {
    Write-Log "Removing intermediate files..." -Level "Info"

    @(
        "$PWD\teacher_server.log",
        "$PWD\teacher_server_err.log",
        "$OutputDir\checkpoint_epoch_*",
        "$OutputDir\fused_model"
    ) | ForEach-Object {
        if (Test-Path $_) {
            Remove-Item $_ -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    Write-Log "Cleanup complete" -Level "Success"
}

# ─── Final Summary ──────────────────────────────────────────────────────────

Write-Header "Pipeline Complete"
Write-Log ""
Write-Log "Output artifacts:" -Level "Info"
Write-Log "  • Final model: $OutputDir/final_model" -Level "Info"
Write-Log "  • GGUF: $OutputDir/gguf/bonsai-mobile-q4_k_m.gguf" -Level "Info"
Write-Log "  • Metadata: $OutputDir/gguf/bonsai-mobile-q4_k_m.metadata.json" -Level "Info"
Write-Log "  • Model card: $OutputDir/gguf/README.md" -Level "Info"
Write-Log ""
Write-Log "Next steps:" -Level "Info"
Write-Log "  1. Review model card: code $OutputDir/gguf/README.md" -Level "Info"
Write-Log "  2. Test inference with llama-cpp-python" -Level "Info"
Write-Log "  3. Deploy to mobile using llama-cpp iOS/Android SDKs" -Level "Info"
Write-Log ""

# ── Phase 1: Export Training Data ─────────────────────────────────────────

if (-not $SkipDataExport) {
    Write-Header "Phase 1: Export Training Data"

    Write-Step "Exporting combined training data..."

    $ExportCmd = @(
        "python scripts/export_mobile_training_data.py",
        "--output $DataExportPath",
        "--max-examples 100000",
        "--min-quality 0.70",
        "--domain-weights `"code:0.4,system_repair:0.2,tool_use:0.2,chat:0.1,qa:0.05`""
    ) -join " "

    Write-Host "  Command: $ExportCmd"

    if (-not $DryRun) {
        & python scripts/export_mobile_training_data.py `
            --output "$DataExportPath" `
            --max-examples 100000 `
            --min-quality 0.70 `
            --domain-weights "code:0.4,system_repair:0.2,tool_use:0.2,chat:0.1,qa:0.05"

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Data export failed"
            exit 1
        }

        if (Test-Path $DataExportPath) {
            $FileSize = (Get-Item $DataExportPath).Length / 1MB
            Write-Host "  Output: $DataExportPath ($FileSize MB)"
        }
    }
}

# ── Phase 2: Start Teacher Sidecar (llama-server) ──────────────────────────

$TeacherPID = $null

if (-not $SkipTeacher) {
    Write-Header "Phase 2: Start Teacher Sidecar (llama-server)"

    # Check if server already running
    $Existing = Get-Process llama-server -ErrorAction SilentlyContinue
    if ($Existing) {
        Write-Warning "llama-server already running on port $LlamaServerPort"
        Write-Step "Using existing teacher sidecar"
    } else {
        Write-Step "Starting llama-server with teacher model..."

        # Determine binary
        $ServerBin = if (Test-Path $LlamaServerBin) { $LlamaServerBin } else { "llama-server" }

        # Build llama-server command
        $TeacherCmd = @(
            $ServerBin,
            "-m", "`"$TeacherGGUF`"",
            "-ngl 99",
            "--port $LlamaServerPort",
            "--no-mmap",
            "-c 2048"
        ) -join " "

        Write-Host "  Command: $TeacherCmd"

        if (-not $DryRun) {
            # Start in background
            $Process = Start-Process -FilePath $ServerBin -ArgumentList `
                "-m `"$TeacherGGUF`" -ngl 99 --port $LlamaServerPort --no-mmap -c 2048" `
                -PassThru -RedirectStandardOutput "llama-server.log" -RedirectStandardError "llama-server.err"

            $TeacherPID = $Process.Id
            Write-Host "  Started (PID: $TeacherPID)"
            Write-Host "  Logs: llama-server.log"

            # Wait for server to be ready
            Write-Step "Waiting for teacher server to be ready..."
            $MaxRetries = 60
            $Retries = 0
            while ($Retries -lt $MaxRetries) {
                try {
                    $Response = curl -s "http://127.0.0.1:$LlamaServerPort/health" -ErrorAction SilentlyContinue
                    if ($Response) {
                        Write-Host "  Teacher ready on http://127.0.0.1:$LlamaServerPort"
                        break
                    }
                } catch { }
                $Retries++
                Start-Sleep -Seconds 1
            }

            if ($Retries -ge $MaxRetries) {
                Write-Warning "Teacher server may not be ready (timeout after 60s)"
            }
        }
    }
}

# ── Phase 3: Training ─────────────────────────────────────────────────────

if (-not $SkipTraining) {
    Write-Header "Phase 3: Knowledge Distillation Training"

    Write-Step "Running training loop..."

    $TrainCmd = @(
        "python scripts/train_bonsai_mobile.py",
        "--student-model $StudentModel",
        "--teacher-api http://127.0.0.1:$LlamaServerPort",
        "--teacher-gguf `"$TeacherGGUF`"",
        "--config `"$ConfigPath`"",
        "--training-data `"$DataExportPath`"",
        "--output-dir `"$OutputDir`"",
        "--epochs $Epochs",
        "--learning-rate $LearningRate",
        "--device $Device",
        "--seed 42"
    ) -join " "

    Write-Host "  Command: $TrainCmd"

    if (-not $DryRun) {
        & python scripts/train_bonsai_mobile.py `
            --student-model $StudentModel `
            --teacher-api "http://127.0.0.1:$LlamaServerPort" `
            --teacher-gguf "$TeacherGGUF" `
            --config "$ConfigPath" `
            --training-data "$DataExportPath" `
            --output-dir "$OutputDir" `
            --epochs $Epochs `
            --learning-rate $LearningRate `
            --device $Device `
            --seed 42

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Training failed"
            exit 1
        }

        Write-Host "  Training logs: $OutputDir"
    }
}

# ── Phase 4: Quantization ─────────────────────────────────────────────────

if (-not $SkipQuantization) {
    Write-Header "Phase 4: Quantization to GGUF"

    $FinalModel = Join-Path $OutputDir "final_model"
    if (-not (Test-Path $FinalModel)) {
        Write-Error "Final model not found: $FinalModel"
        exit 1
    }

    Write-Step "Quantizing to GGUF (Q4_K_M)..."

    $QuantCmd = @(
        "python scripts/quantize_bonsai_mobile.py",
        "--final-model `"$FinalModel`"",
        "--output-dir `"$OutputDir/gguf`"",
        "--model-name bonsai-mobile-v1",
        "--quantization Q4_K_M",
        "--validate",
        "--create-bkp"
    ) -join " "

    Write-Host "  Command: $QuantCmd"

    if (-not $DryRun) {
        & python scripts/quantize_bonsai_mobile.py `
            --final-model "$FinalModel" `
            --output-dir "$OutputDir/gguf" `
            --model-name "bonsai-mobile-v1" `
            --quantization Q4_K_M `
            --validate `
            --create-bkp

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Quantization failed"
            exit 1
        }

        $GGUFPath = Join-Path "$OutputDir/gguf" "bonsai-mobile-v1.gguf"
        if (Test-Path $GGUFPath) {
            $GGUFSize = (Get-Item $GGUFPath).Length / 1MB
            Write-Host "  GGUF: $GGUFPath ($([math]::Round($GGUFSize, 1)) MB)"
        }
    }
}

# ── Phase 5: Benchmarking ─────────────────────────────────────────────────

if (-not $SkipBenchmark) {
    Write-Header "Phase 5: Performance Benchmarking"

    $GGUFPath = Join-Path "$OutputDir/gguf" "bonsai-mobile-v1.gguf"
    if (-not (Test-Path $GGUFPath)) {
        Write-Warning "GGUF not found, skipping benchmark"
    } else {
        Write-Step "Benchmarking model..."

        $BenchCmd = @(
            "python scripts/benchmark_bonsai_mobile.py",
            "--model `"$GGUFPath`"",
            "--output-dir `"$OutputDir/benchmark`"",
            "--num-prompts 50",
            "--device $Device"
        ) -join " "

        Write-Host "  Command: $BenchCmd"

        if (-not $DryRun) {
            & python scripts/benchmark_bonsai_mobile.py `
                --model "$GGUFPath" `
                --output-dir "$OutputDir/benchmark" `
                --num-prompts 50 `
                --device $Device

            Write-Host "  Results: $OutputDir/benchmark"
        }
    }
}

# ── Phase 6: Model Registration ───────────────────────────────────────────

if (-not $SkipRegister) {
    Write-Header "Phase 6: Model Registry"

    $GGUFPath = Join-Path "$OutputDir/gguf" "bonsai-mobile-v1.gguf"
    if (-not (Test-Path $GGUFPath)) {
        Write-Warning "GGUF not found, skipping registration"
    } else {
        Write-Step "Registering model..."

        $ReleaseDir = Join-Path $env:USERPROFILE ".bonsai\models\releases"
        New-Item -ItemType Directory -Path $ReleaseDir -Force | Out-Null

        # Copy GGUF to releases
        Copy-Item $GGUFPath -Destination $ReleaseDir -Force
        Write-Host "  Registered: $ReleaseDir\bonsai-mobile-v1.gguf"

        # Create registry entry
        $RegistryEntry = @{
            name = "bonsai-mobile-v1"
            role = "student_mobile"
            gguf = "$ReleaseDir\bonsai-mobile-v1.gguf"
            vram_gb = 1
            domains = @("coding", "system_repair", "tool_use", "chat", "qa")
            context_len = 2048
            quantisation = "Q4_K_M"
            created = Get-Date -Format "o"
        }

        $RegistryPath = "$ReleaseDir\bonsai-mobile-v1.metadata.json"
        $RegistryEntry | ConvertTo-Json | Out-File $RegistryPath -Encoding UTF8
        Write-Host "  Metadata: $RegistryPath"
    }
}

# ── Cleanup ──────────────────────────────────────────────────────────────

Write-Header "Pipeline Complete"

if ($TeacherPID) {
    Write-Step "Stopping teacher server (PID: $TeacherPID)..."
    Stop-Process -Id $TeacherPID -ErrorAction SilentlyContinue
    Write-Host "  Stopped"
}

Write-Step "Summary:"
Write-Host "  Output directory: $OutputDir"
Write-Host "  Final model: $OutputDir/final_model"
Write-Host "  GGUF: $OutputDir/gguf/bonsai-mobile-v1.gguf"
Write-Host "  Benchmark results: $OutputDir/benchmark"
Write-Host "  Training logs: $OutputDir/train_*.log"

Write-Host "`n" -NoNewline
Write-Host ("=" * 80) -ForegroundColor Cyan
Write-Host "BonsAI Mobile Training Pipeline Complete!" -ForegroundColor Green
Write-Host ("=" * 80) -ForegroundColor Cyan
Write-Host ""

exit 0
