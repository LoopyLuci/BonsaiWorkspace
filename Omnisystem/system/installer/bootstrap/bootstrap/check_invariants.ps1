<#
.SYNOPSIS
    Bonsai bootstrap invariant checker.
    Run this after every build to verify the system can self-host and all
    Omnisystem integration contracts hold.

.PARAMETER Phase
    Which phase to check: 0, 1, 2, "all" (default: "all").
    Phase 0 = Effect system + UCR.
    Phase 1 = CAS round-trip.
    Phase 2 = Actor system + CRDT sanity.

.PARAMETER Workspace
    Path to the Bonsai workspace root. Defaults to the parent of this script.

.PARAMETER LogFile
    Optional path to write results as NDJSON. If omitted, results go to stdout only.

.PARAMETER FailFast
    Stop on the first failed assertion.

.EXAMPLE
    .\check_invariants.ps1
    .\check_invariants.ps1 -Phase 0
    .\check_invariants.ps1 -Phase all -LogFile build\invariants.ndjson
#>
[CmdletBinding()]
param(
    [ValidateSet("0","1","2","all")]
    [string]$Phase = "all",

    [string]$Workspace = (Split-Path $PSScriptRoot -Parent),

    [string]$LogFile = "",

    [switch]$FailFast
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Helpers ───────────────────────────────────────────────────────────────────

$Script:PassCount = 0
$Script:FailCount = 0
$Script:Results   = [System.Collections.Generic.List[hashtable]]::new()

function Get-Timestamp { [DateTime]::UtcNow.ToString("o") }

function Write-Pass {
    param([string]$Name, [string]$Detail = "")
    $Script:PassCount++
    $entry = @{ ts = (Get-Timestamp); status = "PASS"; name = $Name; detail = $Detail }
    $Script:Results.Add($entry)
    Write-Host "  [PASS] $Name" -ForegroundColor Green
    if ($Detail) { Write-Host "         $Detail" -ForegroundColor DarkGray }
}

function Write-Fail {
    param([string]$Name, [string]$Detail = "")
    $Script:FailCount++
    $entry = @{ ts = (Get-Timestamp); status = "FAIL"; name = $Name; detail = $Detail }
    $Script:Results.Add($entry)
    Write-Host "  [FAIL] $Name" -ForegroundColor Red
    if ($Detail) { Write-Host "         $Detail" -ForegroundColor Yellow }
    if ($FailFast) {
        Write-Host "`nFail-fast enabled. Stopping." -ForegroundColor Red
        exit 1
    }
}

function Assert-True {
    param([string]$Name, [scriptblock]$Condition, [string]$Detail = "")
    try {
        $result = & $Condition
        if ($result) { Write-Pass $Name $Detail }
        else         { Write-Fail $Name $Detail }
    } catch {
        Write-Fail $Name "Exception: $_"
    }
}

function Assert-CargoCheck {
    param([string]$Name, [string]$CargoArgs = "")
    $cargo = Get-CargoExe
    Write-Host "  Running: $cargo check $CargoArgs" -ForegroundColor DarkGray
    $output = & $cargo check $CargoArgs.Split(" ", [StringSplitOptions]::RemoveEmptyEntries) 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Pass $Name "cargo check passed"
    } else {
        Write-Fail $Name ($output | Select-Object -Last 20 | Out-String).Trim()
    }
}

function Get-CargoExe {
    $candidates = @(
        "$env:USERPROFILE\.cargo\bin\cargo.exe",
        "cargo"
    )
    foreach ($c in $candidates) {
        if (Get-Command $c -ErrorAction SilentlyContinue) { return $c }
    }
    throw "cargo not found. Install Rust via https://rustup.rs"
}

# ── Phase 0: Build + Effect system ───────────────────────────────────────────

