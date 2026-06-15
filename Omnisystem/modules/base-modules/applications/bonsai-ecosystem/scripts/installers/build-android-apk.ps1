#!/usr/bin/env pwsh
# Build Android APKs for all Bonsai apps
# Usage: .\build-android-apk.ps1

param(
    [string]$OutputDir = "dist\android"
)

$ErrorActionPreference = "Stop"
$workspace = Get-Location

Write-Host "🔨 Building Android APKs..." -ForegroundColor Cyan

$apps = @(
    @{Name="BonsaiBuddy"; Path="android-runtime"; Output="BonsaiBuddy-release.apk"},
    @{Name="ModelWorkshop"; Path="android\BonsaiModelWorkshop"; Output="ModelWorkshop-release.apk"},
    @{Name="McpManager"; Path="android\BonsaiMcpManager"; Output="McpManager-release.apk"}
)

# Create output directory
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

foreach ($app in $apps) {
    $appPath = Join-Path $workspace $app.Path

    if (-not (Test-Path $appPath)) {
        Write-Host "  ⚠️  $($app.Name) not found at $appPath - skipping" -ForegroundColor Yellow
        continue
    }

    Write-Host "  Building $($app.Name)..." -ForegroundColor Yellow

    Push-Location $appPath

    try {
        # Clean
        Write-Host "    Cleaning..." -ForegroundColor Gray
        & ./gradlew clean -q 2>$null

        # Build release APK
        Write-Host "    Compiling..." -ForegroundColor Gray
        & ./gradlew assembleRelease -q 2>$null

        # Find and copy APK
        $apkPath = Get-ChildItem -Path "app/build/outputs/apk/release/" -Filter "*.apk" -Recurse | Select-Object -First 1

        if ($apkPath) {
            Copy-Item $apkPath.FullName "$OutputDir\$($app.Output)" -Force
            Write-Host "    ✅ $($app.Output) ($([Math]::Round($apkPath.Length / 1MB, 2)) MB)" -ForegroundColor Green
        } else {
            Write-Host "    ❌ APK not found" -ForegroundColor Red
        }
    } catch {
        Write-Host "    ❌ Build failed: $_" -ForegroundColor Red
    } finally {
        Pop-Location
    }
}

Write-Host "✅ Android APK builds complete: $OutputDir" -ForegroundColor Green
