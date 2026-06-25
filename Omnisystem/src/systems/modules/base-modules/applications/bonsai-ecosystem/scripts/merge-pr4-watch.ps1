param(
    [int]$pr = 4,
    [string]$repoOwner = 'LoopyLuci',
    [string]$repoName = 'BonsaiWorkspace',
    [int]$pollSeconds = 15,
    [int]$maxMinutes = 120
)

$start = Get-Date
Write-Host "Watching PR #$pr checks for repo $repoOwner/$repoName ..."
while ($true) {
    $json = gh pr view $pr --repo "$repoOwner/$repoName" --json statusCheckRollup,mergeable,state --jq '.' 2>$null
    if (-not $json) {
        Write-Host "gh returned no JSON; retrying..."
        Start-Sleep -Seconds 5
        continue
    }
    $obj = $json | ConvertFrom-Json
    if (-not $obj.statusCheckRollup) {
        Write-Host "No status checks configured."
        break
    }
    $notCompleted = $obj.statusCheckRollup | Where-Object { $_.status -ne 'COMPLETED' }
    if ($notCompleted.Count -eq 0) { break }
    Write-Host ("Waiting for {0} checks to complete..." -f $notCompleted.Count)
    Start-Sleep -Seconds $pollSeconds
    if ((Get-Date) - $start -gt ([TimeSpan]::FromMinutes($maxMinutes))) {
        Write-Host "Timeout waiting for checks."
        exit 4
    }
}

# final fetch
$json = gh pr view $pr --repo "$repoOwner/$repoName" --json statusCheckRollup,mergeable,state --jq '.'
$obj = $json | ConvertFrom-Json
$failed = $obj.statusCheckRollup | Where-Object { $_.conclusion -ne 'SUCCESS' -and $_.conclusion -ne 'NEUTRAL' -and $_.conclusion -ne 'SKIPPED' }
if ($failed.Count -gt 0) {
    Write-Host 'Checks completed but some checks failed:'
    $failed | ForEach-Object { Write-Host ("{0}: {1}" -f $_.name, $_.conclusion) }
    exit 2
}

Write-Host 'All checks passed. Attempting merge via GH API...'
$res = gh api -X PUT /repos/$repoOwner/$repoName/pulls/$pr/merge -f merge_method=merge -f commit_title="Merge PR #$pr (automated)"
Write-Host 'Merge response:'
Write-Host $res
if ($res -match '"merged":\s*true') {
    Write-Host 'PR merged successfully.'
    exit 0
} else {
    Write-Host 'Merge command did not report success.'
    exit 3
}
