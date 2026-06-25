@echo off
setlocal

for %%I in ("%~dp0..") do set "ROOT=%%~fI"
set "JUSTFILE=%ROOT%\scripts\devkit\justfile"

where just >nul 2>nul
if errorlevel 1 (
  echo DevKit launcher requires 'just'. Install with: cargo install just
  exit /b 1
)

just --justfile "%JUSTFILE%" %*
exit /b %ERRORLEVEL%
