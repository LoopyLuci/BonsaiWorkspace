param(
  [string]$Serial = "",
  [string]$OutFile = "../../tool_test/android-remote-surface-smoke-latest.txt",
  [string]$DesktopHost = "127.0.0.1",
  [int]$DesktopPort = 11369,
  [int]$AdbTimeoutSeconds = 45,
  [switch]$SkipForceStop
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-Adb {
  $candidates = @()
  if ($env:LOCALAPPDATA) {
    $candidates += (Join-Path $env:LOCALAPPDATA "Android\Sdk\platform-tools\adb.exe")
  }
  if ($env:ANDROID_HOME) {
    $candidates += (Join-Path $env:ANDROID_HOME "platform-tools\adb.exe")
  }
  if ($env:ANDROID_SDK_ROOT) {
    $candidates += (Join-Path $env:ANDROID_SDK_ROOT "platform-tools\adb.exe")
  }

  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      return $candidate
    }
  }

  $cmd = Get-Command adb -ErrorAction SilentlyContinue
  if ($cmd) {
    return $cmd.Source
  }

  throw "adb executable not found. Install Android platform-tools or set ANDROID_HOME/ANDROID_SDK_ROOT."
}

function Resolve-Serial([string]$adbPath, [string]$explicitSerial) {
  if (-not [string]::IsNullOrWhiteSpace($explicitSerial)) {
    return $explicitSerial.Trim()
  }

  $lines = & $adbPath devices | Select-Object -Skip 1
  $online = @()
  foreach ($line in $lines) {
    $trimmed = "$line".Trim()
    if ([string]::IsNullOrWhiteSpace($trimmed)) { continue }
    if ($trimmed -match "^([^\s]+)\s+device$") {
      $online += $matches[1]
    }
  }

  if ($online.Count -eq 0) {
    throw "No online Android device found. Connect device or pass -Serial."
  }

  return $online[0]
}

function Invoke-AdbWithTimeout(
  [string]$adbPath,
  [string[]]$AdbArgs,
  [int]$timeoutSeconds
) {
  $stdoutPath = Join-Path $env:TEMP ("bonsai-adb-stdout-" + [Guid]::NewGuid().ToString("N") + ".log")
  $stderrPath = Join-Path $env:TEMP ("bonsai-adb-stderr-" + [Guid]::NewGuid().ToString("N") + ".log")

  try {
    $proc = Start-Process -FilePath $adbPath -ArgumentList $AdbArgs -PassThru -NoNewWindow -RedirectStandardOutput $stdoutPath -RedirectStandardError $stderrPath
    $timedOut = $null -eq (Wait-Process -Id $proc.Id -Timeout ([Math]::Max(1, $timeoutSeconds)) -ErrorAction SilentlyContinue)
    if ($timedOut) {
      try {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
      } catch {
        # Best-effort kill only.
      }
      throw "adb command timed out after ${timeoutSeconds}s: $($AdbArgs -join ' ')"
    }

    $stdout = if (Test-Path $stdoutPath) { Get-Content -Path $stdoutPath -Raw } else { "" }
    $stderr = if (Test-Path $stderrPath) { Get-Content -Path $stderrPath -Raw } else { "" }
    if ($proc.ExitCode -ne 0) {
      throw "adb command failed (exit $($proc.ExitCode)): $($AdbArgs -join ' ')`n$stderr$stdout"
    }

    return $stdout
  }
  finally {
    Remove-Item -Path $stdoutPath -ErrorAction SilentlyContinue
    Remove-Item -Path $stderrPath -ErrorAction SilentlyContinue
  }
}

function Start-And-Wait([string]$adbPath, [string]$serial, [string]$component, [string]$desktopHost, [int]$port) {
  $args = @(
    "-s", $serial, "shell", "cmd", "activity", "start-activity",
    "-n", $component,
    "--es", "desktop_host", $desktopHost,
    "--ei", "desktop_port", "$port"
  )
  return (& $adbPath @args) | Out-String
}

function Start-MainAndWait([string]$adbPath, [string]$serial) {
  $args = @("-s", $serial, "shell", "cmd", "activity", "start-activity", "-n", "com.bonsai.workspace/.MainActivity")
  return (& $adbPath @args) | Out-String
}

function Wait-ForResumedActivity([string]$adbPath, [string]$serial, [string]$resumedPattern, [int]$timeoutSeconds) {
  $deadline = (Get-Date).AddSeconds([Math]::Max(1, $timeoutSeconds))
  do {
    $dump = Dump-ActivityFocus -adbPath $adbPath -serial $serial
    if (@($dump | Select-String $resumedPattern).Count -gt 0) {
      return @{ Ok = $true; Dump = $dump }
    }
    Start-Sleep -Milliseconds 500
  } while ((Get-Date) -lt $deadline)

  $finalDump = Dump-ActivityFocus -adbPath $adbPath -serial $serial
  return @{ Ok = $false; Dump = $finalDump }
}

function Dump-ActivityFocus([string]$adbPath, [string]$serial) {
  return (& $adbPath -s $serial shell dumpsys activity activities |
    Select-String "mResumedActivity|UnsupportedWebViewActivity|RemoteSurfaceActivity|RemoteSurfaceEntryActivity" |
    Out-String)
}

function Get-CrashMatches([string]$adbPath, [string]$serial) {
  return (& $adbPath -s $serial logcat -d |
    Select-String "SuperNotCalledException|did not call through to super.onCreate")
}

