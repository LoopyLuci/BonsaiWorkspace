# Shim — delegates to scripts/build/BonsaiExeLauncherBuilder.ps1
& (Join-Path $PSScriptRoot 'scripts\build\BonsaiExeLauncherBuilder.ps1') @args
