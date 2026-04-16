param(
  [string]$ShortcutName = 'Bonsai Workspace',
  [ValidateSet('User', 'Public', 'Both')]
  [string]$DesktopScope = 'User',
  [string]$LaunchArgs = '',
  [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$workspaceRoot = $PSScriptRoot
$launcherCmd = Join-Path $workspaceRoot 'Launch-BonsaiWorkspace.cmd'
$iconPath = Join-Path $workspaceRoot 'bonsai-workspace\src-tauri\icons\icon.ico'

if (-not (Test-Path $launcherCmd)) {
  throw "Launcher script not found: $launcherCmd"
}

$desktopTargets = @()
switch ($DesktopScope) {
  'User' { $desktopTargets += [Environment]::GetFolderPath('Desktop') }
  'Public' { $desktopTargets += [Environment]::GetFolderPath('CommonDesktopDirectory') }
  'Both' {
    $desktopTargets += [Environment]::GetFolderPath('Desktop')
    $desktopTargets += [Environment]::GetFolderPath('CommonDesktopDirectory')
  }
}

$wsh = New-Object -ComObject WScript.Shell
$created = @()

foreach ($desktop in $desktopTargets) {
  if ([string]::IsNullOrWhiteSpace($desktop) -or -not (Test-Path $desktop)) {
    Write-Warning "Desktop path not available: $desktop"
    continue
  }

  $shortcutPath = Join-Path $desktop ($ShortcutName + '.lnk')
  if ((Test-Path $shortcutPath) -and -not $Force) {
    Write-Host "Shortcut already exists (use -Force to overwrite): $shortcutPath" -ForegroundColor Yellow
    continue
  }

  $shortcut = $wsh.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = $launcherCmd
  $shortcut.Arguments = $LaunchArgs
  $shortcut.WorkingDirectory = $workspaceRoot
  $shortcut.Description = 'Launch Bonsai Workspace'
  if (Test-Path $iconPath) {
    $shortcut.IconLocation = $iconPath
  }
  $shortcut.Save()

  $created += $shortcutPath
  Write-Host "Created shortcut: $shortcutPath" -ForegroundColor Green
}

if ($created.Count -eq 0) {
  Write-Host 'No shortcut was created.' -ForegroundColor Yellow
} else {
  Write-Host 'Done. You can now launch Bonsai from the desktop icon.' -ForegroundColor Cyan
}
