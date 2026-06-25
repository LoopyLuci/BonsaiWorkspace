# ============================================================================
# OMNISYSTEM LAUNCHERS BUILD SCRIPT
# ============================================================================
# Builds all BonsaiEcosystem launchers and desktop environment
# Handles both standard Rust projects and Tauri applications
# ============================================================================

param(
    [ValidateSet("all", "desktop", "gui", "launcher")]
    [string]$Target = "all",
    [switch]$Release = $false,
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BuildMode = if ($Release) { "release" } else { "debug" }

# ============================================================================
# CONFIGURATION
# ============================================================================

$Config = @{
    BuildDir = Join-Path $RootDir "build"
    OutputDir = Join-Path $RootDir "Omnisystem\launchers"
    LogFile = Join-Path $RootDir "build\build.log"
    Projects = @()
}

# Define projects
$Config.Projects = @(
    @{
        Name = "Desktop Environment (Main)"
        Key = "desktop"
        Path = "Omnisystem\applications\bonsai-desktop-environment"
        Type = "cargo"
        BinaryName = "Omnisystem"
        OutputName = "Omnisystem.exe"
        Description = "BonsaiEcosystem Desktop Environment v29.0.0"
    },
    @{
        Name = "Desktop Environment (TUI)"
        Key = "desktop"
        Path = "Omnisystem\applications\bonsai-desktop-environment"
        Type = "cargo"
        BinaryName = "Omnisystem_TUI"
        OutputName = "Omnisystem_TUI.exe"
        Description = "BonsaiEcosystem Desktop Environment - Interactive TUI"
    },
    @{
        Name = "Omnisystem GUI Launcher"
        Key = "gui"
        Path = "Omnisystem\src\crates\omnisystem-launcher-gui\src-tauri"
        Type = "tauri"
        BinaryName = "omnisystem-launcher-tauri"
        OutputName = "OmnisystemGUI.exe"
        Description = "Omnisystem Native Desktop Launcher (Tauri)"
    },
    @{
        Name = "BonsaiEcosystem Launcher"
        Key = "launcher"
        Path = "Omnisystem\modules\base-modules\applications\bonsai-ecosystem\launcher"
        Type = "cargo"
        BinaryName = "bonsai-launcher"
        OutputName = "BonsaiLauncher.exe"
        Description = "BonsaiEcosystem Application Launcher"
    }
)

# ============================================================================
# UTILITY FUNCTIONS
# ============================================================================

function Write-Header {
    param([string]$Text)
    Write-Host "`n╔════════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║ $($Text.PadRight(62)) ║" -ForegroundColor Cyan
    Write-Host "╚════════════════════════════════════════════════════════════════╝`n" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Text)
    Write-Host "✓ $Text" -ForegroundColor Green
}

function Write-Error {
    param([string]$Text)
    Write-Host "✗ $Text" -ForegroundColor Red
}

function Write-Info {
    param([string]$Text)
    Write-Host "• $Text" -ForegroundColor Cyan
}

function Write-Log {
    param([string]$Message)
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $LogMsg = "[$Timestamp] $Message"

    # Ensure directory exists
    $LogDir = Split-Path -Parent $Config.LogFile
    if (-not (Test-Path $LogDir)) {
        New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
    }

    Add-Content -Path $Config.LogFile -Value $LogMsg
}

function Test-Prerequisites {
    Write-Info "Checking prerequisites..."

    try {
        $RustVer = & cargo --version 2>&1
        Write-Success "Rust/Cargo: $RustVer"
    }
    catch {
        Write-Error "Rust not found. Install from https://rustup.rs/"
        return $false
    }

    return $true
}