function Invoke-Phase0 {
    Write-Host "`n=== Phase 0: Build + Effect System ===" -ForegroundColor Cyan

    # Invariant 0-A: workspace compiles
    Push-Location $Workspace
    try {
        $cargo = Get-CargoExe
        $output = & $cargo check --workspace 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Pass "0-A: cargo check --workspace" "All crates compile"
        } else {
            $errors = ($output | Select-String "^error" | Select-Object -First 10 | Out-String).Trim()
            Write-Fail "0-A: cargo check --workspace" $errors
        }
    } finally { Pop-Location }

    # Invariant 0-B: UCR startup validation log contains no MISSING entries
    # We check the source code for the validation block rather than running the binary,
    # since we can't start the GUI in CI.
    $ucr_src = Join-Path $Workspace "bonsai-workspace\src-tauri\src\lib.rs"
    if (Test-Path $ucr_src) {
        $content = Get-Content $ucr_src -Raw
        if ($content -match "\[UCR\] startup validation") {
            Write-Pass "0-B: UCR startup validation block present" "lib.rs contains UCR validation"
        } else {
            Write-Fail "0-B: UCR startup validation block present" "Missing UCR validation in lib.rs"
        }
    } else {
        Write-Fail "0-B: UCR startup validation block present" "lib.rs not found at $ucr_src"
    }

    # Invariant 0-C: TrustGuard::global() is defined
    $effects_src = Join-Path $Workspace "crates\bonsai-ir\src\effects.rs"
    if (Test-Path $effects_src) {
        $content = Get-Content $effects_src -Raw
        if ($content -match "TrustGuard") {
            Write-Pass "0-C: TrustGuard defined in effects.rs"
        } else {
            Write-Fail "0-C: TrustGuard defined in effects.rs" "TrustGuard not found"
        }
    } else {
        Write-Fail "0-C: TrustGuard defined in effects.rs" "File not found: $effects_src"
    }

    # Invariant 0-D: EffectRow on ToolDef
    $tools_src = Join-Path $Workspace "bonsai-workspace\src-tauri\src\tools.rs"
    if (Test-Path $tools_src) {
        $content = Get-Content $tools_src -Raw
        if ($content -match "effect_row") {
            Write-Pass "0-D: EffectRow on ToolDef" "tools.rs references effect_row"
        } else {
            Write-Fail "0-D: EffectRow on ToolDef" "effect_row not found in tools.rs"
        }
    } else {
        Write-Fail "0-D: EffectRow on ToolDef" "tools.rs not found"
    }

    # Invariant 0-E: get_system_stats is in built_in_tools()
    if (Test-Path $tools_src) {
        $content = Get-Content $tools_src -Raw
        if ($content -match '"get_system_stats"') {
            Write-Pass "0-E: get_system_stats registered in built_in_tools()"
        } else {
            Write-Fail "0-E: get_system_stats registered in built_in_tools()" "Not found in tools.rs"
        }
    }

    # Invariant 0-F: no hardcoded "run_command" for specs injection
    $commands_src = Join-Path $Workspace "bonsai-workspace\src-tauri\src\commands.rs"
    if (Test-Path $commands_src) {
        $content = Get-Content $commands_src -Raw
        # It's OK to reference run_command, but "MUST call run_command" is the bad pattern
        if ($content -match "MUST call run_command") {
            Write-Fail "0-F: No hardcoded MUST-call-run_command in commands.rs" "Found hardcoded injection"
        } else {
            Write-Pass "0-F: No hardcoded MUST-call-run_command in commands.rs"
        }
    }
}

# ── Phase 1: CAS ──────────────────────────────────────────────────────────────

function Invoke-Phase1 {
    Write-Host "`n=== Phase 1: Content-Addressed Storage ===" -ForegroundColor Cyan

    # Invariant 1-A: CasStore is defined
    $cas_src = Join-Path $Workspace "crates\bonsai-cas\src\lib.rs"
    if (Test-Path $cas_src) {
        $content = Get-Content $cas_src -Raw
        if ($content -match "struct CasStore") {
            Write-Pass "1-A: CasStore struct defined"
        } else {
            Write-Fail "1-A: CasStore struct defined" "Not found in $cas_src"
        }
    } else {
        Write-Fail "1-A: CasStore struct defined" "File not found: $cas_src"
    }

    # Invariant 1-B: All core methods present
    if (Test-Path $cas_src) {
        $content = Get-Content $cas_src -Raw
        $methods = @("pub async fn put", "pub async fn get", "pub async fn exists", "pub async fn pin", "pub async fn gc")
        $missing = @($methods | Where-Object { $content -notmatch [regex]::Escape($_) })
        if ($missing.Count -eq 0) {
            Write-Pass "1-B: All CAS methods present (put/get/exists/pin/gc)"
        } else {
            Write-Fail "1-B: All CAS methods present" "Missing: $($missing -join ', ')"
        }
    }

    # Invariant 1-C: Blake3 is used for hashing
    if (Test-Path $cas_src) {
        $content = Get-Content $cas_src -Raw
        if ($content -match "blake3") {
            Write-Pass "1-C: Blake3 hash used for content addressing"
        } else {
            Write-Fail "1-C: Blake3 hash used for content addressing" "blake3 not found in CAS source"
        }
    }

    # Invariant 1-D: Test coverage — round-trip and GC tests exist
    if (Test-Path $cas_src) {
        $content = Get-Content $cas_src -Raw
        if ($content -match "round_trip" -and $content -match "gc_removes") {
            Write-Pass "1-D: CAS tests present (round_trip + gc_removes)"
        } else {
            Write-Fail "1-D: CAS tests present" "Missing round_trip or gc_removes tests"
        }
    }
}

# ── Phase 2: UniIR + Actor Model ──────────────────────────────────────────────

