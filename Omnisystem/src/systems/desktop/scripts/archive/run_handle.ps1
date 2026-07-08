$ErrorActionPreference = 'Stop'
$targetPid = 44036
$tmp = Join-Path $env:TEMP 'Handle.zip'
$dest = Join-Path $env:TEMP 'Handle'
$out = Join-Path $env:TEMP 'handle_output.txt'

Write-Host "IsAdmin: $(([Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"

if (-not (Test-Path $dest)) {
    Write-Host "Downloading Handle.zip to $tmp"
    try {
        Invoke-WebRequest -Uri 'https://download.sysinternals.com/files/Handle.zip' -OutFile $tmp -UseBasicParsing -ErrorAction Stop
        Expand-Archive -Path $tmp -DestinationPath $dest -Force
    } catch {
        Write-Host "Download or extract failed: $($_.Exception.Message)"
        exit 2
    }
} else {
    Write-Host "Handle directory already exists: $dest"
}

$h = Join-Path $dest 'handle.exe'
if (-not (Test-Path $h)) {
    Write-Host "handle.exe not found at $h"
    exit 3
}

Write-Host "Running handle -accepteula -p $targetPid and saving to $out"
try {
    & $h -accepteula -p $targetPid 2>&1 | Tee-Object -FilePath $out
} catch {
    Write-Host "Handle failed: $($_.Exception.Message)"
    exit 4
}

Write-Host "`nNetstat (filtered):"
netstat -abno | Select-String '11369|11420|11421|11666|1420' | Tee-Object -FilePath $out -Append

Write-Host "`nSaved output to $out"
