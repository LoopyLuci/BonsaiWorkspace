# Launch full Omnisystem Ecosystem (IDE + Buddy chat window)
param([Parameter(ValueFromRemainingArguments)][string[]]$ExtraArgs)
$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $workspaceRoot 'omnisystem-workspace\src-tauri\target\release\omnisystem-workspace.exe'
if (-not (Test-Path $exe)) {
    Write-Host "Executable not found at: $exe" -ForegroundColor Red
    Write-Host "Run 'just build' or '.\scripts\build\OmnisystemExeLauncherBuilder.ps1' first." -ForegroundColor Yellow
    exit 1
}
& $exe --mode ecosystem @ExtraArgs
