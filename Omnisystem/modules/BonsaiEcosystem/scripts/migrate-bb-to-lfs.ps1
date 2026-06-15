# Migrate tools/bb.exe into Git LFS and remove it from history (Windows PowerShell)
# Run this from repo root. Requires git and git-lfs installed and on PATH.

Write-Host "Starting Git LFS migration for tools/bb.exe"

# Ensure git-lfs initialized
git lfs install

# Track the binary
git lfs track "tools/bb.exe"

# Add .gitattributes (git-lfs updates it)
git add .gitattributes
try {
    git commit -m "track tools/bb.exe with git-lfs"
} catch {
    Write-Host "No .gitattributes changes to commit"
}

Write-Host "Running 'git lfs migrate import' (this rewrites history locally)"
Write-Host "This may take a while depending on repo size."

# Perform migration
git lfs migrate import --include="tools/bb.exe"

Write-Host "Migration complete locally. Inspect your history, then push with force if you are ready:"
Write-Host "  git push --force --all"
Write-Host "  git push --force --tags"

Write-Host "Note: Force-pushing rewritten history will affect all collaborators. Coordinate before pushing."
