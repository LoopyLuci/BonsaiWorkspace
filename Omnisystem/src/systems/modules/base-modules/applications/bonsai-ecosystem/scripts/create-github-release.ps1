#!/usr/bin/env pwsh
# Create a GitHub release with all built artifacts
# Usage: .\create-github-release.ps1 -Version "1.0.0"

param(
    [string]$Version = "1.0.0",
    [string]$TagMessage = "Bonsai Ecosystem v$Version"
)

$ErrorActionPreference = "Stop"
$workspace = Get-Location
$distDir = Join-Path $workspace "dist"

Write-Host "🚀 Creating GitHub Release v$Version..." -ForegroundColor Cyan

# Verify gh CLI is installed
try {
    $null = gh --version
} catch {
    Write-Host "❌ GitHub CLI (gh) not found. Install from: https://cli.github.com/" -ForegroundColor Red
    exit 1
}

# Create tag
Write-Host "  Creating git tag v$Version..." -ForegroundColor Yellow
try {
    git tag -a "v$Version" -m $TagMessage
    git push origin "v$Version"
    Write-Host "  ✅ Tag created and pushed" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Tag already exists or push failed: $_" -ForegroundColor Yellow
}

# Collect artifacts
$artifacts = @()
$artifactFiles = Get-ChildItem -Path $distDir -Recurse -File | Where-Object {
    $_.Extension -in @(".exe", ".dmg", ".AppImage", ".apk", ".aab")
}

if ($artifactFiles.Count -eq 0) {
    Write-Host "⚠️  No artifacts found in $distDir" -ForegroundColor Yellow
    Write-Host "   Run the build pipeline first:" -ForegroundColor Gray
    Write-Host "   ./scripts/ci/bonsai-ci-runner.ps1 -Stage all" -ForegroundColor Gray
} else {
    Write-Host "  Found $($artifactFiles.Count) artifact(s):" -ForegroundColor Yellow
    foreach ($file in $artifactFiles) {
        $sizeMB = [Math]::Round($file.Length / 1MB, 2)
        Write-Host "    📦 $($file.Name) ($sizeMB MB)" -ForegroundColor Cyan
        $artifacts += $file.FullName
    }
}

# Create release on GitHub
$releaseNotes = @"
## 🚀 Bonsai Ecosystem v$Version

### 🧬 Desktop Applications
- **Bonsai Workspace IDE** — AI-powered code editor and orchestration system
- **Model Workshop** — Design, build, and train AI models with beautiful UI
- **MCP Manager** — Configure MCP servers, clients, and tools
- **Bonsai Nexus** — Unified launcher for all ecosystem applications

### 📱 Android Applications
- **Bonsai Buddy** — AI companion with real-time WebSocket updates
- **Model Workshop Mobile** — Manage models on the go
- **MCP Manager Mobile** — Monitor MCP servers from device

### 🚀 Backend Services
- **Octopus AI** — Distributed inference engine
- **Bonsai API Gateway** — REST/GraphQL API layer
- **BMF Messaging** — Sovereign email/SMS delivery system

### ✨ Features
✅ Real-time WebSocket support for live training progress
✅ Native installers for Windows, macOS, Linux, and Android
✅ Fully automated CI/CD pipeline
✅ Cross-platform support with unified UI
✅ Production-grade stability

### 📥 Installation
- **Windows**: Run \`BonsaiEcosystem-Setup.exe\` and follow the installer
- **macOS**: Drag \`BonsaiEcosystem.app\` to Applications
- **Linux**: Run \`BonsaiEcosystem-x86_64.AppImage\`
- **Android**: Install \`BonsaiBuddy-release.apk\` or use Play Store

See [CHANGELOG.md](CHANGELOG.md) for full details.
"@

Write-Host "  Creating GitHub release..." -ForegroundColor Yellow

$args = @(
    "release", "create", "v$Version",
    "--title", "Bonsai Ecosystem v$Version",
    "--notes", $releaseNotes,
    "--draft:$false"
)

# Add artifact files
foreach ($artifact in $artifacts) {
    $args += $artifact
}

try {
    & gh @args
    Write-Host "✅ GitHub Release v$Version created successfully!" -ForegroundColor Green
    Write-Host "   View at: https://github.com/LoopyLuci/BonsaiWorkspace/releases/tag/v$Version" -ForegroundColor Cyan
} catch {
    Write-Host "❌ Failed to create release: $_" -ForegroundColor Red
    exit 1
}