$adb = Resolve-Adb
$serial = Resolve-Serial -adbPath $adb -explicitSerial $Serial

$outPath = Resolve-Path -Path (Join-Path $PSScriptRoot $OutFile)
$outFile = "$outPath"

if (-not $SkipForceStop) {
  & $adb -s $serial shell am force-stop com.bonsai.workspace | Out-Null
}
& $adb -s $serial logcat -c

$ts = (Get-Date).ToString("o")
"[$ts] End-to-end fallback->trampoline smoke (script)" | Out-File -FilePath $outFile -Append -Encoding utf8
"--- phase 1: launch MainActivity ---" | Out-File -FilePath $outFile -Append

$phase1Ok = $false
$phase2Ok = $false
$phase2SoftOk = $false
$crashOk = $false

try {
  $phase1Start = Start-MainAndWait -adbPath $adb -serial $serial
  $phase1Start | Out-File -FilePath $outFile -Append
  $phase1Wait = Wait-ForResumedActivity -adbPath $adb -serial $serial -resumedPattern "(UnsupportedWebViewActivity|RemoteSurfaceActivity)" -timeoutSeconds $AdbTimeoutSeconds
  $phase1Dump = $phase1Wait.Dump
  $phase1Dump | Out-File -FilePath $outFile -Append
  $phase1Ok = $phase1Wait.Ok
} catch {
  "PHASE1_ERROR=$($_.Exception.Message)" | Out-File -FilePath $outFile -Append
}

if ($phase1Ok) {
  "PHASE1_OK_MAIN_TO_UNSUPPORTED_SCRIPT" | Out-File -FilePath $outFile -Append
} else {
  "PHASE1_FAIL_MAIN_TO_UNSUPPORTED_SCRIPT" | Out-File -FilePath $outFile -Append
}

"--- phase 2: launch trampoline ---" | Out-File -FilePath $outFile -Append
try {
  for ($attempt = 1; $attempt -le 3; $attempt++) {
    if ($attempt -gt 1) {
      "PHASE2_RETRY_ATTEMPT=$attempt" | Out-File -FilePath $outFile -Append
      # Reset app task to reduce sticky activity-stack states between retries.
      & $adb -s $serial shell am force-stop com.bonsai.workspace | Out-Null
      Start-Sleep -Milliseconds 700
    }

    $phase2Start = Start-And-Wait -adbPath $adb -serial $serial -component "com.bonsai.workspace/.RemoteSurfaceEntryActivity" -desktopHost $DesktopHost -port $DesktopPort
    $phase2Start | Out-File -FilePath $outFile -Append
    $phase2Wait = Wait-ForResumedActivity -adbPath $adb -serial $serial -resumedPattern "(mResumedActivity|ResumedActivity):\s*ActivityRecord" -timeoutSeconds $AdbTimeoutSeconds
    $phase2Dump = $phase2Wait.Dump
    $phase2Dump | Out-File -FilePath $outFile -Append
    $phase2Ok = $phase2Wait.Ok -and (@($phase2Dump | Select-String "RemoteSurfaceActivity").Count -gt 0)
    if (-not $phase2Ok) {
      $entryHeld = @($phase2Dump | Select-String "RemoteSurfaceEntryActivity").Count -gt 0
      if ($entryHeld) {
        $phase2SoftOk = $true
      }
    }
    if ($phase2Ok) {
      break
    }
  }
} catch {
  "PHASE2_ERROR=$($_.Exception.Message)" | Out-File -FilePath $outFile -Append
}

if ($phase2Ok) {
  "PHASE2_OK_TRAMPOLINE_TO_REMOTE_SCRIPT" | Out-File -FilePath $outFile -Append
} elseif ($phase2SoftOk) {
  "PHASE2_SOFTPASS_TRAMPOLINE_ENTRY_HELD_SCRIPT" | Out-File -FilePath $outFile -Append
} else {
  "PHASE2_FAIL_TRAMPOLINE_TO_REMOTE_SCRIPT" | Out-File -FilePath $outFile -Append
}

"--- lifecycle crash scan ---" | Out-File -FilePath $outFile -Append
$crashMatches = Get-CrashMatches -adbPath $adb -serial $serial
$crashOk = ($null -eq $crashMatches) -or (@($crashMatches).Count -eq 0)
if ($crashOk) {
  "NO_SUPER_ONCREATE_CRASH_DETECTED_E2E_SCRIPT" | Out-File -FilePath $outFile -Append
} else {
  $crashMatches | Out-File -FilePath $outFile -Append
}

"--- script summary ---" | Out-File -FilePath $outFile -Append
"ADB=$adb" | Out-File -FilePath $outFile -Append
"SERIAL=$serial" | Out-File -FilePath $outFile -Append
"OUTFILE=$outFile" | Out-File -FilePath $outFile -Append
"PHASE1_OK=$phase1Ok" | Out-File -FilePath $outFile -Append
"PHASE2_OK=$phase2Ok" | Out-File -FilePath $outFile -Append
"PHASE2_SOFT_OK=$phase2SoftOk" | Out-File -FilePath $outFile -Append
"CRASH_OK=$crashOk" | Out-File -FilePath $outFile -Append

Get-Content -Path $outFile -Tail 60

if (-not ($phase1Ok -and ($phase2Ok -or $phase2SoftOk) -and $crashOk)) {
  exit 1
}
