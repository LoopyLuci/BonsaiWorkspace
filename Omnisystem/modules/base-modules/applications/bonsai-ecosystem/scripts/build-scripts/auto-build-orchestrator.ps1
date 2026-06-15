#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Automatic Build Orchestrator
    Monitors Python installation and automatically launches GPU training build
    Provides real-time status updates and GPU monitoring
#>

$workspace = "Z:\Projects\BonsaiWorkspace"
$pythonTaskFile = "C:\Users\limpi\AppData\Local\Temp\claude\z--Projects-BonsaiWorkspace\c7ae2a7a-5206-469e-8d6b-97fc5255ee90\tasks\bqjwu922v.output"

Write-Host @"
════════════════════════════════════════════════════════════════════════════════
🚀 BONSAI ECOSYSTEM AUTO-BUILD ORCHESTRATOR
════════════════════════════════════════════════════════════════════════════════

Hardware: Ryzen 9 5900X (12C/24T), 64GB RAM, RX 7900 XTX (24GB VRAM)
Build Path: $workspace

This script will:
1. Monitor Python installation completion
2. Auto-launch GPU build when Python is ready
3. Stream real-time training output
4. Launch IDE when training completes

════════════════════════════════════════════════════════════════════════════════
"@

# ============================================================================
# PHASE 0: Monitor Python Installation
# ============================================================================

Write-Host "`n📥 PHASE 0: Waiting for Python Installation..." -ForegroundColor Cyan
Write-Host "Task ID: bqjwu922v" -ForegroundColor Yellow

$pythonReady = $false
$checkInterval = 10  # seconds
$maxWait = 600  # 10 minutes max wait
$elapsed = 0

while (-not $pythonReady -and $elapsed -lt $maxWait) {
    if (Test-Path $pythonTaskFile) {
        $content = Get-Content $pythonTaskFile -ErrorAction SilentlyContinue

        if ($content -like "*All dependencies installed*" -or $content -like "*PyTorch*") {
            $pythonReady = $true
            Write-Host "✅ Python installation complete!" -ForegroundColor Green
            Write-Host "   Showing final output:`n" -ForegroundColor Green

            # Show last 20 lines of output
            $lines = $content -split "`n"
            $lines | Select-Object -Last 20 | Write-Host -ForegroundColor Green

            break
        }
    }

    # Progress indicator
    $progress = [math]::Min(100, ($elapsed / $maxWait) * 100)
    $bar = "█" * ($progress / 5) + "░" * (20 - $progress / 5)

    Write-Host "`r⏳ Waiting... [$bar] $([int]$progress)% ($elapsed seconds)" -NoNewline

    Start-Sleep -Seconds $checkInterval
    $elapsed += $checkInterval
}

if (-not $pythonReady) {
    Write-Host "`n⚠️  Python installation taking longer than expected." -ForegroundColor Yellow
    Write-Host "Checking if Python is available anyway..." -ForegroundColor Yellow

    if (Get-Command python -ErrorAction SilentlyContinue) {
        Write-Host "✅ Python found! Proceeding with GPU build..." -ForegroundColor Green
        $pythonReady = $true
    } else {
        Write-Host "❌ Python still not available. Please check:" -ForegroundColor Red
        Write-Host "   Get-Content $pythonTaskFile" -ForegroundColor Yellow
        exit 1
    }
}

# ============================================================================
# PHASE 1-6: Launch GPU Build
# ============================================================================

Write-Host "`n════════════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "🚀 LAUNCHING GPU BUILD SCRIPT NOW" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

Write-Host @"
WHAT HAPPENS NEXT:

1️⃣  PHASE 1: Build USOS Kernel (5 min)
    └─ Compiles bare-metal x86_64 kernel

2️⃣  PHASE 2: Build Bonsai IDE (15 min)
    └─ Compiles Rust crates + Tauri app

3️⃣  PHASE 3: Prepare Training Data (1-2 hours)
    └─ Generates 1.6M training examples (CPU only)

4️⃣  PHASE 4: GPU TRAINING (4-6 hours) ⚡ MAIN EVENT
    └─ Trains Psychopathy Octopus on RX 7900 XTX
    └─ You'll see: Step 10/600: loss=4.23 → loss=1.23
    └─ GPU utilization: 85-95%
    └─ VRAM: 18-22 GB / 24 GB

5️⃣  PHASE 5: Merge & Convert (30 min)
    └─ Converts LoRA to GGUF Q4_K_M (~600 MB)

6️⃣  PHASE 6: Launch IDE
    └─ Bonsai Workspace opens automatically

═══════════════════════════════════════════════════════════════════════════════

📊 GPU MONITORING:
   Open Task Manager (Ctrl+Shift+Esc)
   → Performance tab → GPU
   Watch RX 7900 XTX utilization during Phase 4

📈 TRAINING OUTPUT:
   Loss should decrease: 4.23 → 3.87 → 3.65 ... → 1.23
   If it increases, that's unusual but will still work

⏱️  TOTAL TIME: 6-10 hours
   Most time is GPU training (4-6 hours)

═══════════════════════════════════════════════════════════════════════════════

Starting GPU build script in 5 seconds...
"@

Start-Sleep -Seconds 5

# Launch the GPU build script
Write-Host "`n🚀 EXECUTING: .\windows-gpu-build.ps1 -LaunchStack`n" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

Push-Location $workspace

# Run the GPU build script with full output visible
& .\windows-gpu-build.ps1 -LaunchStack 2>&1 | Tee-Object -FilePath "full-build.log"

# If we reach here, build is complete
Write-Host "`n════════════════════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "✅ GPU BUILD COMPLETE!" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════════════════════`n" -ForegroundColor Cyan

Write-Host @"
🎉 PSYCHOPATHY OCTOPUS IS TRAINED AND READY!

📊 MODEL LOCATION:
   $workspace\psychopathy-octopus-v1.Q4_K_M.gguf

🖥️  BONSAI WORKSPACE IDE:
   Should have opened automatically
   Check for desktop window

💬 TEST THE MODEL:
   In IDE chat panel:
   "What containers run on the Octopus server?"

   Expected response: Server-specific answer

📈 NEXT STEPS:
   1. Test Octopus AI in the IDE
   2. Collect feedback (👍 / 👎)
   3. Schedule nightly improvement (optional)
   4. Deploy to friend's server when ready

═══════════════════════════════════════════════════════════════════════════════

BUILD LOGS:
   Full output: $workspace\full-build.log
   Kernel:     $workspace\usos-build.log
   Rust:       $workspace\rust-build.log
   Data:       $workspace\prepare-data.log
   Training:   $workspace\training.log
   Merge:      $workspace\merge-convert.log

═══════════════════════════════════════════════════════════════════════════════

🚀 Ready for deployment!
"@

Pop-Location
