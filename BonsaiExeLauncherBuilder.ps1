param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceRoot = $ScriptDir
$FrontendDir = Join-Path $WorkspaceRoot "bonsai-workspace\src"
$TauriDir = Join-Path $WorkspaceRoot "bonsai-workspace\src-tauri"
$OutputExe = Join-Path $WorkspaceRoot "BonsaiWorkspace.exe"

function Write-Info([string]$Message) {
  Write-Host "[builder] $Message" -ForegroundColor Yellow
}

function Write-Success([string]$Message) {
  Write-Host "[builder] $Message" -ForegroundColor Green
}

function Write-FailAndExit([string]$Message, [int]$Code = 1) {
  Write-Host "[builder] ERROR: $Message" -ForegroundColor Red
  exit $Code
}

function Run-Step([string]$Description, [scriptblock]$Action) {
  Write-Info $Description
  & $Action
  if ($LASTEXITCODE -ne 0) {
    Write-FailAndExit "$Description failed with exit code $LASTEXITCODE" $LASTEXITCODE
  }
}

function Require-Command([string]$Name) {
  if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
    Write-FailAndExit "Required command '$Name' was not found on PATH."
  }
}

function Parse-NodeMajor([string]$VersionText) {
  $trimmed = $VersionText.Trim()
  if ($trimmed.StartsWith("v")) {
    $trimmed = $trimmed.Substring(1)
  }
  $majorToken = $trimmed.Split(".")[0]
  return [int]$majorToken
}

function Resolve-BuiltExePath([string]$TauriRoot) {
  $candidates = @(
    (Join-Path $TauriRoot "target\release\bonsai-workspace.exe")
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  $bundleRoot = Join-Path $TauriRoot "target\release\bundle"
  if (Test-Path $bundleRoot) {
    $bundleExe = Get-ChildItem -Path $bundleRoot -Recurse -Filter "bonsai-workspace.exe" -File -ErrorAction SilentlyContinue |
      Select-Object -First 1
    if ($bundleExe) {
      return $bundleExe.FullName
    }
  }

  return $null
}

try {
  Write-Info "Workspace root: $WorkspaceRoot"

  if (-not (Test-Path $FrontendDir)) {
    Write-FailAndExit "Frontend directory not found: $FrontendDir"
  }
  if (-not (Test-Path $TauriDir)) {
    Write-FailAndExit "Tauri directory not found: $TauriDir"
  }

  Write-Info "Running preflight checks"
  Require-Command "node"
  Require-Command "npm"
  Require-Command "cargo"
  Require-Command "npx"

  $nodeVersion = (& node --version).Trim()
  if ($LASTEXITCODE -ne 0) {
    Write-FailAndExit "Unable to read Node.js version"
  }
  $nodeMajor = Parse-NodeMajor $nodeVersion
  if ($nodeMajor -lt 18) {
    Write-FailAndExit "Node.js v18+ required. Found $nodeVersion"
  }
  Write-Success "Node.js: $nodeVersion"

  $npmVersion = (& npm --version).Trim()
  if ($LASTEXITCODE -ne 0) {
    Write-FailAndExit "Unable to read npm version"
  }
  Write-Success "npm: $npmVersion"

  $cargoVersion = (& cargo --version).Trim()
  if ($LASTEXITCODE -ne 0) {
    Write-FailAndExit "Unable to read cargo version"
  }
  Write-Success "cargo: $cargoVersion"

  $tauriViaNpx = $false
  $tauriVersion = ""
  $tauriVersion = (& npx tauri --version 2>$null).Trim()
  if ($LASTEXITCODE -eq 0 -and $tauriVersion) {
    $tauriViaNpx = $true
    Write-Success "tauri (npx): $tauriVersion"
  } else {
    $tauriVersion = (& cargo tauri --version 2>$null).Trim()
    if ($LASTEXITCODE -eq 0 -and $tauriVersion) {
      Write-Success "tauri (cargo): $tauriVersion"
    } else {
      Write-FailAndExit "Tauri CLI not found. Install @tauri-apps/cli or cargo-tauri."
    }
  }

  Push-Location $FrontendDir
  try {
    Run-Step "Installing frontend dependencies (npm install)" { npm install }
    Run-Step "Building frontend assets (npm run build)" { npm run build }
  } finally {
    Pop-Location
  }

  $distCandidates = @(
    (Join-Path $WorkspaceRoot "bonsai-workspace\dist"),
    (Join-Path $FrontendDir "dist")
  )
  $distPath = $distCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (-not $distPath) {
    Write-FailAndExit "Frontend build finished, but dist/ was not found in bonsai-workspace."
  }
  Write-Success "Frontend dist found: $distPath"

  if ($tauriViaNpx) {
    Push-Location $FrontendDir
    try {
      Run-Step "Building Tauri application with npx tauri build" { npx tauri build }
    } finally {
      Pop-Location
    }
  } else {
    Push-Location $TauriDir
    try {
      Run-Step "Building release executable with cargo build --release" { cargo build --release }
    } finally {
      Pop-Location
    }
  }

  $builtExe = Resolve-BuiltExePath $TauriDir
  if (-not $builtExe) {
    Write-FailAndExit "Could not locate built bonsai-workspace.exe under src-tauri/target/release or bundle output."
  }

  Copy-Item -Path $builtExe -Destination $OutputExe -Force
  if (-not (Test-Path $OutputExe)) {
    Write-FailAndExit "Copy step failed; output exe not found at $OutputExe"
  }

  $outputInfo = Get-Item $OutputExe
  $sizeMb = [Math]::Round($outputInfo.Length / 1MB, 2)
  Write-Success "Build complete."
  Write-Success "Output: $($outputInfo.FullName)"
  Write-Success "Size: $sizeMb MB"
  exit 0
}
catch {
  Write-FailAndExit $_.Exception.Message
}
