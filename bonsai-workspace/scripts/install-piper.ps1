<#
.SYNOPSIS
    Install Piper TTS sidecar and a default English voice for Bonsai.

.DESCRIPTION
    Downloads the latest Piper release binary for Windows x86_64, extracts
    piper.exe to %LOCALAPPDATA%\com.bonsai.workspace\sidecars\, then
    downloads the en_US-lessac-medium voice to the voices directory.

    Everything is 100% offline after this script runs — no runtime downloads.

.PARAMETER Voice
    Voice model to download. Default: en_US-lessac-medium
    Browse all voices at: https://huggingface.co/rhasspy/piper-voices/tree/main

.PARAMETER Force
    Re-download even if piper.exe and the voice already exist.

.EXAMPLE
    .\install-piper.ps1
    .\install-piper.ps1 -Voice en_US-ryan-medium -Force
#>

param(
    [string]$Voice = "en_US-lessac-medium",
    [switch]$Force
)

Set-StrictMode -Version 3
$ErrorActionPreference = "Stop"

$BonsaiBase  = "$env:LOCALAPPDATA\com.bonsai.workspace"
$SidecarDir  = "$BonsaiBase\sidecars"
$VoicesDir   = "$BonsaiBase\voices"
$PiperExe    = "$SidecarDir\piper.exe"

# ── Piper binary ──────────────────────────────────────────────────────────────

if ((Test-Path $PiperExe) -and -not $Force) {
    Write-Host "[piper] piper.exe already installed at $PiperExe" -ForegroundColor Cyan
} else {
    Write-Host "[piper] Fetching latest release info from GitHub..." -ForegroundColor Cyan

    $releaseApi = "https://api.github.com/repos/rhasspy/piper/releases/latest"
    $headers    = @{ "User-Agent" = "bonsai-installer/1.0" }

    try {
        $release = Invoke-RestMethod $releaseApi -Headers $headers -TimeoutSec 30
    } catch {
        Write-Error "Failed to fetch Piper release info: $_"
        exit 1
    }

    # Find the Windows asset (amd64 / x86_64)
    $asset = $release.assets | Where-Object { $_.name -like "*windows*" -and $_.name -like "*.zip" } | Select-Object -First 1
    if (-not $asset) {
        Write-Error "Could not find a Windows zip asset in the latest Piper release. Assets: $($release.assets.name -join ', ')"
        exit 1
    }

    Write-Host "[piper] Downloading $($asset.name) ($([math]::Round($asset.size/1MB, 1)) MB)..."
    $zipPath = "$env:TEMP\piper-windows.zip"
    Invoke-WebRequest $asset.browser_download_url -OutFile $zipPath -TimeoutSec 300

    Write-Host "[piper] Extracting..."
    $extractDir = "$env:TEMP\piper-extract"
    if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
    Expand-Archive $zipPath $extractDir

    # piper.exe may be nested in a subdirectory
    $exeFound = Get-ChildItem $extractDir -Filter piper.exe -Recurse | Select-Object -First 1
    if (-not $exeFound) {
        Write-Error "piper.exe not found in extracted archive."
        exit 1
    }

    New-Item -ItemType Directory -Force $SidecarDir | Out-Null

    # Copy everything from the directory containing piper.exe (DLLs, .ort, pkgconfig)
    Get-ChildItem $exeFound.DirectoryName -File | ForEach-Object {
        Copy-Item $_.FullName $SidecarDir -Force
    }

    # Copy espeak-ng-data directory (required for phonemization)
    $espeak = Join-Path $exeFound.DirectoryName "espeak-ng-data"
    if (Test-Path $espeak) {
        $dest = Join-Path $SidecarDir "espeak-ng-data"
        if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
        Copy-Item $espeak $SidecarDir -Recurse -Force
        Write-Host "[piper] Copied espeak-ng-data to $dest"
    } else {
        Write-Warning "[piper] espeak-ng-data not found in archive — piper may fail to load voices."
    }

    Remove-Item $zipPath -Force -ErrorAction SilentlyContinue
    Remove-Item $extractDir -Recurse -Force -ErrorAction SilentlyContinue

    Write-Host "[piper] piper.exe installed to $PiperExe" -ForegroundColor Green
}

# ── Voice model ───────────────────────────────────────────────────────────────

New-Item -ItemType Directory -Force $VoicesDir | Out-Null

$onnxFile = "$VoicesDir\$Voice.onnx"
$jsonFile  = "$VoicesDir\$Voice.onnx.json"

if ((Test-Path $onnxFile) -and (Test-Path $jsonFile) -and -not $Force) {
    Write-Host "[piper] Voice $Voice already installed." -ForegroundColor Cyan
} else {
    # Parse voice name: en_US-lessac-medium → lang=en/en_US, name=lessac, quality=medium
    if ($Voice -notmatch "^([a-z]{2}_[A-Z]{2})-([^-]+)-(.+)$") {
        Write-Error "Voice name must be in format LANG_REGION-NAME-QUALITY (e.g. en_US-lessac-medium)"
        exit 1
    }
    $langRegion = $Matches[1]            # en_US
    $lang       = $langRegion.Split("_")[0]  # en
    $voiceName  = $Matches[2]            # lessac
    $quality    = $Matches[3]            # medium

    $hfBase = "https://huggingface.co/rhasspy/piper-voices/resolve/main"
    $voicePath = "$lang/$langRegion/$voiceName/$quality"

    Write-Host "[piper] Downloading voice $Voice..."

    foreach ($suffix in @(".onnx", ".onnx.json")) {
        $url  = "$hfBase/$voicePath/$Voice$suffix"
        $dest = "$VoicesDir\$Voice$suffix"
        Write-Host "  GET $url"
        try {
            Invoke-WebRequest $url -OutFile $dest -TimeoutSec 300
        } catch {
            Write-Error "Failed to download ${Voice}${suffix}: $_"
            exit 1
        }
    }
    Write-Host "[piper] Voice installed: $onnxFile" -ForegroundColor Green
}

# ── Smoke test ────────────────────────────────────────────────────────────────

Write-Host "`n[piper] Running smoke test..." -ForegroundColor Cyan
$testOut = "$env:TEMP\piper-test.wav"
try {
    $pcm = echo "Hello from Bonsai." | & $PiperExe --model $onnxFile --output-raw 2>$null
    if ($LASTEXITCODE -eq 0 -and $pcm.Length -gt 0) {
        Write-Host "[piper] Smoke test PASSED — Piper synthesized audio successfully." -ForegroundColor Green
    } else {
        Write-Warning "[piper] Smoke test returned exit $LASTEXITCODE or empty output. Check piper.exe manually."
    }
} catch {
    Write-Warning "[piper] Smoke test skipped (could not run piper.exe): $_"
}

Write-Host @"

[piper] Installation complete.
  Binary : $PiperExe
  Voice  : $onnxFile

The Bonsai TTS endpoint is now active:
  POST http://127.0.0.1:<port>/api/v1/tts/speak
  Body : {"text":"Hello", "voice":"$Voice"}

To install additional voices, run:
  .\install-piper.ps1 -Voice en_US-ryan-medium
"@ -ForegroundColor Cyan
