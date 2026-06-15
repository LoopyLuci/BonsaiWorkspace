<#
PowerShell helper for git-filter-repo cleanup on Windows.
Usage: from repo root in PowerShell: .\scripts\git-filter-repo-cleanup.ps1
#>
param(
    [string]$OldBranch = "fix/warnings-wasm-example",
    [string]$Remote = "origin"
)

Set-StrictMode -Version Latest

$BackupBranch = "backup/$OldBranch-" + (Get-Date -UFormat "%Y%m%dT%H%M%SZ")
Write-Output "Backing up $OldBranch to $BackupBranch on $Remote..."

git fetch $Remote
git checkout -B $BackupBranch $OldBranch
git push $Remote "$BackupBranch`:$BackupBranch"

$origUrl = (git remote get-url $Remote)
$tmpRoot = Join-Path $env:TEMP ("git-filter-repo-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tmpRoot -Force | Out-Null

Write-Output "Creating mirror clone in $tmpRoot..."
git clone --mirror . "$tmpRoot\repo.git"
Set-Location "$tmpRoot\repo.git"

Write-Output "Running git-filter-repo to remove /target/ and common large binaries..."
# require git-filter-repo installed and on PATH
git filter-repo --invert-paths --path-glob "bonsai-runtime/target/**" --path-glob "**/target/**" --path-glob "*.pdb" --path-glob "*.rlib" --path-glob "*.exe" --path-glob "*.dll" --path-glob "*.so" --path-glob "*.dylib"

Write-Output "Pushing rewritten history back to origin (force)..."
if ([string]::IsNullOrWhiteSpace($origUrl)) { throw "origin url not found" }

git remote set-url origin $origUrl

git push --force origin --all

git push --force origin --tags

Write-Output "Done. See scripts/git-filter-repo-instructions.md for next steps and collaborator coordination."