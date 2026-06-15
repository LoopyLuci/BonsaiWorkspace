# =============================================================================
# Bonsai Build and Run Script
# Compiles all launchers, Android APKs, and runs the Bonsai Workspace desktop app
# =============================================================================

param(
    [switch]$SkipAndroid,
    [switch]$OnlyDesktop,
    [switch]$OnlyAndroid,
    [switch]$OnlyBuild
)

# Setup PATH with Rust/Cargo
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$CARGO = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$RUSTC = "$env:USERPROFILE\.cargo\bin\rustc.exe"
$ROOT = "Z:\Projects\BonsaiEcosystem"
$WS_DIR = "$ROOT\bonsai-workspace"

Write-Host "🚀 Bonsai Build and Run Script" -ForegroundColor Cyan
Write-Host "===============================" -ForegroundColor Cyan
Write-Host ""
Write-Host "✓ Rust version: $(& $RUSTC --version)" -ForegroundColor Green
Write-Host "✓ Cargo version: $(& $CARGO --version)" -ForegroundColor Green
Write-Host "✓ Node version: $(node --version)" -ForegroundColor Green
Write-Host "✓ npm version: $(npm --version)" -ForegroundColor Green
Write-Host ""

# =============================================================================
# PHASE 1: Build Desktop App (Bonsai Workspace)
# =============================================================================

if (-not $OnlyAndroid) {
    Write-Host "📦 Building Bonsai Workspace (Desktop App)..." -ForegroundColor Yellow
    Write-Host "================================================" -ForegroundColor Yellow

    Set-Location $WS_DIR

    # Install dependencies in src directory
    Write-Host "📝 Installing npm dependencies..." -ForegroundColor Cyan
    Set-Location "$WS_DIR\src"
    npm install --legacy-peer-deps

    if ($LASTEXITCODE -ne 0) {
        Write-Host "⚠️  npm install in src had issues, continuing..." -ForegroundColor Yellow
    }

    # Also install Tauri dependencies in src-tauri
    Set-Location "$WS_DIR\src-tauri"
    npm install --legacy-peer-deps

    if ($LASTEXITCODE -ne 0) {
        Write-Host "⚠️  npm install in src-tauri had issues, continuing..." -ForegroundColor Yellow
    }

    # Build the Tauri app
    Write-Host "🔨 Compiling Tauri application..." -ForegroundColor Cyan
    Set-Location $WS_DIR

    # Use cargo tauri with correct syntax
    & $CARGO tauri build

    if ($LASTEXITCODE -ne 0) {
        Write-Host "❌ Tauri build failed" -ForegroundColor Red
        exit 1
    }

    Write-Host "✅ Bonsai Workspace built successfully!" -ForegroundColor Green
    Write-Host ""
}

# =============================================================================
# PHASE 2: Build Android APK
# =============================================================================

if (-not $OnlyDesktop -and -not $SkipAndroid) {
    Write-Host "📱 Building Bonsai Android APK..." -ForegroundColor Yellow
    Write-Host "===============================================" -ForegroundColor Yellow

    $ANDROID_DIR = "$ROOT\android-runtime"

    if (Test-Path $ANDROID_DIR) {
        Set-Location $ANDROID_DIR

        # Check for Gradle
        $GRADLE = Get-Command gradle -ErrorAction SilentlyContinue

        if ($GRADLE) {
            Write-Host "🔨 Compiling Android APK with Gradle..." -ForegroundColor Cyan
            gradle assembleRelease

            if ($LASTEXITCODE -eq 0) {
                Write-Host "✅ Android APK built successfully!" -ForegroundColor Green
                $APK = Get-ChildItem -Path "app/build/outputs/apk/release/*.apk" -ErrorAction SilentlyContinue | Select-Object -First 1
                if ($APK) {
                    Write-Host "📍 APK location: $($APK.FullName)" -ForegroundColor Cyan
                }
            } else {
                Write-Host "⚠️  Android build skipped or failed (Gradle not properly configured)" -ForegroundColor Yellow
            }
        } else {
            Write-Host "⚠️  Gradle not found, skipping Android build" -ForegroundColor Yellow
        }
    } else {
        Write-Host "⚠️  Android directory not found at $ANDROID_DIR, skipping Android build" -ForegroundColor Yellow
    }

    Write-Host ""
}

# =============================================================================
# PHASE 3: Find and Run the Desktop App
# =============================================================================

if (-not $OnlyBuild -and -not $OnlyAndroid) {
    Write-Host "🎯 Launching Bonsai Workspace..." -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Yellow

    Set-Location "$ROOT\bonsai-workspace"

    # Find the built executable
    $TARGET_DIR = "$ROOT\bonsai-workspace\src-tauri\target\release"
    $EXE = Get-ChildItem -Path "$TARGET_DIR\*.exe" -ErrorAction SilentlyContinue | Where-Object {$_.Name -match "Bonsai|bonsai-workspace"} | Select-Object -First 1

    if ($EXE) {
        Write-Host "✅ Found executable: $($EXE.Name)" -ForegroundColor Green
        Write-Host "🚀 Launching application..." -ForegroundColor Cyan
        Write-Host ""

        # Run the app
        & $EXE.FullName
    } else {
        Write-Host "❌ Could not find compiled executable in $TARGET_DIR" -ForegroundColor Red
        Write-Host "Try running with --OnlyBuild first" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "Available executables:" -ForegroundColor Cyan
        Get-ChildItem -Path "$TARGET_DIR\*.exe" -ErrorAction SilentlyContinue | Select-Object Name
        exit 1
    }
}

Write-Host ""
Write-Host "✅ Build complete!" -ForegroundColor Green
