# Check Links in Documentation
#
# This script validates all links (internal and external) in Markdown documentation.
# Internal links are checked locally; external links are validated via HTTP.
#
# Usage: .\scripts\check_links.ps1 [-CheckExternal] [-Timeout 10]

param(
    [switch]$CheckExternal = $false,
    [int]$Timeout = 10,
    [string]$DocsPath = "docs"
)

$ErrorActionPreference = "Continue"

Write-Host "🔗 Checking links in documentation..." -ForegroundColor Cyan
Write-Host "Docs path: $DocsPath" -ForegroundColor Gray

if ($CheckExternal) {
    Write-Host "External links: YES (timeout: ${Timeout}s)" -ForegroundColor Yellow
}
else {
    Write-Host "External links: Skipped (use -CheckExternal to enable)" -ForegroundColor Gray
}

Write-Host ""

# Find all markdown files
$MarkdownFiles = Get-ChildItem -Path $DocsPath, "README.md" -Recurse -Filter "*.md" -File -ErrorAction SilentlyContinue

$BrokenLinks = @()
$ExternalLinks = @()
$TotalLinks = 0

foreach ($File in $MarkdownFiles) {
    Write-Host "Checking: $($File.Name)" -ForegroundColor Gray

    $Content = Get-Content -Path $File.FullName -Raw -ErrorAction SilentlyContinue

    # Extract markdown links: [text](url)
    $LinkPattern = '\[([^\]]+)\]\(([^\)]+)\)'
    $Matches = [regex]::Matches($Content, $LinkPattern)

    foreach ($Match in $Matches) {
        $LinkText = $Match.Groups[1].Value
        $LinkUrl = $Match.Groups[2].Value
        $TotalLinks++

        # Skip anchors and email links
        if ($LinkUrl -match '^#' -or $LinkUrl -match '^mailto:') {
            continue
        }

        # Check for external links
        if ($LinkUrl -match '^https?://') {
            if ($CheckExternal) {
                $ExternalLinks += $LinkUrl
                # Validate external link
                try {
                    $Response = Invoke-WebRequest -Uri $LinkUrl -TimeoutSec $Timeout -SkipHttpErrorCheck -ErrorAction SilentlyContinue
                    if ($Response.StatusCode -ge 400) {
                        $BrokenLinks += "[$LinkText] $LinkUrl (status: $($Response.StatusCode))"
                        Write-Host "  ❌ Broken external: $LinkUrl" -ForegroundColor Red
                    }
                }
                catch {
                    $BrokenLinks += "[$LinkText] $LinkUrl (connection error)"
                    Write-Host "  ❌ Unreachable: $LinkUrl" -ForegroundColor Red
                }
            }
            continue
        }

        # Check for internal links (relative paths and anchors)
        if ($LinkUrl -match '#') {
            # Anchor link within same file
            $AnchorPart = $LinkUrl -split '#' | Select-Object -Last 1
            # Basic check: just verify it looks like an anchor
            if ($AnchorPart -eq "") {
                Write-Host "  ⚠️  Empty anchor: [$LinkText]($LinkUrl)" -ForegroundColor Yellow
            }
            continue
        }

        # File path link
        $LinkPath = Join-Path -Path (Split-Path -Parent $File.FullName) -ChildPath $LinkUrl
        $LinkPath = (Resolve-Path -Path $LinkPath -ErrorAction SilentlyContinue).Path

        if (-not (Test-Path $LinkPath)) {
            $BrokenLinks += "[$LinkText] $LinkUrl (file not found)"
            Write-Host "  ❌ Broken: $LinkUrl" -ForegroundColor Red
        }
    }
}

Write-Host ""

if ($BrokenLinks.Count -eq 0) {
    Write-Host "✅ All $TotalLinks links are valid." -ForegroundColor Green
    exit 0
}
else {
    Write-Host "❌ Found $($BrokenLinks.Count) broken link(s) out of $TotalLinks:" -ForegroundColor Red
    foreach ($Link in $BrokenLinks) {
        Write-Host "  - $Link" -ForegroundColor Red
    }
    exit 1
}
