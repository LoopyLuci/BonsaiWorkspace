param(
  [string]$RepoRoot = "",
  [int]$WorkspacePort = 11369,
  [int]$BuddyPort = 11420,
  [int]$BotAdminPort = 11666,
  [int]$TimeoutSeconds = 120,
  [string]$SummaryPath = ""
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
  $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\.." )).Path
} else {
  $RepoRoot = (Resolve-Path $RepoRoot).Path
}

$runnerDir = Join-Path $RepoRoot "tests\ecosystem-integration"
$logsDir = Join-Path $runnerDir "logs"
New-Item -ItemType Directory -Path $logsDir -Force | Out-Null

if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
  $SummaryPath = Join-Path $runnerDir "summary-latest.json"
}

$results = New-Object System.Collections.Generic.List[object]
$startedAt = Get-Date

$workspaceProc = $null
$botProc = $null
$detectedBotPort = $BotAdminPort

function Invoke-HttpJson {
  param(
    [Parameter(Mandatory = $true)][string]$Uri,
    [string]$Method = "GET",
    [object]$Body = $null,
    [hashtable]$Headers = @{},
    [int]$TimeoutSec = 20
  )

  $args = @{
    Uri = $Uri
    Method = $Method
    Headers = $Headers
    TimeoutSec = $TimeoutSec
    ErrorAction = 'Stop'
  }

  if ($null -ne $Body) {
    $args['Body'] = ($Body | ConvertTo-Json -Depth 12)
    $args['ContentType'] = 'application/json'
  }

  try {
    $resp = Invoke-WebRequest @args
    $parsed = $null
    try { $parsed = $resp.Content | ConvertFrom-Json -Depth 12 } catch { $parsed = $resp.Content }
    return [pscustomobject]@{
      Ok = $true
      StatusCode = [int]$resp.StatusCode
      Content = $parsed
      Raw = $resp.Content
      Error = $null
    }
  } catch {
    $status = $null
    $raw = ""
    try {
      if ($_.Exception.Response) {
        $status = [int]$_.Exception.Response.StatusCode
        $stream = $_.Exception.Response.GetResponseStream()
        if ($stream) {
          $reader = New-Object System.IO.StreamReader($stream)
          $raw = $reader.ReadToEnd()
        }
      }
    } catch {}

    return [pscustomobject]@{
      Ok = $false
      StatusCode = $status
      Content = $null
      Raw = $raw
      Error = $_.Exception.Message
    }
  }
}

function Wait-Healthy {
  param(
    [Parameter(Mandatory = $true)][string]$Uri,
    [int]$TimeoutSec = 60
  )

  $deadline = (Get-Date).AddSeconds($TimeoutSec)
  while ((Get-Date) -lt $deadline) {
    $probe = Invoke-HttpJson -Uri $Uri -Method GET -TimeoutSec 3
    if ($probe.Ok -and $probe.StatusCode -eq 200) {
      return $true
    }
    Start-Sleep -Milliseconds 500
  }
  return $false
}

function Run-Test {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Body
  )

  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $pass = $false
  $notes = ""
  $details = $null

  try {
    $outcome = & $Body
    if ($outcome -is [hashtable] -or $outcome -is [pscustomobject]) {
      $pass = [bool]$outcome.Pass
      $notes = [string]$outcome.Notes
      $details = $outcome.Details
    } elseif ($outcome -is [bool]) {
      $pass = $outcome
    } else {
      $pass = $true
      $details = $outcome
    }
  } catch {
    $pass = $false
    $notes = $_.Exception.Message
  } finally {
    $sw.Stop()
  }

  $row = [pscustomobject]@{
    name = $Name
    pass = $pass
    duration_ms = [int]$sw.ElapsedMilliseconds
    notes = $notes
    details = $details
  }
  $results.Add($row)

  $status = if ($pass) { "PASS" } else { "FAIL" }
  Write-Host ("[{0}] {1} ({2}ms)" -f $status, $Name, $row.duration_ms)
  if (-not [string]::IsNullOrWhiteSpace($notes)) {
    Write-Host ("  -> {0}" -f $notes)
  }
}

