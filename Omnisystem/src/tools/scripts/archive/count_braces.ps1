$s = Get-Content 'Z:\Projects\BonsaiWorkspace\bonsai_autopilot.ps1' -Raw
$o = ($s.ToCharArray() | Where-Object { $_ -eq '{' }).Count
$c = ($s.ToCharArray() | Where-Object { $_ -eq '}' }).Count
Write-Host ("Open:{0} Close:{1}" -f $o,$c)
