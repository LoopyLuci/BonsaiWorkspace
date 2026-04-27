# Self-hosted runner — Offline-first setup

This document shows how to prepare a machine to run the mobile testing smoke tests locally and as a GitHub self-hosted runner without Docker. The goal: everything required to run the tests (adb/ffmpeg/node and the repo scripts) can be installed and used locally on the device.

Summary:
- No Docker required.
- You can run tests locally without any GitHub connectivity (recommended for device labs).
- If you want to register a GitHub self-hosted runner, that step requires an internet connection to GitHub, but the tests themselves do not need Docker or other external runtime services.

Prerequisites
- Node.js 18+ installed and on PATH.
- `adb` (Android platform-tools) on PATH.
- `ffmpeg` on PATH (static build preferred for offline installs).
- A USB-connected Android device with USB debugging enabled.

Quick verification (after installing prerequisites):

```bash
# Linux / macOS
node -v
adb version
ffmpeg -version

# Windows (PowerShell)
node -v
adb version
ffmpeg -version
```

Run the smoke test locally (no GitHub runner involved)

1. Start the server that drives the device capture:

```bash
# from workspace root (Linux/macOS)
cd tools/mobile-testing-automation/android-automation
node server.js

# Windows PowerShell
Set-Location 'tools\\mobile-testing-automation\\android-automation'
node server.js
```

2. In another shell, run the e2e smoke test script:

```bash
node tools/mobile-testing-automation/android-automation/test/e2e-fmp4-smoke.js
```

Or use the convenience wrapper scripts included here:

```bash
# Linux
./tools/mobile-testing-automation/self-hosted-runner/run-smoke.sh

# Windows (PowerShell)
.\\tools\\mobile-testing-automation\\self-hosted-runner\\run-smoke.ps1
```

Offline installation notes

- adb (platform-tools):
  - Download the platform-tools ZIP from the Android developer site on a machine with connectivity and copy the ZIP to the target device. Extract and place the `platform-tools` directory somewhere local, then add it to PATH.
  - Example (manual extract): `C:\tools\platform-tools` or `/opt/platform-tools`.

- ffmpeg:
  - Use a statically linked ffmpeg build for your platform (BtbN builds or similar). Download the archive on an internet-enabled machine and copy the binary to the target host. Place `ffmpeg` on PATH.

- Node.js:
  - Use the official installers or an offline package for your platform. Install Node and ensure `node` is on PATH.

Registering a GitHub self-hosted runner (optional)

- Registering a runner requires connectivity to GitHub. If you prefer a fully offline lab, skip registration and run the server and tests locally as above.
- If you register a runner, set these labels so the workflow picks it up: `self-hosted`, `linux` (or `windows`), and `mobile-testing`.

Example systemd unit (Linux) — copy and adapt

```ini
[Unit]
Description=Bonsai mobile-testing self-hosted runner
After=network.target

[Service]
User=runner
WorkingDirectory=/opt/bonsai/bonsai-workspace
ExecStart=/opt/bonsai/actions-runner/run.sh
Restart=always

[Install]
WantedBy=multi-user.target
```

Windows startup notes

- For Windows, create a Scheduled Task or use NSSM to run the runner/service at user login. The core requirement is that PATH includes `node`, `adb`, and `ffmpeg` and that the working directory is the workspace root.

Security & device access
- The runner and scripts require ADB USB permission; ensure the host user can access connected devices.
- Keep device access controlled and avoid exposing device USB to public runners.

Optional auth token

- The server supports an optional shared-secret token to restrict access to the HTTP API and WebSocket endpoints. You can pass the token as the first argument to `start-server.sh`/`start-server.ps1` or set the `AUTH_TOKEN` environment variable prior to starting the server. The Tauri `testing_toolkit_run` command and the `start-server` convenience scripts will propagate the token to the server process as `AUTH_TOKEN`.

Examples:

```bash
# start server with token (Linux/macOS)
./start-server.sh "my-secret-token"

# start server with token (PowerShell)
.\start-server.ps1 -Token "my-secret-token"
```

Files added here
- `tools/mobile-testing-automation/self-hosted-runner/run-smoke.sh`
- `tools/mobile-testing-automation/self-hosted-runner/run-smoke.ps1`

Automation scripts (added):

- `start-server.sh` / `start-server.ps1`: start the `server.js` in background and record a PID file.
- `stop-server.sh` / `stop-server.ps1`: stop the background server (PID file or process search).
- `status-server.sh` / `status-server.ps1`: check whether the server is running.
- `run-multi.sh` / `run-multi.ps1`: run the multi-client fMP4 test.
- `run-extended.sh` / `run-extended.ps1`: run the smoke test repeatedly for stability.
- `run-all.sh` / `run-all.ps1`: orchestrate server start, smoke+multi+extended runs, collect artifacts, and stop server.
- `collect-logs.sh` / `collect-logs.ps1`: collect `last_stream-*.h264`, server logs, and other debug artifacts into a timestamped folder.
- `run-smoke-with-artifacts.sh` / `run-smoke-with-artifacts.ps1`: run the smoke test, collect artifacts, and optionally stop the server if this script started it.
- `install-service.sh` / `uninstall-service.sh`: install/uninstall the systemd unit (Linux).
- `install-task.ps1` / `uninstall-task.ps1`: import/remove the Windows Task Scheduler XML.

Usage examples (Linux):

```bash
# start server in background
./tools/mobile-testing-automation/self-hosted-runner/start-server.sh

# run smoke test and collect artifacts
./tools/mobile-testing-automation/self-hosted-runner/run-smoke-with-artifacts.sh

# run the full suite
./tools/mobile-testing-automation/self-hosted-runner/run-all.sh
```

Usage examples (Windows PowerShell):

```powershell
# start server in background
.\tools\mobile-testing-automation\self-hosted-runner\start-server.ps1

# run smoke test and collect artifacts
.\tools\mobile-testing-automation\self-hosted-runner\run-smoke-with-artifacts.ps1

# run the full suite
#.\tools\mobile-testing-automation\self-hosted-runner\run-all.ps1
```

If you want, I can also:
- add a systemd unit file and a sample Windows Task XML file to this folder,
- create a short checklist to add to the repo README for onboarding device labs.

---
Last updated: April 22, 2026
