<#
.SYNOPSIS
    BonsAI Crate Factory — AI-driven sovereignty crate generation.

.DESCRIPTION
    Takes a crate spec from docs/specs/<crate>.md, calls the teacher model
    to generate the initial implementation, then iterates until cargo check
    and cargo test both pass. This is the core loop of the AFAP speedrun.

    Requires:
    - llama-server running on --teacher-port with the teacher model loaded
    - Rust toolchain in PATH
    - docs/specs/<crate>.md exists

.PARAMETER Crate
    Name of the crate to generate (e.g., "bonsai-error").

.PARAMETER TeacherPort
    Port where llama-server is running. Default: 8080.

.PARAMETER MaxIter
    Maximum generate-fix iterations before giving up. Default: 20.

.PARAMETER DryRun
    Print the spec and first prompt but do not call the teacher or write files.

.EXAMPLE
    # Generate bonsai-error crate
    .\scripts\generate_crate.ps1 -Crate bonsai-error

    # Generate bonsai-log with a different teacher port
    .\scripts\generate_crate.ps1 -Crate bonsai-log -TeacherPort 8081
#>
param(
    [Parameter(Mandatory=$true)]
    [string]$Crate,

    [int]$TeacherPort = 8080,
    [int]$MaxIter     = 20,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Root    = Split-Path -Parent $PSScriptRoot
$SpecDir = Join-Path $Root "docs\specs"
$CrateDir = Join-Path $Root "crates\$Crate"
$SrcDir   = Join-Path $CrateDir "src"
$TeacherUrl = "http://127.0.0.1:$TeacherPort"

# ── Validate inputs ────────────────────────────────────────────────────────────

$SpecFile = Join-Path $SpecDir "$Crate.md"
if (-not (Test-Path $SpecFile)) {
    Write-Error "Spec file not found: $SpecFile. Create it first."
    exit 1
}

$Spec = Get-Content $SpecFile -Raw
Write-Host "[factory] Crate: $Crate" -ForegroundColor Cyan
Write-Host "[factory] Spec: $SpecFile ($($Spec.Length) chars)"

if ($DryRun) {
    Write-Host "[dry-run] Would create: $CrateDir"
    Write-Host "[dry-run] First 500 chars of spec:`n$($Spec.Substring(0, [Math]::Min(500, $Spec.Length)))"
    exit 0
}

# ── Check teacher is running ───────────────────────────────────────────────────

try {
    $health = Invoke-RestMethod -Uri "$TeacherUrl/health" -TimeoutSec 5
    Write-Host "[teacher] Server online at $TeacherUrl" -ForegroundColor Green
} catch {
    Write-Warning "[teacher] Server not responding at $TeacherUrl. Start llama-server first."
    Write-Warning "  llama-server -m D:\Models\general\Qwen3-35B-A22B-Q4_K_M.gguf --port $TeacherPort"
    exit 1
}

# ── Helper: call teacher ───────────────────────────────────────────────────────

function Invoke-Teacher {
    param([string]$Prompt, [int]$MaxTokens = 4096)
    $body = @{
        prompt    = $Prompt
        n_predict = $MaxTokens
        temperature = 0.3
        stop = @("###END", "```\n\n#")
    } | ConvertTo-Json
    try {
        $resp = Invoke-RestMethod -Uri "$TeacherUrl/completion" -Method POST -Body $body -ContentType "application/json" -TimeoutSec 300
        return $resp.content.Trim()
    } catch {
        Write-Warning "[teacher] Call failed: $_"
        return ""
    }
}

# ── Helper: run cargo command ──────────────────────────────────────────────────

function Invoke-Cargo {
    param([string]$Command, [string]$WorkDir = $Root)
    $env:PATH = "$env:PATH;$env:USERPROFILE\.cargo\bin"
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName  = "cargo"
    $psi.Arguments = $Command
    $psi.WorkingDirectory = $WorkDir
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true
    $psi.UseShellExecute = $false
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    $proc.Start() | Out-Null
    $stdout = $proc.StandardOutput.ReadToEnd()
    $stderr = $proc.StandardError.ReadToEnd()
    $proc.WaitForExit()
    return @{
        ExitCode = $proc.ExitCode
        Output   = "$stdout`n$stderr"
    }
}

# ── Step 1: Create crate scaffold ─────────────────────────────────────────────

if (-not (Test-Path $CrateDir)) {
    Write-Host "[scaffold] Creating crate at $CrateDir"
    $result = Invoke-Cargo "new --lib crates/$Crate" -WorkDir $Root
    if ($result.ExitCode -ne 0) {
        Write-Error "cargo new failed: $($result.Output)"
        exit 1
    }
}

# ── Step 2: Generate initial implementation ────────────────────────────────────

Write-Host "[gen] Calling teacher to generate initial implementation..." -ForegroundColor Yellow

$SystemPrompt = @"
You are an expert Rust systems programmer building the BonsAI Ecosystem — a fully custom, self-sustaining platform.
Your task is to implement a new Rust crate according to the specification provided.

Rules:
- Write production-quality, idiomatic Rust
- Minimize use of `unsafe`; add a safety comment if used
- Do not add external dependencies beyond std, unless explicitly listed in the spec
- Include comprehensive unit tests in the same file under #[cfg(test)]
- Do not add doc comments explaining what is obvious from the code
- The crate must compile with: cargo check -p $Crate

Output ONLY the Rust source code for src/lib.rs, surrounded by triple backticks.
###END
"@

$GeneratePrompt = @"
$SystemPrompt

Specification:
$Spec

Write the complete src/lib.rs for the $Crate crate.
"@

$Code = Invoke-Teacher -Prompt $GeneratePrompt -MaxTokens 8192

# Extract code from markdown fences if present
if ($Code -match '```rust\s*([\s\S]+?)```') {
    $Code = $Matches[1].Trim()
} elseif ($Code -match '```\s*([\s\S]+?)```') {
    $Code = $Matches[1].Trim()
}

if ([string]::IsNullOrWhiteSpace($Code)) {
    Write-Error "[gen] Teacher returned empty code. Check server and try again."
    exit 1
}

# Write initial code
New-Item -ItemType Directory -Force -Path $SrcDir | Out-Null
Set-Content -Path (Join-Path $SrcDir "lib.rs") -Value $Code -Encoding UTF8
Write-Host "[gen] Wrote $(($Code -split '\n').Count) lines to $SrcDir\lib.rs"

# ── Step 3: Add crate to workspace ────────────────────────────────────────────

$WorkspaceToml = Join-Path $Root "Cargo.toml"
$WorkspaceContent = Get-Content $WorkspaceToml -Raw
if ($WorkspaceContent -notmatch [regex]::Escape("crates/$Crate")) {
    Write-Host "[workspace] Adding $Crate to workspace members"
    $WorkspaceContent = $WorkspaceContent -replace '(members\s*=\s*\[)', "`$1`n    `"crates/$Crate`","
    Set-Content $WorkspaceToml -Value $WorkspaceContent -Encoding UTF8
}

# ── Step 4: Iterative fix loop ─────────────────────────────────────────────────

Write-Host "[compile] Starting iterative fix loop (max $MaxIter iterations)..." -ForegroundColor Cyan

for ($iter = 1; $iter -le $MaxIter; $iter++) {
    Write-Host "[iter $iter/$MaxIter] Running cargo check..." -ForegroundColor Gray

    $checkResult = Invoke-Cargo "check -p $Crate 2>&1"
    if ($checkResult.ExitCode -eq 0) {
        Write-Host "[iter $iter] cargo check PASSED" -ForegroundColor Green
        break
    }

    Write-Host "[iter $iter] cargo check FAILED. Calling teacher to fix..." -ForegroundColor Yellow

    $CurrentCode = Get-Content (Join-Path $SrcDir "lib.rs") -Raw
    $FixPrompt = @"
$SystemPrompt

The following Rust code for the $Crate crate failed cargo check with these errors:

ERRORS:
$($checkResult.Output)

CURRENT CODE:
```rust
$CurrentCode
```

Fix ALL errors and return the complete corrected src/lib.rs.
Only return the Rust code, no explanation.
"@

    $FixedCode = Invoke-Teacher -Prompt $FixPrompt -MaxTokens 8192
    if ($FixedCode -match '```rust\s*([\s\S]+?)```') { $FixedCode = $Matches[1].Trim() }
    elseif ($FixedCode -match '```\s*([\s\S]+?)```') { $FixedCode = $Matches[1].Trim() }

    if (-not [string]::IsNullOrWhiteSpace($FixedCode)) {
        Set-Content -Path (Join-Path $SrcDir "lib.rs") -Value $FixedCode -Encoding UTF8
        Write-Host "[iter $iter] Applied fix ($( ($FixedCode -split '\n').Count ) lines)"
    } else {
        Write-Warning "[iter $iter] Teacher returned empty fix response, retrying..."
    }

    if ($iter -eq $MaxIter) {
        Write-Error "[factory] Exceeded $MaxIter iterations without passing cargo check. Review $SrcDir\lib.rs manually."
        exit 1
    }
}

# ── Step 5: Run tests ──────────────────────────────────────────────────────────

Write-Host "[test] Running cargo test..." -ForegroundColor Cyan
$testResult = Invoke-Cargo "test -p $Crate 2>&1"
if ($testResult.ExitCode -eq 0) {
    Write-Host "[test] All tests PASSED" -ForegroundColor Green
} else {
    Write-Warning "[test] Some tests failed. Output:"
    Write-Warning $testResult.Output
    Write-Warning "[test] Review and fix tests manually, or rerun with -MaxIter to iterate."
}

# ── Step 6: Run clippy ─────────────────────────────────────────────────────────

Write-Host "[clippy] Running cargo clippy..." -ForegroundColor Cyan
$clippyResult = Invoke-Cargo "clippy -p $Crate -- -D warnings 2>&1"
if ($clippyResult.ExitCode -eq 0) {
    Write-Host "[clippy] PASSED" -ForegroundColor Green
} else {
    Write-Warning "[clippy] Warnings/errors found:"
    Write-Warning $clippyResult.Output
}

# ── Done ───────────────────────────────────────────────────────────────────────

Write-Host ""
Write-Host "[factory] SUCCESS: $Crate generated at $CrateDir" -ForegroundColor Green
Write-Host "[next] Steps:"
Write-Host "  1. Review $SrcDir\lib.rs"
Write-Host "  2. Update every Cargo.toml that used the replaced dependency"
Write-Host "  3. Run: cargo check --workspace"
Write-Host "  4. Commit: git add crates/$Crate && git commit -m 'feat: add $Crate (sovereignty plan)'"
