param(
    [string]$Message = "chore: add mcp-server crate"
)

Write-Host "Staging changes..."
git add .

$status = git status --porcelain
if (-not $status) {
    Write-Host "No changes to commit."
    exit 0
}

Write-Host "Committing with message: $Message"
git commit -m $Message
if ($LASTEXITCODE -ne 0) {
    Write-Error "git commit failed"
    exit $LASTEXITCODE
}

Write-Host "Committed."
