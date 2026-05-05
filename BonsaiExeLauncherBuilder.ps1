param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceRoot = $ScriptDir
$FrontendDir = Join-Path $WorkspaceRoot "bonsai-workspace\src"
$TauriDir = Join-Path $WorkspaceRoot "bonsai-workspace\src-tauri"
$OutputExe = Join-Path $WorkspaceRoot "BonsaiWorkspace.exe"
$WorkspaceTargetRelease = Join-Path $WorkspaceRoot "target\release"

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
    (Join-Path $TauriRoot "target\release\bonsai-workspace.exe"),
    (Join-Path $WorkspaceTargetRelease "bonsai-workspace.exe")
  )

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  $bundleRoots = @(
    (Join-Path $TauriRoot "target\release\bundle"),
    (Join-Path $WorkspaceTargetRelease "bundle")
  )
  foreach ($bundleRoot in $bundleRoots) {
    if (Test-Path $bundleRoot) {
      $bundleExe = Get-ChildItem -Path $bundleRoot -Recurse -Filter "*.exe" -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match 'bonsai[- ]workspace' } |
        Select-Object -First 1
      if ($bundleExe) {
        return $bundleExe.FullName
      }
    }
  }

  return $null
}

function Test-TauriViaCargo {
  $v = (& cargo tauri --version 2>$null).Trim()
  if ($LASTEXITCODE -eq 0 -and $v) {
    return @{ ok = $true; version = $v }
  }
  return @{ ok = $false; version = "" }
}

function Test-TauriViaNpx([switch]$NoInstall) {
  Push-Location $FrontendDir
  try {
    $args = @()
    if ($NoInstall) { $args += "--no-install" }
    $args += @("tauri", "--version")
    $v = (& npx @args 2>$null).Trim()
    if ($LASTEXITCODE -eq 0 -and $v) {
      return @{ ok = $true; version = $v; noInstall = [bool]$NoInstall }
    }
  } finally {
    Pop-Location
  }
  return @{ ok = $false; version = ""; noInstall = [bool]$NoInstall }
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
  if (-not (Get-Command "npx" -ErrorAction SilentlyContinue)) {
    Write-Info "npx not found on PATH; will prefer cargo-tauri if available."
  }

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

  $tauriBuildMethod = ""
  $npxNoInstall = $false
  $cargoTauri = Test-TauriViaCargo
  if ($cargoTauri.ok) {
    $tauriBuildMethod = "cargo-tauri"
    Write-Success "tauri (cargo): $($cargoTauri.version)"
  } else {
    if (Get-Command "npx" -ErrorAction SilentlyContinue) {
      $npxLocal = Test-TauriViaNpx -NoInstall
      if ($npxLocal.ok) {
        $tauriBuildMethod = "npx-tauri"
        $npxNoInstall = $true
        Write-Success "tauri (npx --no-install): $($npxLocal.version)"
      } else {
        $npxRemote = Test-TauriViaNpx
        if ($npxRemote.ok) {
          $tauriBuildMethod = "npx-tauri"
          $npxNoInstall = $false
          Write-Success "tauri (npx): $($npxRemote.version)"
          Write-Info "Using network-enabled npx tauri fallback."
        }
      }
    }
  }

  if (-not $tauriBuildMethod) {
    Write-Info "Tauri CLI not detected; will use cargo build --release fallback for local exe."
  }

  if (Get-Command sccache -ErrorAction SilentlyContinue) {
    $env:RUSTC_WRAPPER = "sccache"
    Write-Success "sccache enabled"
  } else {
    Write-Host "[builder] Tip: install sccache for faster rebuilds (cargo install sccache)" -ForegroundColor DarkYellow
  }

  $tauriBuildOk = $false
  if ($tauriBuildMethod -eq "cargo-tauri" -or $tauriBuildMethod -eq "npx-tauri") {
    Push-Location $FrontendDir
    try {
      Run-Step "Installing frontend dependencies (npm install)" { npm install --prefer-offline --no-audit --no-fund --loglevel=error }
    } finally {
      Pop-Location
    }
  }

  if ($tauriBuildMethod -eq "cargo-tauri") {
    Push-Location $TauriDir
    try {
      Write-Info "Building with cargo tauri build (includes frontend via beforeBuildCommand)"
      & cargo tauri build
      if ($LASTEXITCODE -eq 0) {
        $tauriBuildOk = $true
      } else {
        Write-Host "[builder] WARNING: cargo tauri build failed with exit code $LASTEXITCODE. Falling back to cargo build --release." -ForegroundColor Yellow
      }
    } finally {
      Pop-Location
    }
  }

  if (-not $tauriBuildOk -and $tauriBuildMethod -eq "npx-tauri") {
    Push-Location $FrontendDir
    try {
      Write-Info "Building with npx tauri build (includes frontend via beforeBuildCommand)"
      $npxArgs = @()
      if ($npxNoInstall) { $npxArgs += "--no-install" }
      $npxArgs += @("tauri", "build")
      & npx @npxArgs
      if ($LASTEXITCODE -eq 0) {
        $tauriBuildOk = $true
      } else {
        Write-Host "[builder] WARNING: npx tauri build failed with exit code $LASTEXITCODE. Falling back to cargo build --release." -ForegroundColor Yellow
      }
    } finally {
      Pop-Location
    }
  }

  if (-not $tauriBuildOk) {
    Push-Location $FrontendDir
    try {
      Run-Step "Installing frontend dependencies (npm install)" { npm install --prefer-offline --no-audit --no-fund --loglevel=error }
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

    Push-Location $TauriDir
    try {
      Run-Step "Building release executable with cargo build --release (fallback)" { cargo build --release }
      $tauriBuildOk = $true
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
