$max=60
for ($i=0; $i -lt $max; $i++) {
  $done = gh pr view 3 --json statusCheckRollup --jq 'all(.statusCheckRollup[]; .status == "COMPLETED")'
  Write-Output ("poll {0}: {1}" -f $i, $done)
  if ($done -eq 'true') { exit 0 }
  Start-Sleep -Seconds 10
}
exit 1
