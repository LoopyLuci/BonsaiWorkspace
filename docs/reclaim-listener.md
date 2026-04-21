# Reclaim listener helper

This document describes `scripts/reclaim-listener.ps1`, a helper to inspect
and (optionally) reclaim processes that own LISTEN sockets on local ports.

When to use
- You see `netstat` or `Get-NetTCPConnection` entries showing `LISTENING` on
  a local port, but the owning PID does not map to a running process. This
  typically indicates a kernel-level stale socket or a handle owned by a
  process that cannot be easily identified.

Important safety notes
- This script will only forcibly stop processes when you pass `-ForceKill`.
- Running as Administrator is strongly recommended for accurate diagnostics
  and to stop processes.
- If the script reports a PID that cannot be mapped to a process, do not
  attempt to `taskkill` that PID — instead use `TCPView` (Sysinternals) or
  reboot the machine.

Usage

Inspect default ports (no changes):

```powershell
.\scripts\reclaim-listener.ps1
```

Inspect specific ports without killing:

```powershell
.\scripts\reclaim-listener.ps1 -Ports 11666,11369
```

Inspect and forcibly stop found processes (use with caution):

```powershell
.\scripts\reclaim-listener.ps1 -Ports 11666 -ForceKill
```

Use `-UseHandle` to run Sysinternals `handle.exe` (if available) for extra
diagnostics when the owning PID cannot be resolved.

When to reboot or use TCPView
- If the script reports a PID with no matching process, that indicates a
  kernel-level or orphaned handle. In that case, use `TCPView` (GUI) to
  attempt to close the handle or reboot the host to clear the kernel state.

References
- Sysinternals TCPView: https://learn.microsoft.com/sysinternals/downloads/tcpview
- Sysinternals Handle: https://learn.microsoft.com/sysinternals/downloads/handle
