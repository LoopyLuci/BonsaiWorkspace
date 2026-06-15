param(
    [switch]$CI
)

function Check-Command($name) {
    $null -ne (Get-Command $name -ErrorAction SilentlyContinue)
}

Write-Host "[clojurewasm] Checking prerequisites..."
$missing = @()
foreach ($c in @('git','java','rustup','cargo')) {
    if (-not (Check-Command $c)) { $missing += $c }
}
if ($missing.Count -gt 0) {
    Write-Error "Missing prerequisites: $($missing -join ', ')"
    exit 1
}

Write-Host "Ensuring wasm32-wasi target is available..."
$targets = & rustup target list --installed 2>$null
if ($targets -notmatch 'wasm32-wasi') {
    if ($CI) {
        Write-Host "CI mode: adding wasm32-wasi target"
        rustup target add wasm32-wasi
    } else {
        Write-Host "Please add the target: rustup target add wasm32-wasi"
    }
}

$buildDir = Join-Path $PSScriptRoot '..' 'build' 'clojurewasm' | Resolve-Path -ErrorAction SilentlyContinue
if (-not $buildDir) { $buildDir = Join-Path $PSScriptRoot '..' 'build' 'clojurewasm' }
$repo = 'https://github.com/clojurewasm/ClojureWasm.git'

if (Test-Path (Join-Path $buildDir '.git')) {
    Write-Host "Updating existing clone in $buildDir"
    Push-Location $buildDir; git fetch --all --prune; git pull --ff-only; Pop-Location
} else {
    Write-Host "Cloning $repo into $buildDir"
    git clone --depth 1 $repo $buildDir
}

if ($CI) {
    Write-Host "CI mode: clone successful; skipping build"
    exit 0
}

Write-Host "Looking for repository build helpers..."
if (Test-Path (Join-Path $buildDir 'build.sh')) {
    Write-Host "Found build.sh; running it"
    Push-Location $buildDir; & bash ./build.sh; Pop-Location
} elseif (Test-Path (Join-Path $buildDir 'Makefile')) {
    Write-Host "Found Makefile; running make"
    Push-Location $buildDir; & make; Pop-Location
} elseif (Test-Path (Join-Path $buildDir 'gradlew')) {
    Write-Host "Found gradlew; running ./gradlew assemble"
    Push-Location $buildDir; & .\gradlew assemble; Pop-Location
} else {
    Write-Host "No obvious build entrypoint found. Inspect the cloned repo and follow README for build steps."
}

Write-Host "Done."
