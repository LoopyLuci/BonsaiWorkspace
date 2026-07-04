<#
.SYNOPSIS
  Inspect and optionally reclaim processes owning LISTEN sockets on local ports.

.DESCRIPTION
  Uses `Get-NetTCPConnection` (preferred) or `netstat -abno` as a fallback to
  find processes listening on supplied ports. If a matching process is found,
  prints metadata and (optionally) attempts to stop it. If the owning PID
  cannot be mapped to a process, the script will recommend using TCPView or
  rebooting the host because this indicates a kernel-level/stale listen.

.PARAMETERS
  Ports    - Array of port numbers to inspect. Defaults to common bot ports.
  ForceKill - If provided, attempts to forcibly stop any found process.
  UseHandle - If provided and Sysinternals `handle.exe` is available, runs it
              for additional diagnostics when a PID cannot be mapped.

.EXAMPLE
  # Inspect default ports (no changes):
  .\scripts\reclaim-listener.ps1

  # Inspect ports and attempt to forcibly stop matching processes:
  .\scripts\reclaim-listener.ps1 -Ports 11666 -ForceKill

NOTE: Run as Administrator for full diagnostics and to stop processes.
#>

param(
    [Parameter(Mandatory=$false)]
    [int[]]$Ports = @(11369,11420,11666),

    [switch]$ForceKill,

    [switch]$UseHandle
)

function Get-OwnersByPort {
    param([int]$Port)
    $owners = @()
    try {
        $conns = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction Stop
        foreach ($c in $conns) {
            $owners += [PSCustomObject]@{ Port = $Port; OwningPid = $c.OwningProcess }
        }
    } catch {
        # Fallback to parsing netstat output when Get-NetTCPConnection isn't available
        $lines = (netstat -abno) 2>$null | Select-String -Pattern ":$Port\s" -SimpleMatch
        foreach ($line in $lines) {
            if ($line -match '\s+LISTENING\s+(\d+)$') {
                $pid = [int]$matches[1]
                $owners += [PSCustomObject]@{ Port = $Port; OwningPid = $pid }
            }
        }
    }
    return $owners
}

function Describe-Owner {
    param([PSCustomObject]$Owner)
    $pid = $Owner.OwningPid
    $port = $Owner.Port
    $proc = $null
    try {
        $proc = Get-Process -Id $pid -ErrorAction Stop
        $info = Get-CimInstance Win32_Process -Filter "ProcessId=$pid" -ErrorAction SilentlyContinue
        $exe = if ($info) { $info.ExecutablePath } else { $null }
        Write-Host "Port $port -> PID $pid -> $($proc.ProcessName) $($exe)" -ForegroundColor Cyan
    } catch {
        Write-Host "Port $port -> PID $pid -> (no matching process found)" -ForegroundColor Yellow
        Write-Host "  This looks like a kernel-level/stale LISTEN. Use TCPView (Sysinternals) or reboot." -ForegroundColor Yellow
        if ($UseHandle) {
            $handlePath = (Get-Command handle.exe -ErrorAction SilentlyContinue).Path
            if ($handlePath) {
                Write-Host "Running handle.exe for PID $pid (may require admin)..." -ForegroundColor Gray
                & $handlePath -a -p $pid
            } else {
                Write-Host "handle.exe not found in PATH. Download from Sysinternals and run as admin." -ForegroundColor Gray
            }
        }
        return
    }

    if ($ForceKill) {
        Write-Host "Stopping process $pid ($($proc.ProcessName))..." -ForegroundColor Red
        try {
            Stop-Process -Id $pid -Force -ErrorAction Stop
            Write-Host "Stopped PID $pid" -ForegroundColor Green
        } catch {
            Write-Host "Failed to stop PID $pid: $_" -ForegroundColor Red
        }
    } else {
        if ($proc.ProcessName -match 'bonsai|bot') {
            Write-Host "Detected process appears bot-related; consider stopping it with -ForceKill." -ForegroundColor Magenta
        } else {
            Write-Host "Process appears unrelated. To forcibly stop it, re-run with -ForceKill." -ForegroundColor Gray
        }
    }
}

#-- Execution starts here
if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Warning "This script should be run as Administrator for full diagnostics and to stop processes."
}

foreach ($p in $Ports) {
    Write-Host "Checking port $p..." -ForegroundColor White
    $owners = Get-OwnersByPort -Port $p
    if (-not $owners -or $owners.Count -eq 0) {
        Write-Host "  No LISTEN entries found for port $p." -ForegroundColor DarkGreen
        continue
    }
    foreach ($o in $owners) {
        Describe-Owner -Owner $o
    }
    Write-Host ""  # blank line
}

Write-Host "Done. If you still see kernel-owned LISTEN entries with no matching PID, use TCPView (Sysinternals) or reboot the host." -ForegroundColor White
