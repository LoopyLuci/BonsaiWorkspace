$ErrorActionPreference = "Stop"

$RootDir = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$JustfilePath = Join-Path $RootDir "scripts/devkit/justfile"

if (-not (Get-Command just -ErrorAction SilentlyContinue)) {
    Write-Error "DevKit launcher requires 'just'. Install with: cargo install just"
    exit 1
}

& just --justfile $JustfilePath @args
exit $LASTEXITCODE
