#!/usr/bin/env pwsh
# Live Build Monitor - Shows everything as it happens

$buildLogFile = "C:\Users\limpi\AppData\Local\Temp\claude\z--Projects-BonsaiWorkspace\c7ae2a7a-5206-469e-8d6b-97fc5255ee90\tasks\bnolehqo3.output"

Write-Host @"
════════════════════════════════════════════════════════════════════════════════
🖥️  LIVE BUILD MONITOR - BONSAI ECOSYSTEM GPU TRAINING
════════════════════════════════════════════════════════════════════════════════

Watching: $buildLogFile

This will show you EVERYTHING as it happens:
  ✓ Kernel compilation
  ✓ IDE building
  ✓ Data preparation
  ✓ GPU TRAINING with live loss values
  ✓ Model conversion

Press Ctrl+C to stop watching (build continues in background)

════════════════════════════════════════════════════════════════════════════════

"@

$lastLineCount = 0

while ($true) {
    if (Test-Path $buildLogFile) {
        $currentContent = Get-Content $buildLogFile -ErrorAction SilentlyContinue

        if ($currentContent) {
            $lines = @($currentContent)
            $currentLineCount = $lines.Count

            # Show new lines
            if ($currentLineCount -gt $lastLineCount) {
                $newLines = $lines[($lastLineCount)..($currentLineCount - 1)]

                foreach ($line in $newLines) {
                    # Color code important messages
                    if ($line -like "*Python*" -or $line -like "*PHASE*") {
                        Write-Host $line -ForegroundColor Green
                    }
                    elseif ($line -like "*loss=*" -or $line -like "*Step*") {
                        Write-Host $line -ForegroundColor Yellow
                    }
                    elseif ($line -like "*complete*" -or $line -like "*✅*") {
                        Write-Host $line -ForegroundColor Green
                    }
                    elseif ($line -like "*error*" -or $line -like "*❌*" -or $line -like "*failed*") {
                        Write-Host $line -ForegroundColor Red
                    }
                    else {
                        Write-Host $line
                    }
                }

                $lastLineCount = $currentLineCount
            }
        }
    }
    else {
        Write-Host "Waiting for build to start..." -ForegroundColor Yellow
    }

    Start-Sleep -Milliseconds 500
}
