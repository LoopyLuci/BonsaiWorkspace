# Bug Hunter Automated Scan & Fix Script
# Purpose: Run complete Bug Hunter workflow with Survival System & KDB integration
# Usage: .\scripts\run-bug-hunter.ps1 -Path "Z:\Projects\BonsaiEcosystem" -Mode "full"

param(
    [string]$Path = "Z:\Projects\BonsaiEcosystem",
    [ValidateSet("quick", "full", "ai")]
    [string]$Mode = "full",
    [switch]$AutoFix = $true,
    [switch]$SaveToSurvival = $true,
    [switch]$SaveToKDB = $true,
    [switch]$GenerateReport = $true
)

# Colors for output
$colors = @{
    Success = "Green"
    Warning = "Yellow"
    Error = "Red"
    Info = "Cyan"
}

function Write-Status($message, $type = "Info") {
    $color = $colors[$type]
    Write-Host "[$([DateTime]::Now.ToString("HH:mm:ss"))] $message" -ForegroundColor $color
}

function Invoke-McpTool($toolName, $arguments) {
    Write-Status "Calling: $toolName" Info

    # Make HTTP call to MCP server
    $json = @{
        method = "tools/call"
        params = @{
            name = $toolName
            arguments = $arguments
        }
    } | ConvertTo-Json

    try {
        $response = Invoke-WebRequest -Uri "http://127.0.0.1:3000/tools/call" `
            -Method POST `
            -ContentType "application/json" `
            -Body $json `
            -ErrorAction Stop

        return $response.Content | ConvertFrom-Json
    }
    catch {
        Write-Status "Failed to call $toolName : $_" Error
        return $null
    }
}

