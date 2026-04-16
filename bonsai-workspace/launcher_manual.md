# Bonsai Workspace Launch Manual

This manual is the authoritative, step-by-step guide for launching Bonsai Workspace correctly.
It is written so a first-time user can start the app without guessing.

## 1) What You Are Launching

Bonsai Workspace is a Tauri desktop app with:

- Frontend in `bonsai-workspace/src`
- Rust backend in `bonsai-workspace/src-tauri`

Most command mistakes come from running commands in the wrong folder.

## 2) Supported Launch Modes

Use the mode that matches your goal.

1. All-in-one launcher (recommended): one command handles preflight and launch.
2. Full desktop app manual mode: run Tauri app directly.
3. Frontend-only dev server: run Vite UI by itself.
4. API and test utilities: run scripted validation suites.

If unsure, always use Mode 1.

## 2.1) One-command launcher (recommended)

Fastest Windows one-click entrypoint from workspace root:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace"
.\Launch-BonsaiWorkspace.cmd
```

Examples:

```powershell
# Checks only
.\Launch-BonsaiWorkspace.cmd -PreflightOnly

# Desktop + USB strict validation
.\Launch-BonsaiWorkspace.cmd -Mode desktop+usb -StrictApp -ApkPath "C:\path\to\app.apk" -Serial "DEVICE_SERIAL"
```

This wrapper automatically switches into `bonsai-workspace/src` and runs the unified launcher.

Run from `bonsai-workspace/src`:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src"

# Checks only
npm run launch:preflight

# Checks only + dedicated report file
npm run launch:preflight:report

# Launch desktop app cleanly
npm run launch:desktop

# Launch desktop app + run USB regression + append ledger evidence
npm run launch:desktop+usb
```

Advanced launcher example:

```powershell
npm run launch:all -- --mode desktop+usb --strict-app --apk-path "C:\path\to\app.apk" --serial "DEVICE_SERIAL"
```

Launcher notes:

1. If port `11369` is already used by a healthy Bonsai runtime, launcher will attach to the existing runtime rather than start a duplicate.
2. Every launcher run writes a JSON report to `tool_test/launcher/latest.json` (or custom `--report-path`).
3. On launcher failure, the JSON report is still written and includes the error text.

## 2.2) Exit Code and Report Triage (Important)

If `Launch-BonsaiWorkspace.cmd` returns exit code `1`, do not assume the app failed.

Run this check first:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace"
Get-Content ".\tool_test\launcher\latest.json"
```

Interpretation rules:

1. If `ok` is `true`, launch succeeded even if the shell command returned `1`.
2. If `ok` is `false`, use `error` and `phases` fields to identify failure stage.
3. If API health phase completed, verify Bonsai window/API before retrying.

## 3) Prerequisites (Required)

Install these once:

1. Rust 1.77 or newer
2. Node.js 20 or newer
3. Tauri CLI 2.x
4. Windows only:
	- Visual Studio Build Tools with Desktop development with C++
	- Microsoft WebView2 Runtime

Quick checks:

```powershell
rustc --version
cargo --version
node --version
npm --version
cargo tauri --version
```

If `cargo tauri --version` fails:

```powershell
cargo install tauri-cli --version "^2"
```

## 4) First-Time Setup (Required Once Per Machine)

From workspace root:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src"
npm install
```

Optional but recommended backend compile check:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace"
cargo check --manifest-path "bonsai-workspace\src-tauri\Cargo.toml"
```

## 5) Correct Full Launch Procedure (Manual Mode)

This is the default way to run Bonsai.

```powershell
Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src-tauri"
cargo tauri dev
```

Expected behavior:

1. Vite dev server starts automatically for `../src`.
2. Tauri desktop window opens.
3. You can open Settings and interact with the app.

Do not run `npm run dev` at workspace root. It will fail because frontend scripts live in `bonsai-workspace/src`.

## 6) Frontend-Only Launch (Mode 3)

Use this only when you intentionally do UI-only work.

```powershell
Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src"
npm run dev
```

This does not launch the full Tauri desktop runtime.

## 7) Smoke and Regression Commands (Mode 4)

Run from `bonsai-workspace/src` unless noted.

Core tests:

```powershell
npm run test:agent-routing-ci
npm run test:agent-orchestrated
npm run test:bonsai-live-testing-feature:watch
```

Android USB regression:

```powershell
npm run test:android-usb-regression
```

Evidence ledger append:

```powershell
npm run evidence:append-usb-ledger
```

## 8) Android Tablet Launch and Connection Flow

When a tablet is connected over USB debugging:

1. Launch Bonsai with Mode 1.
2. Open Settings.
3. Use Android USB Lab:
	- Refresh USB Devices
	- Select device serial
	- Set adb reverse to API port
	- Install APK (if you have one)
	- Launch App
	- Run USB Regression Suite

Terminal equivalent quick verification:

```powershell
npm run test:android-usb-regression
```

If package is not installed, launch may be skipped unless strict mode is required.

## 9) Launch Verification Checklist

After startup, confirm all are true:

1. Desktop window is visible.
2. Settings opens without errors.
3. API test in settings can run.
4. If tablet connected, USB device appears in list.
5. `npm run test:android-usb-regression` returns `USB_REGRESSION_OK=1`.

## 10) Common Failures and Exact Fixes

1. Error: `npm run dev` fails at workspace root
	- Fix: run it from `bonsai-workspace/src`.

2. Error: `cargo tauri dev` fails with missing toolchain/components
	- Fix: install Visual Studio Build Tools C++ workload and WebView2 runtime.

3. Error: `adb` not found
	- Fix: install Android platform-tools or ensure `%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe` exists.

4. USB device listed as unauthorized
	- Fix: approve USB debugging prompt on tablet, then refresh devices.

5. Regression passes but app did not launch
	- Cause: package not installed and strict mode not enabled.
	- Fix: install APK first, then re-run with strict requirement when needed.

6. Launcher command returned exit code `1`, but app appears healthy
	- Cause: shell interruption or launcher process-level nonzero exit while runtime became healthy.
	- Fix: inspect `tool_test/launcher/latest.json`; treat `ok=true` as successful launch.

## 11) Daily Operator Shortcut

If dependencies are already installed, this is the fastest repeatable start:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src"
npm run launch:desktop
```

Then for USB tablet validation in a second terminal:

```powershell
Set-Location "z:\Projects\BonsaiWorkspace\bonsai-workspace\src"
npm run launch:desktop+usb
```

## 12) Definition of Launch Success

A correct launch is complete only when:

1. Desktop app opens through `cargo tauri dev`.
2. No folder-path command mistakes were made.
3. Optional tablet path is validated (if connected).
4. Regression evidence is generated and appendable.
