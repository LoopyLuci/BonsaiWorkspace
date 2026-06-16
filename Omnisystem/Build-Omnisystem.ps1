#Requires -Version 5.0
<#
.SYNOPSIS
    Omnisystem Build Script - Creates Omnisystem.exe in root

.DESCRIPTION
    One-line build: run once, get Omnisystem.exe in root. That's it.

.EXAMPLE
    .\Build-Omnisystem.ps1
    .\Build-Omnisystem.ps1 -Launch

#>

param(
    [switch]$Launch
)

$ErrorActionPreference = "Stop"

# Setup paths
$OmnisystemDir = Split-Path -Parent $PSCommandPath
$RootDir = Split-Path -Parent $OmnisystemDir
$LauncherScript = Join-Path $OmnisystemDir "Omnisystem.Launcher.ps1"
$ExePath = Join-Path $RootDir "Omnisystem.exe"
$TempDir = Join-Path $OmnisystemDir ".build-temp"

Write-Host ""
Write-Host "OMNISYSTEM BUILD" -ForegroundColor Cyan
Write-Host ""

# Verify launcher script exists
if (-not (Test-Path $LauncherScript)) {
    Write-Host "ERROR: Launcher script not found" -ForegroundColor Red
    exit 1
}

Write-Host "Creating Omnisystem.exe..." -ForegroundColor Yellow

# Create temp directory for build artifacts
if (-not (Test-Path $TempDir)) {
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
}

# C# source for minimal launcher
$CsSource = @"
using System;
using System.Diagnostics;
using System.IO;

namespace Omnisystem {
    class Program {
        static void Main(string[] args) {
            // Get Omnisystem directory
            string omnisystemDir = Path.Combine(
                AppDomain.CurrentDomain.BaseDirectory,
                "..", "Omnisystem"
            );
            string launcherScript = Path.Combine(omnisystemDir, "Omnisystem.Launcher.ps1");

            // Launch PowerShell with the launcher script
            ProcessStartInfo psi = new ProcessStartInfo {
                FileName = "powershell.exe",
                Arguments = $"-NoExit -ExecutionPolicy Bypass -File \"{launcherScript}\"",
                UseShellExecute = false,
                RedirectStandardOutput = false
            };

            try {
                using (Process p = Process.Start(psi)) {
                    p.WaitForExit();
                }
            } catch (Exception ex) {
                Console.WriteLine("ERROR: Failed to launch Omnisystem");
                Console.WriteLine(ex.Message);
                Environment.Exit(1);
            }
        }
    }
}
"@

# Compile C# to exe
$CsFile = Join-Path $TempDir "Program.cs"
$CsExe = Join-Path $TempDir "Omnisystem_build.exe"

# Write C# source
Set-Content -Path $CsFile -Value $CsSource

# Compile using csc.exe
$CscPath = "C:\Program Files\Microsoft Visual Studio\*\*\MSBuild\Current\Bin\Roslyn\csc.exe"
$CscExe = Get-ChildItem $CscPath -ErrorAction SilentlyContinue | Select-Object -First 1

if ($CscExe) {
    Write-Host "Compiling with C# compiler..." -ForegroundColor Yellow
    & $CscExe.FullName /out:$CsExe $CsFile 2>&1 | Out-Null
} else {
    # Fallback: try System.Reflection.Metadata approach
    Write-Host "Using .NET compilation..." -ForegroundColor Yellow

    $CompilerPath = Join-Path $env:ProgramFiles "dotnet\dotnet.exe"
    if (Test-Path $CompilerPath) {
        $ProjFile = Join-Path $TempDir "build.csproj"

        @"
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net6.0</TargetFramework>
    <PublishSingleFile>true</PublishSingleFile>
    <SelfContained>true</SelfContained>
  </PropertyGroup>
</Project>
"@ | Set-Content $ProjFile

        & $CompilerPath publish $ProjFile -c Release -o (Split-Path $CsExe) 2>&1 | Out-Null
        $CsExe = Join-Path (Split-Path $CsExe) "build" "build.exe"
    } else {
        Write-Host "ERROR: No C# compiler found. Install .NET SDK from https://dotnet.microsoft.com/download" -ForegroundColor Red
        exit 1
    }
}

if (-not (Test-Path $CsExe)) {
    Write-Host "ERROR: Failed to compile launcher" -ForegroundColor Red
    exit 1
}

# Copy to root
Copy-Item $CsExe $ExePath -Force

if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: Failed to create Omnisystem.exe" -ForegroundColor Red
    exit 1
}

$FileSize = [math]::Round((Get-Item $ExePath).Length / 1KB, 2)
Write-Host ""
Write-Host "SUCCESS: Omnisystem.exe ready ($FileSize KB)" -ForegroundColor Green
Write-Host "Location: $ExePath" -ForegroundColor Green
Write-Host ""

# Cleanup temp
Remove-Item $TempDir -Recurse -Force -ErrorAction SilentlyContinue

# Launch if requested
if ($Launch) {
    Write-Host "Launching..." -ForegroundColor Cyan
    Write-Host ""
    & $ExePath
}
