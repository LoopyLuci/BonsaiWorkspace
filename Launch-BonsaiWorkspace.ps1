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
  exit $LASTEXITCODE
}
finally {
  Pop-Location
}
