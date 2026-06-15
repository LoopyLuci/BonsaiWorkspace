$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile('Z:\Projects\BonsaiWorkspace\bonsai_autopilot.ps1',[ref]$null,[ref]$errors)
if ($errors) { $errors | Format-List } else { Write-Host 'No parser errors via AST parser' }