function Build-CargoProject {
    param(
        [hashtable]$Project,
        [string]$RootDir,
        [string]$BuildMode,
        [string]$BuildDir
    )

    $ProjectPath = Join-Path $RootDir $Project.Path

    Write-Host "`nBuilding: $($Project.Name)"
    Write-Host "  Path: $ProjectPath"

    if (-not (Test-Path $ProjectPath)) {
        Write-Error "Project path not found"
        Write-Log "ERROR: $($Project.Name) - Path not found: $ProjectPath"
        return $false
    }

    if (-not (Test-Path (Join-Path $ProjectPath "Cargo.toml"))) {
        Write-Error "Cargo.toml not found"
        Write-Log "ERROR: $($Project.Name) - Cargo.toml not found"
        return $false
    }

    try {
        Push-Location $ProjectPath

        $CargoCmd = @("build")
        if ($Release) { $CargoCmd += "--release" }

        Write-Info "Executing: cargo $($CargoCmd -join ' ')"
        Write-Log "$($Project.Name): cargo $($CargoCmd -join ' ')"

        $Output = & cargo $CargoCmd 2>&1
        $Output | ForEach-Object { Write-Log $_ }

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Build failed with exit code $LASTEXITCODE"
            Write-Log "ERROR: $($Project.Name) - Build failed"
            return $false
        }

        # Verify binary exists
        $TargetSubdir = if ($Release) { "release" } else { "debug" }
        $BinaryPath = Join-Path $ProjectPath "target\$TargetSubdir\$($Project.BinaryName).exe"

        if (Test-Path $BinaryPath) {
            Write-Success "Binary created: $($Project.BinaryName).exe"
            Write-Log "$($Project.Name): Binary created successfully"

            # Copy to output
            $OutputPath = Join-Path $Config.OutputDir $Project.OutputName
            Copy-Item -Path $BinaryPath -Destination $OutputPath -Force
            Write-Success "Copied to: $($Project.OutputName)"

            return $true
        } else {
            Write-Error "Binary not found at expected path"
            Write-Log "ERROR: $($Project.Name) - Binary not found at $BinaryPath"
            return $false
        }
    }
    catch {
        Write-Error "Build error: $_"
        Write-Log "ERROR: $($Project.Name) - $_"
        return $false
    }
    finally {
        Pop-Location
    }
}

function Build-TauriProject {
    param(
        [hashtable]$Project,
        [string]$RootDir,
        [string]$BuildMode,
        [string]$BuildDir
    )

    $ProjectPath = Join-Path $RootDir $Project.Path

    Write-Host "`nBuilding: $($Project.Name) (Tauri)"
    Write-Host "  Path: $ProjectPath"

    if (-not (Test-Path $ProjectPath)) {
        Write-Error "Project path not found"
        return $false
    }

    try {
        Push-Location $ProjectPath

        # Check for tauri.conf.json or tauri-build setup
        if (-not (Test-Path "Cargo.toml")) {
            Write-Error "Cargo.toml not found"
            return $false
        }

        Write-Info "Installing dependencies..."
        Write-Log "$($Project.Name): Installing dependencies"

        # For Tauri projects, we need cargo build
        $CargoCmd = @("build")
        if ($Release) { $CargoCmd += "--release" }

        Write-Info "Executing: cargo $($CargoCmd -join ' ')"
        Write-Log "$($Project.Name): cargo $($CargoCmd -join ' ')"

        $Output = & cargo $CargoCmd 2>&1
        $Output | ForEach-Object { Write-Log $_ }

        if ($LASTEXITCODE -ne 0) {
            Write-Error "Build failed with exit code $LASTEXITCODE"
            Write-Log "ERROR: $($Project.Name) - Build failed"
            return $false
        }

        # Find the compiled binary
        $TargetSubdir = if ($Release) { "release" } else { "debug" }
        $PossiblePaths = @(
            (Join-Path $ProjectPath "target\$TargetSubdir\$($Project.BinaryName).exe"),
            (Join-Path $ProjectPath "..\..\..\target\$TargetSubdir\$($Project.BinaryName).exe"),
            (Join-Path $ProjectPath "target\$TargetSubdir\$($Project.BinaryName.Replace('-', '_')).exe")
        )

        $BinaryPath = $PossiblePaths | Where-Object { Test-Path $_ } | Select-Object -First 1

        if ($BinaryPath) {
            Write-Success "Binary created: $($Project.BinaryName).exe"

            # Copy to output with file locking handling
            $OutputPath = Join-Path $Config.OutputDir $Project.OutputName
            try {
                # Try to stop running process if it exists
                $ProcessName = [System.IO.Path]::GetFileNameWithoutExtension($Project.OutputName)
                Stop-Process -Name $ProcessName -Force -ErrorAction SilentlyContinue
                Start-Sleep -Milliseconds 500

                Copy-Item -Path $BinaryPath -Destination $OutputPath -Force -ErrorAction Stop
                Write-Success "Copied to: $($Project.OutputName)"
                return $true
            }
            catch {
                Write-Error "Failed to copy binary: $_"
                Write-Log "ERROR: $($Project.Name) - Copy failed: $_"
                return $false
            }
        } else {
            Write-Error "Binary not found at expected paths"
            Write-Log "ERROR: $($Project.Name) - Binary not found"
            return $false
        }
    }
    catch {
        Write-Error "Build error: $_"
        Write-Log "ERROR: $($Project.Name) - $_"
        return $false
    }
    finally {
        Pop-Location
    }
}

