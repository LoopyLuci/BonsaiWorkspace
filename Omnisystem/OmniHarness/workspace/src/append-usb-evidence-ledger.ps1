param(
  [string]$ArtifactPath = "../../tool_test/android-usb-regression/latest.json",
  [string]$LedgerPath = "../../Runner-Streaming_System.md",
  [string]$RunReference = "local-shell",
  [string]$EvidenceSource = 'Local CLI (test:android-usb-regression)'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (!(Test-Path $ArtifactPath)) {
  throw "Artifact not found: $ArtifactPath"
}

if (!(Test-Path $LedgerPath)) {
  throw "Ledger doc not found: $LedgerPath"
}

$artifactRaw = Get-Content $ArtifactPath -Raw
$artifactJson = $artifactRaw | ConvertFrom-Json
$hash = (Get-FileHash $ArtifactPath -Algorithm SHA256).Hash

# Timestamp — read directly from JSON text to preserve original ISO string exactly.
$tsMatch = [regex]::Match($artifactRaw, '"ts"\s*:\s*"([^"]+)"')
if ($tsMatch.Success) {
  $ts = $tsMatch.Groups[1].Value
} else {
  $tsValue = $artifactJson.ts
  if ($tsValue -is [datetime]) {
    $ts = $tsValue.ToUniversalTime().ToString("o")
  } else {
    $ts = [string]$tsValue
  }
}

if ([string]::IsNullOrWhiteSpace($ts)) {
  throw "Artifact timestamp (ts) is missing."
}

$serial = [string]$artifactJson.serial
if ([string]::IsNullOrWhiteSpace($serial)) {
  $serial = "unknown"
}

$verdict = if ($artifactJson.ok) { "PASS" } else { "FAIL" }

# Support both camelCase (schema v1) and snake_case (schema v2) field names.
$apiPort = [string]$artifactJson.apiPort
if ([string]::IsNullOrWhiteSpace($apiPort)) {
  $apiPort = [string]$artifactJson.api_port
}
if ([string]::IsNullOrWhiteSpace($apiPort)) {
  $apiPort = "unknown"
}

# Strict mode (schema v2 field).
$strictMode = $false
if ($null -ne $artifactJson.strict_require_app) {
  $strictMode = [bool]$artifactJson.strict_require_app
}
$strictLabel = if ($strictMode) { "strict" } else { "non-strict" }

# Resolved APK path (schema v2 field).
$resolvedApk = ""
if ($null -ne $artifactJson.resolved_apk_path) {
  $resolvedApk = [string]$artifactJson.resolved_apk_path
}

# Find launch step for skip/hint notes (check both 'reason' and 'hint' fields).
$launchStep = $null
foreach ($s in $artifactJson.steps) {
  if ([string]$s.label -eq "launch app") {
    $launchStep = $s
    break
  }
}

$notes = "api port $apiPort; $strictLabel"
if ($launchStep -and $launchStep.skipped) {
  $skipNote = [string]$launchStep.hint
  if ([string]::IsNullOrWhiteSpace($skipNote)) {
    $skipNote = [string]$launchStep.reason
  }
  if ([string]::IsNullOrWhiteSpace($skipNote)) {
    $skipNote = "launch step skipped"
  }
  $notes = "$notes; $skipNote"
}
if (-not [string]::IsNullOrWhiteSpace($resolvedApk)) {
  $apkBase = [System.IO.Path]::GetFileName($resolvedApk)
  $notes = "$notes; apk: $apkBase"
}

# Keep table formatting stable — pipes inside cells break Markdown tables.
$notes = $notes -replace "\|", "/"

$row = "| $ts | $EvidenceSource | $RunReference | ``tool_test/android-usb-regression/latest.json`` | ``$hash`` | ``$serial`` | $verdict | $notes |"

$content = Get-Content $LedgerPath -Raw
if ($content.Contains($hash)) {
  Write-Host "Ledger already contains this artifact hash. No changes made."
  exit 0
}

$marker = "Ledger update rule:"
$idx = $content.IndexOf($marker)
if ($idx -lt 0) {
  throw "Could not locate ledger insertion marker: '$marker'"
}

$insert = "$row`r`n`r`n"
$updated = $content.Insert($idx, $insert)
Set-Content -Path $LedgerPath -Value $updated -Encoding UTF8

Write-Host "Appended ledger row to $LedgerPath"
Write-Host "Timestamp: $ts"
Write-Host "SHA256: $hash"
Write-Host "Strict: $strictLabel"
Write-Host "Verdict: $verdict"
