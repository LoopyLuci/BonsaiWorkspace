$path = 'Z:\Projects\BonsaiWorkspace\bonsai_autopilot.ps1'
$lines = Get-Content $path
$open=0; $close=0
for ($i=0; $i -lt $lines.Count; $i++) {
  $line = $lines[$i]
  $o = ($line.ToCharArray() | Where-Object { $_ -eq '{' }).Count
  $c = ($line.ToCharArray() | Where-Object { $_ -eq '}' }).Count
  $open += $o; $close += $c
  $diff = $open - $close
  if ($o -gt 0 -or $c -gt 0) { Write-Host ("{0,4}: +{1} -{2} => diff={3} | {4}" -f ($i+1), $o, $c, $diff, $line) }
}
Write-Host ('FINAL DIFF={0}' -f ($open-$close))
