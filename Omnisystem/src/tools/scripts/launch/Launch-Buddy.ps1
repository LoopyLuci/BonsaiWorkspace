# Launch Bonsai Buddy (AI chat window only)
param([Parameter(ValueFromRemainingArguments)][string[]]$ExtraArgs)
$workspaceRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$exe = Join-Path $workspaceRoot 'bonsai-workspace\src-tauri\target\release\bonsai-workspace.exe'
if (-not (Test-Path $exe)) {
    Write-Host "Executable not found at: $exe" -ForegroundColor Red
    Write-Host "Run 'just build' or '.\scripts\build\BonsaiExeLauncherBuilder.ps1' first." -ForegroundColor Yellow
    exit 1
}
& $exe --mode buddy @ExtraArgs