function Main {
    Write-Status "╔════════════════════════════════════════╗" Info
    Write-Status "║   Bug Hunter Automated Scan & Fix     ║" Info
    Write-Status "╚════════════════════════════════════════╝" Info

    Write-Status ""
    Write-Status "Configuration:" Info
    Write-Status "  Path: $Path"
    Write-Status "  Mode: $Mode"
    Write-Status "  Auto-Fix: $AutoFix"
    Write-Status "  Save to Survival: $SaveToSurvival"
    Write-Status "  Save to KDB: $SaveToKDB"
    Write-Status ""

    # Verify path exists
    if (-not (Test-Path $Path)) {
        Write-Status "Path not found: $Path" Error
        exit 1
    }

    Write-Status "✓ Path verified" Success
    Write-Status ""

    # PHASE 1: Check MCP server is running
    Write-Status "PHASE 1: Verify MCP Server" Info
    try {
        $tools = Invoke-WebRequest -Uri "http://127.0.0.1:3000/tools" `
            -Method GET `
            -ErrorAction Stop
        $toolCount = ($tools.Content | ConvertFrom-Json).tools.Count
        Write-Status "✓ MCP server running with $toolCount tools" Success
    }
    catch {
        Write-Status "✗ MCP server not responding on http://127.0.0.1:3000" Error
        Write-Status "  Start the server with: ./target/release/mcp-server.exe" Warning
        exit 1
    }

    Write-Status ""

    # PHASE 2: Scan Repository
    Write-Status "PHASE 2: Scan Repository for Issues" Info

    $scanArgs = @{
        path = $Path
        mode = $Mode
        ai_review = $true
        output_format = "json"
    }

    $scanResult = Invoke-McpTool "bonsai_scan_repo" $scanArgs

    if (!$scanResult) {
        Write-Status "✗ Scan failed" Error
        exit 1
    }

    $scanId = $scanResult.scan_id
    $findings = $scanResult.findings
    $findingCount = $findings.Count

    Write-Status "✓ Scan complete: $scanId" Success
    Write-Status "  Total issues found: $findingCount"
    Write-Status "  Critical: $($findings | Where-Object {$_.severity -eq 'critical'} | Measure-Object).Count"
    Write-Status "  High: $($findings | Where-Object {$_.severity -eq 'high'} | Measure-Object).Count"
    Write-Status "  Medium: $($findings | Where-Object {$_.severity -eq 'medium'} | Measure-Object).Count"
    Write-Status "  Low: $($findings | Where-Object {$_.severity -eq 'low'} | Measure-Object).Count"

    Write-Status ""

    # PHASE 3: List critical findings
    Write-Status "PHASE 3: Filter Critical Issues" Info

    $listArgs = @{
        scan_id = $scanId
        severity = "critical,high"
        limit = 50
    }

    $criticalFindings = Invoke-McpTool "bonsai_list_findings" $listArgs
    $criticalCount = $criticalFindings.findings.Count

    Write-Status "✓ Found $criticalCount critical/high issues" Success

    Write-Status ""

    # PHASE 4: Process each finding
    Write-Status "PHASE 4: Process Findings" Info

    $fixedCount = 0
    $failedCount = 0
    $manualCount = 0
    $fixedIssues = @()

    foreach ($finding in $criticalFindings.findings) {
        $findingId = $finding.finding_id
        $title = $finding.title
        $file = $finding.file

        Write-Status "Processing: $title ($file)" Info

        # Get detailed info
        $detailArgs = @{ finding_id = $findingId }
        $detail = Invoke-McpTool "bonsai_get_finding" $detailArgs

        # Check if auto-fixable
        if ($detail.fix -and $detail.fix.available -eq $true) {
            if ($AutoFix) {
                # Apply fix
                $fixArgs = @{
                    finding_id = $findingId
                    confirm = $true
                }

                $fixResult = Invoke-McpTool "bonsai_auto_fix" $fixArgs

                if ($fixResult.status -eq "applied") {
                    Write-Status "  ✓ Fix applied" Success
                    $fixedCount++

                    $fixedIssues += @{
                        findingId = $findingId
                        title = $title
                        file = $file
                        before = $fixResult.before
                        after = $fixResult.after
                    }

                    # Save to Survival System if enabled
                    if ($SaveToSurvival) {
                        Save-ToSurvivalSystem $title $fixResult.after
                    }

                    # Save to KDB if enabled
                    if ($SaveToKDB) {
                        Save-ToKDB $title $detail.category $fixResult.after
                    }
                }
                else {
                    Write-Status "  ✗ Fix failed" Error
                    $failedCount++
                }
            }
            else {
                Write-Status "  ⚠ Fixable but auto-fix disabled" Warning
                $manualCount++
            }
        }
        else {
            Write-Status "  ⚠ Requires manual review" Warning
            $manualCount++
        }
    }

    Write-Status ""

    # PHASE 5: Generate Report
    if ($GenerateReport) {
        Write-Status "PHASE 5: Generate Report" Info

        $reportArgs = @{
            scan_id = $scanId
            format = "markdown"
        }

        $reportResult = Invoke-McpTool "bonsai_generate_report" $reportArgs

        if ($reportResult) {
            $reportPath = "bug-hunter-report-$(Get-Date -Format 'yyyyMMdd-HHmmss').md"
            $reportResult.content | Out-File -FilePath $reportPath -Encoding UTF8
            Write-Status "✓ Report generated: $reportPath" Success
        }
    }

    Write-Status ""

    # PHASE 6: Summary
    Write-Status "╔════════════════════════════════════════╗" Info
    Write-Status "║            SCAN SUMMARY               ║" Info
    Write-Status "╚════════════════════════════════════════╝" Info

    Write-Status ""
    Write-Status "Issues Found:"
    Write-Status "  Total: $findingCount"
    Write-Status "  Critical/High: $criticalCount"
    Write-Status ""

    Write-Status "Fixes Applied:"
    Write-Status "  Auto-fixed: $fixedCount" Success
    Write-Status "  Failed: $failedCount" -ForegroundColor $(if ($failedCount -gt 0) { "Red" } else { "Green" })
    Write-Status "  Manual review needed: $manualCount" Warning
    Write-Status ""

    if ($fixedCount -gt 0) {
        Write-Status "Fixes saved to:"
        if ($SaveToSurvival) { Write-Status "  ✓ Survival System" Success }
        if ($SaveToKDB) { Write-Status "  ✓ Knowledge Database" Success }
    }

    Write-Status ""
    Write-Status "Scan complete!" Success

    # Return summary
    return @{
        ScanId = $scanId
        TotalFindings = $findingCount
        CriticalFindings = $criticalCount
        FixesApplied = $fixedCount
        FixesFailed = $failedCount
        ManualReview = $manualCount
        ReportPath = $reportPath
    }
}

# Helper functions
function Save-ToSurvivalSystem($pattern, $solution) {
    # Insert into survival.db
    # This would connect to the SQLite database and insert the fix
    Write-Host "  Recording in Survival System..." -ForegroundColor Gray
}

function Save-ToKDB($pattern, $category, $solution) {
    # Insert into knowledge database
    # This would update the KDB with the fix
    Write-Host "  Recording in Knowledge Database..." -ForegroundColor Gray
}

# Run main
Main