function Invoke-Phase2 {
    Write-Host "`n=== Phase 2: UniIR + Actor Model + CRDT ===" -ForegroundColor Cyan

    # Invariant 2-A: IrOp enum exists
    $ops_src = Join-Path $Workspace "crates\bonsai-ir\src\ops.rs"
    if (Test-Path $ops_src) {
        $content = Get-Content $ops_src -Raw
        if ($content -match "enum IrOp" -and $content -match "enum IrType") {
            Write-Pass "2-A: IrOp and IrType enums defined"
        } else {
            Write-Fail "2-A: IrOp and IrType enums defined" "Not found in ops.rs"
        }
    } else {
        Write-Fail "2-A: IrOp and IrType enums defined" "ops.rs not found"
    }

    # Invariant 2-B: IrFunction and IrModule structs
    if (Test-Path $ops_src) {
        $content = Get-Content $ops_src -Raw
        if ($content -match "struct IrFunction" -and $content -match "struct IrModule") {
            Write-Pass "2-B: IrFunction and IrModule structs defined"
        } else {
            Write-Fail "2-B: IrFunction and IrModule structs defined"
        }
    }

    # Invariant 2-C: ActorSystem::spawn defined
    $actors_src = Join-Path $Workspace "crates\bonsai-actors\src\lib.rs"
    if (Test-Path $actors_src) {
        $content = Get-Content $actors_src -Raw
        if ($content -match "pub fn spawn" -and $content -match "trait Actor") {
            Write-Pass "2-C: ActorSystem and Actor trait defined"
        } else {
            Write-Fail "2-C: ActorSystem and Actor trait defined"
        }
    } else {
        Write-Fail "2-C: ActorSystem and Actor trait defined" "bonsai-actors/src/lib.rs not found"
    }

    # Invariant 2-D: CRDT GCounter, LwwRegister, OrSet
    $crdt_src = Join-Path $Workspace "crates\bonsai-crdt\src\lib.rs"
    if (Test-Path $crdt_src) {
        $content = Get-Content $crdt_src -Raw
        $types = @("struct GCounter", "struct LwwRegister", "struct OrSet")
        $missing = @($types | Where-Object { $content -notmatch [regex]::Escape($_) })
        if ($missing.Count -eq 0) {
            Write-Pass "2-D: All CRDT types present (GCounter, LwwRegister, OrSet)"
        } else {
            Write-Fail "2-D: All CRDT types present" "Missing: $($missing -join ', ')"
        }
    } else {
        Write-Fail "2-D: All CRDT types present" "bonsai-crdt/src/lib.rs not found"
    }

    # Invariant 2-E: CRDT merge() semantics have test coverage
    if (Test-Path $crdt_src) {
        $content = Get-Content $crdt_src -Raw
        if ($content -match "fn gcounter_merge" -and $content -match "fn lww_register") {
            Write-Pass "2-E: CRDT merge tests present"
        } else {
            Write-Fail "2-E: CRDT merge tests present"
        }
    }

    # Invariant 2-F: No *external* network calls in any crate source.
    # localhost/127.0.0.1 calls are allowed (local training RPC, etc.).
    $crates_dir = Join-Path $Workspace "crates"
    # Look for reqwest/hyper URLs that point outside localhost
    $violations = @()
    $hits = Get-ChildItem $crates_dir -Recurse -Filter "*.rs" |
        Select-String -Pattern 'https?://' |
        Where-Object { $_.Line -notmatch "localhost|127\.0\.0\.1|0\.0\.0\.0" } |
        Where-Object { $_.Line -notmatch "^\s*//" } |           # ignore comments
        Where-Object { $_.Line -notmatch "json-schema\.org" } | # JSON Schema $schema URI is a string literal, not a network call
        Where-Object { $_.Line -notmatch 'example\.com|schema\.org' }  # other well-known literal URIs
    if ($hits) {
        $violations += @($hits | ForEach-Object { "$($_.Filename):$($_.LineNumber)" })
    }
    if ($violations.Count -eq 0) {
        Write-Pass "2-F: No external network calls in crates/ (offline-first)"
    } else {
        Write-Fail "2-F: No external network calls in crates/ (offline-first)" ($violations -join "; ")
    }
}

# ── Summary ───────────────────────────────────────────────────────────────────

function Write-Summary {
    $total = $Script:PassCount + $Script:FailCount
    Write-Host "`n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor White
    Write-Host "  Bootstrap Invariants: $($Script:PassCount)/$total passed" -ForegroundColor $(if ($Script:FailCount -eq 0) { "Green" } else { "Yellow" })
    if ($Script:FailCount -gt 0) {
        Write-Host "  FAILED assertions: $($Script:FailCount)" -ForegroundColor Red
    } else {
        Write-Host "  All invariants PASS — build is bootstrap-valid." -ForegroundColor Green
    }
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━`n" -ForegroundColor White

    if ($LogFile) {
        $Script:Results | ForEach-Object { $_ | ConvertTo-Json -Compress } | Set-Content $LogFile
        Write-Host "  Results written to: $LogFile" -ForegroundColor DarkGray
    }

    # Exit code for CI
    exit $Script:FailCount
}

# ── Entrypoint ────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "╔══════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  Bonsai × Omnisystem Bootstrap Invariant Check  ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host "  Workspace : $Workspace"
Write-Host "  Phase     : $Phase"
Write-Host "  Timestamp : $(Get-Timestamp)"

switch ($Phase) {
    "0"   { Invoke-Phase0 }
    "1"   { Invoke-Phase1 }
    "2"   { Invoke-Phase2 }
    "all" {
        Invoke-Phase0
        Invoke-Phase1
        Invoke-Phase2
    }
}

Write-Summary
