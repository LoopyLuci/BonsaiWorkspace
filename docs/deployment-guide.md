# Bonsai Ecosystem — Production Deployment Guide

> **Audience:** System administrators and power users deploying the Bonsai Ecosystem on a
> dedicated machine or server.  
> **Applies to:** Bonsai Workspace · BonsaiBot · Bonsai Everywhere · Bonsai Buddy Android

---

## Table of Contents

1. [System Requirements](#1-system-requirements)
2. [Installing Prerequisites](#2-installing-prerequisites)
3. [Building from Source](#3-building-from-source)
4. [Running as a Service](#4-running-as-a-service)
5. [Auto-Start on Boot](#5-auto-start-on-boot)
6. [Firewall Configuration](#6-firewall-configuration)
7. [Securing the Deployment](#7-securing-the-deployment)
8. [Backing Up Configuration & Data](#8-backing-up-configuration--data)
9. [Upgrading Between Versions](#9-upgrading-between-versions)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. System Requirements

### Minimum (CPU-only inference)

| Component | Requirement |
|-----------|-------------|
| CPU | x86-64, 4 cores, AVX2 support |
| RAM | 8 GB (16 GB recommended for 7B models) |
| Disk | 20 GB free (SSD strongly recommended) |
| OS | Windows 10/11 · Ubuntu 22.04+ · macOS 13+ |
| Network | Local LAN access on ports 11369, 11420, 11666 |

### Recommended (GPU-accelerated inference)

| Component | Requirement |
|-----------|-------------|
| GPU | NVIDIA RTX 3060+ (8 GB VRAM) or AMD RX 6700+ |
| RAM | 32 GB |
| Disk | 100 GB NVMe SSD |
| CUDA | 12.x (NVIDIA) / ROCm 5.7+ (AMD) |

### Large-Scale / Server Deployments

| Component | Requirement |
|-----------|-------------|
| CPU | 16+ cores |
| RAM | 64–128 GB |
| GPU | NVIDIA A-series / RTX 4090 (24 GB VRAM) |
| Disk | 500 GB+ NVMe, RAID recommended |

---

## 2. Installing Prerequisites

### 2.1 Rust (required for Workspace & Bot)

```bash
# All platforms
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup update stable
```

Windows (PowerShell):
```powershell
winget install Rustlang.Rustup
rustup update stable
```

### 2.2 Node.js 20 LTS (required for frontend build)

```bash
# Linux/macOS — use nvm
nvm install 20 && nvm use 20

# Windows
winget install OpenJS.NodeJS.LTS
```

### 2.3 Android Platform Tools (optional — for Bonsai Buddy APK)

Install Android Studio or the standalone command-line tools:
- **JDK 17+** required (`java --version`)
- `ANDROID_HOME` environment variable must point to the SDK root
- Accept all SDK licenses: `sdkmanager --licenses`

### 2.4 ffmpeg (optional — for media-processing tools)

```bash
# Ubuntu/Debian
sudo apt install ffmpeg

# macOS
brew install ffmpeg

# Windows
winget install Gyan.FFmpeg
```

### 2.5 Babashka (optional — for Clojure task automation)

```bash
# Linux/macOS
curl -sLO https://raw.githubusercontent.com/babashka/babashka/master/install
bash install

# Windows (PowerShell)
.\scripts\install-bb.ps1
```

---

## 3. Building from Source

### 3.1 Clone the Repository

```bash
git clone https://github.com/LoopyLuci/BonsaiWorkspace.git
cd BonsaiWorkspace
```

### 3.2 Build Bonsai Workspace (Desktop)

The recommended path uses the provided launcher builder which orchestrates the
frontend build and Tauri compilation in a single pass.

**Windows:**
```powershell
.\BonsaiExeLauncherBuilder.ps1
```

**Linux / macOS:**
```bash
cd bonsai-workspace
npm install
npm run build
cd src-tauri
cargo build --release
```

Output artifact: `BonsaiWorkspace.exe` (Windows) or `target/release/bonsai-workspace` (Unix).

### 3.3 Build BonsaiBot

```bash
cd bonsai-bot
cargo build --release
```

Output: `target/release/bonsai-bot` (Unix) / `target\release\bonsai-bot.exe` (Windows).

Configure platform adapters before first run:
```bash
cp bonsai-bot/config.example.toml bonsai-bot/config.toml
# Edit config.toml — add Discord token, Telegram token, email credentials, Matrix homeserver
```

### 3.4 Build Browser Extension (Bonsai Everywhere)

```bash
cd browser-extension
npm install
npm run build:chrome   # outputs dist/chrome/
npm run build:firefox  # outputs dist/firefox/
```

- **Chrome/Edge/Brave/Opera GX:** Load `dist/chrome/` as an unpacked extension in developer mode.
- **Firefox:** Load `dist/firefox/manifest.json` via `about:debugging`.
- **Production ZIP/XPI:** `npm run package` (see `scripts/package-release.ps1` for automated packaging).

### 3.5 Build Bonsai Buddy (Android)

```bash
cd bonsai-buddy-android
./gradlew assembleDebug       # debug APK
./gradlew assembleRelease     # release APK (requires signing config)
```

APK output: `app/build/outputs/apk/`

---

## 4. Running as a Service

### 4.1 Windows — NSSM (Non-Sucking Service Manager)

Install NSSM: `winget install NSSM.NSSM`

**Bonsai Workspace service:**
```powershell
nssm install BonsaiWorkspace "C:\Path\To\BonsaiWorkspace.exe"
nssm set BonsaiWorkspace AppDirectory "C:\Path\To\BonsaiWorkspace"
nssm set BonsaiWorkspace AppStdout "C:\logs\workspace-stdout.log"
nssm set BonsaiWorkspace AppStderr "C:\logs\workspace-stderr.log"
nssm set BonsaiWorkspace Start SERVICE_AUTO_START
nssm start BonsaiWorkspace
```

**BonsaiBot service:**
```powershell
nssm install BonsaiBot "C:\Path\To\bonsai-bot.exe"
nssm set BonsaiBot AppDirectory "C:\Path\To\bonsai-bot"
nssm set BonsaiBot AppEnvironmentExtra "BONSAI_BOT_ADMIN_TOKEN=your-token-here"
nssm set BonsaiBot Start SERVICE_AUTO_START
nssm start BonsaiBot
```

### 4.2 Linux — systemd

**Bonsai Workspace unit** (`/etc/systemd/system/bonsai-workspace.service`):

```ini
[Unit]
Description=Bonsai Workspace API
After=network.target

[Service]
Type=simple
User=bonsai
WorkingDirectory=/opt/bonsai
ExecStart=/opt/bonsai/bonsai-workspace
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

**BonsaiBot unit** (`/etc/systemd/system/bonsai-bot.service`):

```ini
[Unit]
Description=BonsaiBot Multi-Platform Adapter
After=network.target bonsai-workspace.service

[Service]
Type=simple
User=bonsai
WorkingDirectory=/opt/bonsai/bonsai-bot
ExecStart=/opt/bonsai/bonsai-bot/bonsai-bot
EnvironmentFile=/etc/bonsai/bot.env
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=multi-user.target
```

Activate:
```bash
sudo systemctl daemon-reload
sudo systemctl enable bonsai-workspace bonsai-bot
sudo systemctl start bonsai-workspace bonsai-bot
sudo systemctl status bonsai-workspace bonsai-bot
```

---

## 5. Auto-Start on Boot

### Windows

The NSSM services created above start automatically. Verify:
```powershell
Get-Service BonsaiWorkspace, BonsaiBot | Select-Object Name, StartType, Status
```

### Linux

```bash
sudo systemctl is-enabled bonsai-workspace
# Should print: enabled
```

### macOS — launchd

Create `/Library/LaunchDaemons/com.bonsai.workspace.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.bonsai.workspace</string>
  <key>ProgramArguments</key>
  <array>
    <string>/opt/bonsai/bonsai-workspace</string>
  </array>
  <key>WorkingDirectory</key><string>/opt/bonsai</string>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>/var/log/bonsai-workspace.log</string>
  <key>StandardErrorPath</key><string>/var/log/bonsai-workspace.err</string>
</dict>
</plist>
```

```bash
sudo launchctl load /Library/LaunchDaemons/com.bonsai.workspace.plist
```

---

## 6. Firewall Configuration

All three API servers bind to `127.0.0.1` by default. To allow remote access
(e.g., for Bonsai Buddy Android or BonsaiBot on another machine), open inbound
rules only for trusted source IPs.

| Service | Port | Protocol |
|---------|------|----------|
| Bonsai Workspace API | 11369 | TCP |
| Bonsai Buddy API | 11420 | TCP |
| Bot Admin API | 11666 | TCP |

### Windows Firewall (PowerShell)

```powershell
# Example: allow Workspace API from local subnet only
New-NetFirewallRule -DisplayName "Bonsai Workspace API" `
  -Direction Inbound -Protocol TCP -LocalPort 11369 `
  -RemoteAddress 192.168.1.0/24 -Action Allow
```

### Linux (ufw)

```bash
# Allow from specific subnet
sudo ufw allow from 192.168.1.0/24 to any port 11369
sudo ufw allow from 192.168.1.0/24 to any port 11420
```

> **Security note:** Never expose port 11666 (Bot Admin API) to the public internet.
> Use a reverse proxy with mTLS or VPN if remote admin access is required.

### Reverse Proxy (nginx — optional)

```nginx
server {
  listen 443 ssl;
  server_name bonsai.example.internal;

  ssl_certificate     /etc/ssl/bonsai.crt;
  ssl_certificate_key /etc/ssl/bonsai.key;

  location /workspace/ {
    proxy_pass http://127.0.0.1:11369/;
  }

  location /buddy/ {
    proxy_pass http://127.0.0.1:11420/;
  }
}
```

---

## 7. Securing the Deployment

### 7.1 Token Management

- Generate the Bot Admin token with a cryptographically secure random source:
  ```bash
  # Linux/macOS
  openssl rand -hex 32
  
  # PowerShell
  [System.Security.Cryptography.RandomNumberGenerator]::GetBytes(32) | ForEach-Object { "{0:x2}" -f $_ } | Join-String
  ```
- Store tokens in environment variables or an encrypted secrets manager, **never** in plain-text config files committed to git.
- Rotate tokens at least every 90 days, or immediately after any suspected compromise.

### 7.2 Allowed Origins

Set `BONSAI_ALLOWED_ORIGINS` to restrict which domains may call the API:
```bash
export BONSAI_ALLOWED_ORIGINS="http://localhost:1420,https://bonsai.example.internal"
```

### 7.3 TLS for Remote Access

Always use HTTPS when exposing any Bonsai endpoint beyond localhost. Use nginx or
Caddy as a TLS-terminating reverse proxy. Do not use self-signed certificates for
production.

### 7.4 Running as a Non-Root User

Create a dedicated system user:
```bash
sudo useradd -r -s /sbin/nologin -d /opt/bonsai bonsai
sudo chown -R bonsai:bonsai /opt/bonsai
```

### 7.5 Resource Limits

The `bonsai-runtime` crate automatically applies resource limits to Python
worker processes (Windows Job Objects / POSIX `rlimit`). Review limits in
`bonsai-runtime/src/limits.rs` and adjust CPU-time and memory caps for your
workload.

---

## 8. Backing Up Configuration & Data

### Critical Files to Back Up

| Path | Contents |
|------|----------|
| `bonsai-bot/config.toml` | Bot adapter credentials |
| `bonsai-workspace/data/` | SQLite databases, model cache index |
| `bonsai-workspace/src-tauri/tauri.conf.json` | Application config |
| `runtimes/python/` | Python worker scripts |
| `.env` / `bot.env` | Environment secrets |

### Backup Script (Linux/macOS)

```bash
#!/usr/bin/env bash
DEST="/backup/bonsai-$(date +%Y%m%d)"
mkdir -p "$DEST"
rsync -av --exclude='target/' --exclude='node_modules/' \
  /opt/bonsai/ "$DEST/"
echo "Backup complete: $DEST"
```

### Windows (PowerShell)

```powershell
$dest = "D:\Backup\bonsai-$(Get-Date -Format yyyyMMdd)"
robocopy Z:\Projects\BonsaiWorkspace $dest /E /XD target node_modules .git
```

> Automate daily backups with Task Scheduler (Windows) or `cron` (Linux/macOS).

---

## 9. Upgrading Between Versions

1. **Read the CHANGELOG.md** for breaking changes and migration notes.
2. **Stop services:**
   ```bash
   sudo systemctl stop bonsai-workspace bonsai-bot
   # Windows: nssm stop BonsaiWorkspace; nssm stop BonsaiBot
   ```
3. **Back up current data** (see §8).
4. **Pull latest code:**
   ```bash
   git fetch origin
   git checkout main
   git pull --ff-only
   ```
5. **Rebuild:**
   ```powershell
   .\BonsaiExeLauncherBuilder.ps1       # Windows
   # Or: cargo build --release          # Unix, per crate
   ```
6. **Restart services:**
   ```bash
   sudo systemctl start bonsai-workspace bonsai-bot
   ```
7. **Verify health:**
   ```bash
   curl http://127.0.0.1:11369/health
   curl http://127.0.0.1:11420/health
   ```

---

## 10. Troubleshooting

### 10.1 Service Fails to Start

- Check logs: `journalctl -u bonsai-workspace -n 50` (Linux) or NSSM log files (Windows).
- Verify the binary path and working directory are correct.
- Ensure the port is not in use: `ss -tlnp | grep 11369` (Linux) / `netstat -ano | findstr 11369` (Windows).

### 10.2 "No Model Slot Is Ready"

- The Workspace API starts before a model is loaded. This is transient — wait a few seconds and retry.
- If the error persists, check that a model file exists in the configured models directory and is not corrupted.

### 10.3 GPU Inference Crashes

- Bonsai Workspace auto-recovers from GPU crashes by falling back to CPU mode.
- To force CPU-only mode, set the inference mode to **CPU Only** in Settings or set `BONSAI_INFERENCE_MODE=cpu` in the environment.
- Check GPU driver version: CUDA 12.x for NVIDIA, ROCm 5.7+ for AMD.

### 10.4 BonsaiBot Platform Adapter Disconnected

- Verify the token/credential in `config.toml` is valid and not expired.
- Check the bot's rate-limit status on each platform's developer dashboard.
- Review bot logs: `journalctl -u bonsai-bot -n 100`.

### 10.5 Browser Extension Fails to Connect

- Ensure Bonsai Workspace is running on port 11369.
- Check the extension's **Allowed Origins** setting matches `http://localhost:11369`.
- On macOS/Linux, confirm no firewall is blocking `127.0.0.1:11369`.

### 10.6 Android App (Bonsai Buddy) Cannot Reach Workspace

- The device must be on the same LAN as the host, or connected via VPN.
- Enter the host's LAN IP in Bonsai Buddy's connection settings (not `localhost`).
- Confirm port 11420 is open in the host firewall for the device's subnet.

### 10.7 Launcher PID Lock Prevents Startup

If `BonsaiWorkspace.exe` refuses to start with "already running":
```powershell
# Check if the PID in the lock file is actually alive
$pid = Get-Content .bonsai-launcher.pid
Get-Process -Id $pid -ErrorAction SilentlyContinue

# If the process is gone, remove the stale lock file
Remove-Item .bonsai-launcher.pid -Force
```

---

*For additional help, open an issue at https://github.com/LoopyLuci/BonsaiWorkspace/issues*