function Start-BackendIfNeeded {
  param(
    [string]$Name,
    [string]$HealthUrl,
    [string]$FilePath,
    [string[]]$ArgumentList,
    [string]$WorkingDirectory
  )

  if (Wait-Healthy -Uri $HealthUrl -TimeoutSec 2) {
    return [pscustomobject]@{ Spawned = $false; Process = $null }
  }

  $stdout = Join-Path $logsDir ("{0}-stdout.log" -f $Name)
  $stderr = Join-Path $logsDir ("{0}-stderr.log" -f $Name)

  $proc = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -WorkingDirectory $WorkingDirectory -PassThru -RedirectStandardOutput $stdout -RedirectStandardError $stderr

  return [pscustomobject]@{ Spawned = $true; Process = $proc }
}

try {
  Write-Host "[runner] Repo root: $RepoRoot"
  Write-Host "[runner] Starting ecosystem integration checks..."

  $workspaceHealth = "http://127.0.0.1:$WorkspacePort/health"
  $buddyHealth = "http://127.0.0.1:$BuddyPort/health"

  $workspaceBoot = Start-BackendIfNeeded -Name "workspace" -HealthUrl $workspaceHealth -FilePath "cargo" -ArgumentList @("run", "--manifest-path", "bonsai-workspace/src-tauri/Cargo.toml") -WorkingDirectory $RepoRoot
  $workspaceProc = $workspaceBoot.Process

  $botBoot = Start-BackendIfNeeded -Name "bot" -HealthUrl "http://127.0.0.1:$BotAdminPort/health" -FilePath "cargo" -ArgumentList @("run", "--manifest-path", "bonsai-bot/Cargo.toml") -WorkingDirectory $RepoRoot
  $botProc = $botBoot.Process

  if (-not (Wait-Healthy -Uri $workspaceHealth -TimeoutSec $TimeoutSeconds)) {
    throw "Workspace API did not become healthy at $workspaceHealth"
  }

  if (-not (Wait-Healthy -Uri $buddyHealth -TimeoutSec $TimeoutSeconds)) {
    throw "Buddy API did not become healthy at $buddyHealth"
  }

  $botCandidates = @($BotAdminPort, 11421, 11666) | Select-Object -Unique
  foreach ($p in $botCandidates) {
    if (Wait-Healthy -Uri "http://127.0.0.1:$p/health" -TimeoutSec 5) {
      $detectedBotPort = $p
      break
    }
  }

  Run-Test -Name "Test 1: Workspace API health" -Body {
    $resp = Invoke-HttpJson -Uri $workspaceHealth
    @{ Pass = ($resp.Ok -and $resp.StatusCode -eq 200); Notes = $resp.Error; Details = $resp.Content }
  }

  Run-Test -Name "Test 2: Buddy API chat completions" -Body {
    $uri = "http://127.0.0.1:$BuddyPort/v1/chat/completions"
    $body = @{
      model = "bonsai-local"
      messages = @(@{ role = "user"; content = "Respond with: integration-ok" })
      stream = $false
    }
    $resp = Invoke-HttpJson -Uri $uri -Method POST -Body $body
    @{ Pass = ($resp.Ok -and $resp.StatusCode -ge 200 -and $resp.StatusCode -lt 300); Notes = $resp.Error; Details = $resp.Content }
  }

  Run-Test -Name "Test 3: Bot admin API status" -Body {
    $health = Invoke-HttpJson -Uri "http://127.0.0.1:$detectedBotPort/health"
    $token = $env:BONSAI_BOT_ADMIN_TOKEN
    $statusResp = $null
    if (-not [string]::IsNullOrWhiteSpace($token)) {
      $statusResp = Invoke-HttpJson -Uri "http://127.0.0.1:$detectedBotPort/status" -Headers @{ Authorization = "Bearer $token" }
    }
    $ok = $health.Ok -and $health.StatusCode -eq 200
    if ($statusResp) {
      $ok = $ok -and $statusResp.Ok
    }
    @{ Pass = $ok; Notes = $health.Error; Details = @{ health = $health.Content; status = $statusResp?.Content } }
  }

  Run-Test -Name "Test 4: Model load/unload cycle" -Body {
    $models = Invoke-HttpJson -Uri "http://127.0.0.1:$BuddyPort/v1/models"
    if (-not $models.Ok) {
      return @{ Pass = $false; Notes = "Could not list models"; Details = $models.Raw }
    }

    $firstModel = $null
    try {
      $firstModel = $models.Content.data[0].id
    } catch {}

    if ([string]::IsNullOrWhiteSpace($firstModel)) {
      return @{ Pass = $false; Notes = "No models reported by /v1/models"; Details = $models.Content }
    }

    $loadResp = Invoke-HttpJson -Uri "http://127.0.0.1:$BuddyPort/api/models/load" -Method POST -Body @{ model = $firstModel }
    $unloadResp = Invoke-HttpJson -Uri "http://127.0.0.1:$BuddyPort/api/models/unload" -Method POST -Body @{ model = $firstModel }

    $ok = $loadResp.Ok -and $unloadResp.Ok
    @{ Pass = $ok; Notes = if ($ok) { "" } else { "Load/unload endpoint failed" }; Details = @{ model = $firstModel; load = $loadResp.Raw; unload = $unloadResp.Raw } }
  }

  Run-Test -Name "Test 5: Swarm orchestration" -Body {
    $submitCandidates = @(
      @{ uri = "http://127.0.0.1:$WorkspacePort/v1/swarm/tasks"; body = @{ prompt = "integration swarm task" } },
      @{ uri = "http://127.0.0.1:$WorkspacePort/v1/swarm/submit"; body = @{ task = "integration swarm task" } }
    )

    $submit = $null
    foreach ($candidate in $submitCandidates) {
      $attempt = Invoke-HttpJson -Uri $candidate.uri -Method POST -Body $candidate.body
      if ($attempt.Ok) { $submit = $attempt; break }
    }

    $workers = Invoke-HttpJson -Uri "http://127.0.0.1:$WorkspacePort/v1/swarm/workers"
    $ok = ($null -ne $submit) -and $workers.Ok
    @{ Pass = $ok; Notes = if ($ok) { "" } else { "Swarm endpoints unavailable or failed" }; Details = @{ submit = $submit; workers = $workers.Raw } }
  }

  Run-Test -Name "Test 6: Tool invocation" -Body {
    $toolUri = "http://127.0.0.1:$BuddyPort/api/tools/invoke"
    $a = Invoke-HttpJson -Uri $toolUri -Method POST -Body @{ tool = "get_datetime"; params = @{} }
    $b = Invoke-HttpJson -Uri $toolUri -Method POST -Body @{ tool = "get_system_stats"; params = @{} }
    $ok = $a.Ok -and $b.Ok
    @{ Pass = $ok; Notes = if ($ok) { "" } else { "Tool invocation failed" }; Details = @{ datetime = $a.Raw; system = $b.Raw } }
  }

  Run-Test -Name "Test 7: RAG search" -Body {
    $fixturePath = Join-Path $runnerDir "rag-fixture.txt"
    Set-Content -Path $fixturePath -Value "Bonsai integration fixture: alpha-bravo-charlie" -Encoding UTF8

    $indexCandidates = @(
      @{ uri = "http://127.0.0.1:$WorkspacePort/v1/rag/index"; body = @{ path = $fixturePath } },
      @{ uri = "http://127.0.0.1:$WorkspacePort/api/rag/index"; body = @{ path = $fixturePath } }
    )
    $queryCandidates = @(
      @{ uri = "http://127.0.0.1:$WorkspacePort/v1/rag/query"; body = @{ query = "alpha-bravo-charlie" } },
      @{ uri = "http://127.0.0.1:$WorkspacePort/api/rag/search"; body = @{ query = "alpha-bravo-charlie" } }
    )

    $indexResp = $null
    foreach ($candidate in $indexCandidates) {
      $attempt = Invoke-HttpJson -Uri $candidate.uri -Method POST -Body $candidate.body
      if ($attempt.Ok) { $indexResp = $attempt; break }
    }

    $queryResp = $null
    foreach ($candidate in $queryCandidates) {
      $attempt = Invoke-HttpJson -Uri $candidate.uri -Method POST -Body $candidate.body
      if ($attempt.Ok) { $queryResp = $attempt; break }
    }

    $ok = ($null -ne $indexResp) -and ($null -ne $queryResp)
    @{ Pass = $ok; Notes = if ($ok) { "" } else { "RAG endpoints unavailable or failed" }; Details = @{ index = $indexResp; query = $queryResp } }
  }

  Run-Test -Name "Test 8: Browser extension build" -Body {
    $extDir = Join-Path $RepoRoot "browser-extension"
    Push-Location $extDir
    try {
      & npm run typecheck | Out-Null
      if ($LASTEXITCODE -ne 0) { return @{ Pass = $false; Notes = "typecheck failed" } }

      & npm run build:chrome | Out-Null
      if ($LASTEXITCODE -ne 0) { return @{ Pass = $false; Notes = "build:chrome failed" } }

      & npm run build:firefox | Out-Null
      if ($LASTEXITCODE -ne 0) { return @{ Pass = $false; Notes = "build:firefox failed" } }

      return @{ Pass = $true; Notes = "" }
    } finally {
      Pop-Location
    }
  }

  Run-Test -Name "Test 9: Android Kotlin compile (if JDK available)" -Body {
    $javaCmd = (Get-Command java -ErrorAction SilentlyContinue)
    if ($null -eq $javaCmd) {
      return @{ Pass = $true; Notes = "SKIP: java not found; compile check not executed" }
    }

    $androidDir = Join-Path $RepoRoot "bonsai-buddy-android"
    if (-not (Test-Path (Join-Path $androidDir "gradlew.bat"))) {
      return @{ Pass = $false; Notes = "gradlew.bat missing in bonsai-buddy-android" }
    }

    Push-Location $androidDir
    try {
      & .\gradlew.bat :app:compileDebugKotlin | Out-Null
      if ($LASTEXITCODE -ne 0) {
        return @{ Pass = $false; Notes = "Android compile failed" }
      }
      return @{ Pass = $true; Notes = "" }
    } finally {
      Pop-Location
    }
  }

  Run-Test -Name "Test 10: Launcher recursion guard" -Body {
    $launchDir = Join-Path $RepoRoot "bonsai-workspace\src"
    $launcher = Join-Path $launchDir "launch-all.mjs"
    $pidFile = Join-Path $RepoRoot ".bonsai-launcher.pid"

    if (-not (Test-Path $launcher)) {
      return @{ Pass = $false; Notes = "launch-all.mjs not found" }
    }

    $sentinel = Start-Process -FilePath "pwsh" -ArgumentList @("-NoProfile", "-Command", "Start-Sleep -Seconds 30") -PassThru
    try {
      Set-Content -Path $pidFile -Value $sentinel.Id -Encoding ASCII
      $output = & node $launcher --preflight-only 2>&1
      $ok = ($LASTEXITCODE -eq 0) -and (($output -join "`n") -match "already running")
      return @{ Pass = $ok; Notes = if ($ok) { "" } else { "launcher did not detect existing PID lock" }; Details = $output }
    } finally {
      if (-not $sentinel.HasExited) {
        Stop-Process -Id $sentinel.Id -Force -ErrorAction SilentlyContinue
      }
      Remove-Item -Path $pidFile -Force -ErrorAction SilentlyContinue
    }
  }
}
finally {
  if ($workspaceProc -and -not $workspaceProc.HasExited) {
    Stop-Process -Id $workspaceProc.Id -Force -ErrorAction SilentlyContinue
  }
  if ($botProc -and -not $botProc.HasExited) {
    Stop-Process -Id $botProc.Id -Force -ErrorAction SilentlyContinue
  }

  $finishedAt = Get-Date
  $passCount = ($results | Where-Object { $_.pass }).Count
  $failCount = $results.Count - $passCount

  $summary = [pscustomobject]@{
    started_at = $startedAt.ToString("o")
    finished_at = $finishedAt.ToString("o")
    duration_ms = [int](($finishedAt - $startedAt).TotalMilliseconds)
    repo_root = $RepoRoot
    ports = @{ workspace = $WorkspacePort; buddy = $BuddyPort; bot_admin = $detectedBotPort }
    totals = @{ total = $results.Count; pass = $passCount; fail = $failCount }
    tests = $results
  }

  $summaryJson = $summary | ConvertTo-Json -Depth 12
  Set-Content -Path $SummaryPath -Value $summaryJson -Encoding UTF8
  $timestamped = Join-Path $runnerDir (("summary-{0}.json" -f (Get-Date -Format "yyyyMMdd-HHmmss")))
  Set-Content -Path $timestamped -Value $summaryJson -Encoding UTF8

  Write-Host ""
  Write-Host ("[runner] Total: {0}, Pass: {1}, Fail: {2}" -f $summary.totals.total, $summary.totals.pass, $summary.totals.fail)
  Write-Host "[runner] Summary: $SummaryPath"
  Write-Host "[runner] Snapshot: $timestamped"

  if ($summary.totals.fail -gt 0) {
    exit 1
  }
}
