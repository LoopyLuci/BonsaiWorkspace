param(
  [ValidateSet('desktop', 'desktop+usb')]
  [string]$Mode = 'desktop',
  [switch]$PreflightOnly,
  [switch]$StrictApp,
  [switch]$NoTests,
  [string]$ApkPath,
  [string]$Serial,
  [string]$WifiHost,
  [int]$WifiPort = 5555,
  [int]$HealthTimeoutMs,
  [switch]$AllowPortInUse,
  [switch]$NoAttachExisting,
  [switch]$NoInstall,
  [switch]$Fast,
  [switch]$RemoteSurfaceSmoke,
  [string]$ReportPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$srcDir = Join-Path $workspaceRoot 'bonsai-workspace\src'
$defaultReportPath = Join-Path $workspaceRoot 'tool_test\launcher\latest.json'

if (-not (Test-Path $srcDir)) {
  throw "Could not find source directory: $srcDir"
}

Push-Location $srcDir
try {
  $effectiveReportPathOverride = $null
  $launchScript = Join-Path $srcDir 'orchestrate-bonsai-ecosystem.mjs'
  if (-not (Test-Path $launchScript)) {
    throw "Could not find launcher script: $launchScript"
  }

  $launchArgs = @($launchScript, '--mode', $Mode)

  if ($PreflightOnly) { $launchArgs += '--preflight-only' }
  if ($StrictApp) { $launchArgs += '--strict-app' }
  if ($NoTests) { $launchArgs += '--no-tests' }
  if ($ApkPath) { $launchArgs += @('--apk-path', $ApkPath) }
  if ($Serial) { $launchArgs += @('--serial', $Serial) }
  if ($WifiHost) { $launchArgs += @('--wifi-host', $WifiHost) }
  if ($PSBoundParameters.ContainsKey('WifiPort')) { $launchArgs += @('--wifi-port', [string]$WifiPort) }
  if ($PSBoundParameters.ContainsKey('HealthTimeoutMs')) { $launchArgs += @('--health-timeout-ms', [string]$HealthTimeoutMs) }
  if ($AllowPortInUse) { $launchArgs += '--allow-port-in-use' }
  if ($NoAttachExisting) { $launchArgs += '--no-attach-existing' }
  if ($NoInstall) { $launchArgs += '--no-install' }
  if ($Fast) { $launchArgs += '--fast' }
  if ($RemoteSurfaceSmoke) { $launchArgs += '--remote-surface-smoke' }
  if ($ReportPath) {
    $resolvedReportPath = if ([System.IO.Path]::IsPathRooted($ReportPath)) {
      $ReportPath
    } else {
      Join-Path $workspaceRoot $ReportPath
    }
    $effectiveReportPathOverride = $resolvedReportPath
    $launchArgs += @('--report-path', $resolvedReportPath)
  }

  # ── Survival Watchdog ────────────────────────────────────────────────────
  # Start the watchdog before the main app so it can monitor and restart Bonsai
  # if it crashes.  Best-effort: skip silently if the binary isn't built yet.
  $watchdogCandidates = @(
    (Join-Path $workspaceRoot 'target\release\bonsai-watchdog.exe'),
    (Join-Path $workspaceRoot 'bonsai-workspace\src-tauri\target\release\bonsai-watchdog.exe'),
    (Join-Path $env:USERPROFILE '.bonsai\bin\bonsai-watchdog.exe')
  )
  $watchdogExe = $watchdogCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1

  if ($watchdogExe) {
    $existing = Get-Process -Name 'bonsai-watchdog' -ErrorAction SilentlyContinue
    if (-not $existing) {
      Write-Host "[one-click] Starting survival watchdog: $watchdogExe" -ForegroundColor Magenta
      Start-Process -FilePath $watchdogExe -WindowStyle Hidden -PassThru | Out-Null
      Start-Sleep -Milliseconds 600   # brief pause to let it arm the KB
    } else {
      Write-Host "[one-click] Watchdog already running (PID $($existing.Id))" -ForegroundColor DarkMagenta
    }
  } else {
    Write-Host "[one-click] Watchdog binary not found — run BonsaiExeLauncherBuilder.ps1 to build it." -ForegroundColor DarkYellow
  }
  # ─────────────────────────────────────────────────────────────────────────

  Write-Host "[one-click] Launching Bonsai Workspace from $srcDir" -ForegroundColor Cyan
  Write-Host "[one-click] node $($launchArgs -join ' ')" -ForegroundColor DarkGray

  & node @launchArgs
  $nodeExit = $LASTEXITCODE

  # launch-all writes a structured report; prefer that truth source when npm exits
  # nonzero despite a healthy/complete launch sequence.
  $effectiveReportPath = if ($effectiveReportPathOverride) {
    $effectiveReportPathOverride
  } else {
    $defaultReportPath
  }

  # If the user chose to exit cleanly (not a crash), stop the watchdog too.
  # The watchdog will restart Bonsai on its own if it detects a crash exit.
  if ($nodeExit -eq 0) {
    Get-Process -Name 'bonsai-watchdog' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
  }

  if ($nodeExit -ne 0 -and (Test-Path $effectiveReportPath)) {
    try {
      $report = Get-Content -Raw -Path $effectiveReportPath | ConvertFrom-Json
      if ($null -ne $report -and $report.ok -eq $true) {
        Write-Host "[one-click] launch report indicates success; normalizing wrapper exit code to 0." -ForegroundColor Yellow
        exit 0
      }
    }
    catch {
      # If report parsing fails, keep original node exit code.
    }
  }

  exit $nodeExit
}
finally {
  Pop-Location
}
