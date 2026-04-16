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
  [string]$ReportPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$workspaceRoot = $PSScriptRoot
$srcDir = Join-Path $workspaceRoot 'bonsai-workspace\src'
$defaultReportPath = Join-Path $workspaceRoot 'tool_test\launcher\latest.json'

if (-not (Test-Path $srcDir)) {
  throw "Could not find source directory: $srcDir"
}

Push-Location $srcDir
try {
  $launchArgs = @('run', 'launch:all', '--', '--mode', $Mode)

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
  if ($ReportPath) { $launchArgs += @('--report-path', $ReportPath) }

  Write-Host "[one-click] Launching Bonsai Workspace from $srcDir" -ForegroundColor Cyan
  Write-Host "[one-click] npm $($launchArgs -join ' ')" -ForegroundColor DarkGray

  & npm @launchArgs
  $npmExit = $LASTEXITCODE

  # launch-all writes a structured report; prefer that truth source when npm exits
  # nonzero despite a healthy/complete launch sequence.
  $effectiveReportPath = if ($ReportPath) {
    if ([System.IO.Path]::IsPathRooted($ReportPath)) { $ReportPath } else { Join-Path $srcDir $ReportPath }
  } else {
    $defaultReportPath
  }

  if ($npmExit -ne 0 -and (Test-Path $effectiveReportPath)) {
    try {
      $report = Get-Content -Raw -Path $effectiveReportPath | ConvertFrom-Json
      if ($null -ne $report -and $report.ok -eq $true) {
        Write-Host "[one-click] launch report indicates success; normalizing wrapper exit code to 0." -ForegroundColor Yellow
        exit 0
      }
    }
    catch {
      # If report parsing fails, keep original npm exit code.
    }
  }

  exit $npmExit
}
finally {
  Pop-Location
}
