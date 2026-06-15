#!/usr/bin/env pwsh
# Build Windows installer using NSIS
# Usage: .\build-windows-installer.ps1

param(
    [string]$OutputDir = "dist\windows"
)

$ErrorActionPreference = "Stop"
$workspace = Get-Location
$installerName = "BonsaiEcosystem-Setup.exe"

Write-Host "🔨 Building Windows installer..." -ForegroundColor Cyan

# Step 1: Build all binaries in release mode
Write-Host "  Building release binaries..." -ForegroundColor Yellow
try {
    cargo build --release -p bonsai-workspace -p bonsai-model-workshop -p bonsai-mcp-manager
    if ($LASTEXITCODE -ne 0) { throw "Build failed" }
} catch {
    Write-Host "  ❌ Build failed: $_" -ForegroundColor Red
    exit 1
}

# Step 2: Create output directory
New-Item -ItemType Directory -Force -Path "$OutputDir\app" | Out-Null

# Step 3: Copy binaries
Write-Host "  Copying binaries..." -ForegroundColor Yellow
Copy-Item "target\release\bonsai-workspace.exe" "$OutputDir\app\" -Force
Copy-Item "target\release\bonsai-model-workshop.exe" "$OutputDir\app\" -Force
Copy-Item "target\release\bonsai-mcp-manager.exe" "$OutputDir\app\" -Force

# Step 4: Generate NSIS installer script
$nsisScript = @"
!include "MUI2.nsh"

Name "Bonsai Ecosystem"
OutFile "BonsaiEcosystem-Setup.exe"
InstallDir "`$PROGRAMFILES\BonsaiEcosystem"
RequestExecutionLevel admin

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_LANGUAGE "English"

Section "Install"
    SetOutPath "`$INSTDIR"
    File /r "app\*"

    CreateDirectory "`$SMPROGRAMS\Bonsai Ecosystem"
    CreateShortCut "`$SMPROGRAMS\Bonsai Ecosystem\Bonsai Workspace.lnk" "`$INSTDIR\bonsai-workspace.exe"
    CreateShortCut "`$SMPROGRAMS\Bonsai Ecosystem\Model Workshop.lnk" "`$INSTDIR\bonsai-model-workshop.exe"
    CreateShortCut "`$SMPROGRAMS\Bonsai Ecosystem\MCP Manager.lnk" "`$INSTDIR\bonsai-mcp-manager.exe"
    CreateShortCut "`$DESKTOP\Bonsai Workspace.lnk" "`$INSTDIR\bonsai-workspace.exe"

    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\BonsaiEcosystem" "DisplayName" "Bonsai Ecosystem"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\BonsaiEcosystem" "UninstallString" "`$INSTDIR\uninstall.exe"
    WriteUninstaller "`$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
    Delete "`$INSTDIR\*.*"
    RMDir /r "`$INSTDIR"
    Delete "`$SMPROGRAMS\Bonsai Ecosystem\*.*"
    RMDir "`$SMPROGRAMS\Bonsai Ecosystem"
    Delete "`$DESKTOP\Bonsai Workspace.lnk"
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\BonsaiEcosystem"
SectionEnd
"@

$nsisScript | Set-Content "$OutputDir\installer.nsi" -Encoding UTF8

# Step 5: Compile installer with NSIS
Write-Host "  Compiling NSIS installer..." -ForegroundColor Yellow
$nsisPath = "C:\Program Files (x86)\NSIS\makensis.exe"
if (Test-Path $nsisPath) {
    Push-Location $OutputDir
    & $nsisPath "installer.nsi"
    Pop-Location

    if (Test-Path "$OutputDir\BonsaiEcosystem-Setup.exe") {
        Write-Host "✅ Windows installer created: $OutputDir\BonsaiEcosystem-Setup.exe" -ForegroundColor Green
    } else {
        Write-Host "❌ NSIS compilation failed" -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "⚠️  NSIS not found at $nsisPath - please install NSIS" -ForegroundColor Yellow
    Write-Host "   Download from: https://nsis.sourceforge.io/" -ForegroundColor Gray
}