# ============================================================================
# MAIN EXECUTION
# ============================================================================

function Main {
    Write-Header "OMNISYSTEM BUILD SYSTEM"

    Write-Info "Target: $Target"
    Write-Info "Mode: $BuildMode"
    Write-Info "Root: $RootDir"

    # Setup
    Write-Log "═════════════════════════════════════════════════════════════"
    Write-Log "BUILD STARTED - Target: $Target, Mode: $BuildMode"

    New-Item -ItemType Directory -Path $Config.BuildDir -Force | Out-Null
    New-Item -ItemType Directory -Path $Config.OutputDir -Force | Out-Null

    if ($Clean) {
        Write-Info "Cleaning build artifacts..."
        Remove-Item -Path $Config.BuildDir -Recurse -Force -ErrorAction SilentlyContinue
        New-Item -ItemType Directory -Path $Config.BuildDir -Force | Out-Null
        New-Item -ItemType Directory -Path $Config.OutputDir -Force | Out-Null
    }

    # Prerequisites
    if (-not (Test-Prerequisites)) {
        return 1
    }

    # Select projects to build
    $ProjectsToBuild = if ($Target -eq "all") {
        $Config.Projects
    } else {
        $Config.Projects | Where-Object { $_.Key -eq $Target }
    }

    Write-Host "`nBuilding $($ProjectsToBuild.Count) project(s)..." -ForegroundColor Cyan

    $Results = @{
        Success = 0
        Failed = 0
        Projects = @()
    }

    # Build projects
    foreach ($Project in $ProjectsToBuild) {
        $Success = $false

        switch ($Project.Type) {
            "cargo" {
                $Success = Build-CargoProject $Project $RootDir $BuildMode $Config.BuildDir
            }
            "tauri" {
                $Success = Build-TauriProject $Project $RootDir $BuildMode $Config.BuildDir
            }
        }

        if ($Success) {
            $Results.Success++
        } else {
            $Results.Failed++
        }

        $Results.Projects += @{
            Name = $Project.Name
            Success = $Success
        }
    }

    # Summary
    Write-Header "BUILD SUMMARY"

    foreach ($Result in $Results.Projects) {
        if ($Result.Success) {
            Write-Success $Result.Name
        } else {
            Write-Error $Result.Name
        }
    }

    Write-Host "`nResults: $($Results.Success) successful, $($Results.Failed) failed" -ForegroundColor Cyan

    if ($Results.Failed -eq 0) {
        Write-Host "`n✓ All builds completed successfully!" -ForegroundColor Green
        Write-Host "  Output directory: $($Config.OutputDir)`n" -ForegroundColor Green
        Write-Log "BUILD COMPLETED SUCCESSFULLY"
        return 0
    } else {
        Write-Host "`n✗ Some builds failed. Check $($Config.LogFile) for details.`n" -ForegroundColor Red
        Write-Log "BUILD FAILED"
        return 1
    }
}

exit (Main)
