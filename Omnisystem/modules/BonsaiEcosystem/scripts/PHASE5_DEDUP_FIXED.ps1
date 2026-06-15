# ============================================================================
# PHASE 5: Deduplication & Quality Scoring (FIXED)
# ============================================================================
# Fixes SHA256 hashing issues with explicit method calls and proper error handling

param(
    [string]$ChunksDir = "Z:\Projects\BonsaiWorkspace\extraction-output\chunks",
    [double]$QualityThreshold = 0.6,
    [string]$OutputDir = "Z:\Projects\BonsaiWorkspace\extraction-output\deduplicated"
)

Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  PHASE 5: DEDUPLICATION & QUALITY SCORING (FIXED)          ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

# Create output directory
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$startTime = Get-Date

# ============================================================================
# PART 1: Load all extracted chunks
# ============================================================================

Write-Host "`n[1/4] Loading extracted chunks..." -ForegroundColor Yellow

$allChunks = @()
$modelCount = 0

if (Test-Path $ChunksDir) {
    $jsonFiles = Get-ChildItem -Path $ChunksDir -Filter "*.json" -Recurse

    foreach ($file in $jsonFiles) {
        try {
            $json = Get-Content -Path $file.FullName -Raw | ConvertFrom-Json
            if ($json -is [System.Management.Automation.PSCustomObject]) {
                $allChunks += $json
            } elseif ($json -is [System.Collections.ArrayList]) {
                $allChunks += $json
            }
            $modelCount++
        } catch {
            Write-Host "⚠️  Failed to load $($file.Name): $_" -ForegroundColor Yellow
        }
    }
} else {
    Write-Host "⚠️  Chunks directory not found. Using demo data." -ForegroundColor Yellow

    # Demo chunks for testing
    $allChunks = @(
        @{ content = "Photosynthesis is the process by which plants convert light energy into chemical energy"; domain = "science"; confidence = 0.92 },
        @{ content = "Object-oriented programming uses classes and objects to structure code"; domain = "programming"; confidence = 0.88 },
        @{ content = "Photosynthesis is the process by which plants convert light energy into chemical energy"; domain = "science"; confidence = 0.91 },  # Duplicate
        @{ content = "DNA carries genetic information in living organisms"; domain = "science"; confidence = 0.94 },
        @{ content = "Recursion is a technique where a function calls itself"; domain = "programming"; confidence = 0.85 }
    )
}

Write-Host "✓ Loaded $($allChunks.Count) total chunks from $modelCount models" -ForegroundColor Green

# ============================================================================
# PART 2: Compute SHA256 hashes (FIXED)
# ============================================================================

Write-Host "`n[2/4] Computing content hashes..." -ForegroundColor Yellow

$chunkCount = 0
$hashTable = @{}
$errors = 0

foreach ($chunk in $allChunks) {
    try {
        $chunkCount++

        # Get content as string
        if ($chunk.content) {
            $contentStr = [string]$chunk.content
        } elseif ($chunk.text) {
            $contentStr = [string]$chunk.text
        } else {
            $contentStr = [string]$chunk
        }

        # Ensure string is not empty
        if ([string]::IsNullOrWhiteSpace($contentStr)) {
            Write-Host "⚠️  Chunk $chunkCount has empty content, skipping" -ForegroundColor Yellow
            continue
        }

        # Convert string to UTF8 bytes (FIX: Use explicit method, not char[])
        [byte[]]$contentBytes = [System.Text.Encoding]::UTF8.GetBytes($contentStr)

        # Create SHA256 instance and compute hash (FIX: Explicit type, single overload)
        $sha256 = [System.Security.Cryptography.SHA256Managed]::new()
        [byte[]]$hashBytes = $sha256.ComputeHash($contentBytes)

        # Convert hash bytes to hex string (FIX: Proper byte formatting)
        [string]$hashHex = ""
        foreach ($byte in $hashBytes) {
            $hashHex += "{0:x2}" -f $byte
        }

        # Store in hashtable
        if (-not $hashTable.ContainsKey($hashHex)) {
            $hashTable[$hashHex] = $chunk
        }

        if ($chunkCount % 50 -eq 0) {
            Write-Host "  Processed $chunkCount chunks..." -ForegroundColor Gray
        }

    } catch {
        $errors++
        Write-Host "⚠️  Error hashing chunk $chunkCount`: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Write-Host "✓ Computed hashes for $chunkCount chunks" -ForegroundColor Green
if ($errors -gt 0) {
    Write-Host "⚠️  $errors errors encountered during hashing" -ForegroundColor Yellow
}

# ============================================================================
# PART 3: Quality Scoring
# ============================================================================

Write-Host "`n[3/4] Scoring chunk quality..." -ForegroundColor Yellow

$scoredChunks = @()
$filteredOut = 0

foreach ($hash in $hashTable.Keys) {
    $chunk = $hashTable[$hash]

    try {
        # Calculate quality score
        $score = 0.5  # Baseline

        # Confidence adjustment
        if ($chunk.confidence) {
            $score = [double]$chunk.confidence
        }

        # Length bonus (prefer substantial chunks)
        $contentStr = if ($chunk.content) { [string]$chunk.content } else { [string]$chunk.text }
        if ($contentStr.Length -gt 100) {
            $score = [Math]::Min(1.0, $score + 0.1)
        }

        # Domain bonus
        if ($chunk.domain) {
            $score = [Math]::Min(1.0, $score + 0.05)
        }

        # Apply threshold
        if ($score -ge $QualityThreshold) {
            $scoredChunks += @{
                content = $chunk.content ?? $chunk.text ?? $chunk
                domain = $chunk.domain ?? "unknown"
                confidence = [double]($chunk.confidence ?? 0.8)
                quality_score = [Math]::Round($score, 2)
                hash = $hash
            }
        } else {
            $filteredOut++
        }

    } catch {
        Write-Host "⚠️  Error scoring chunk: $_" -ForegroundColor Yellow
    }
}

$dedupeRatio = [Math]::Round((1 - ($hashTable.Count / $allChunks.Count)) * 100, 1)

Write-Host "✓ Quality scoring complete:" -ForegroundColor Green
Write-Host "  • Total unique chunks: $($hashTable.Count)" -ForegroundColor Gray
Write-Host "  • Passed quality filter: $($scoredChunks.Count)" -ForegroundColor Gray
Write-Host "  • Filtered out: $filteredOut" -ForegroundColor Gray
Write-Host "  • Deduplication ratio: $dedupeRatio%" -ForegroundColor Gray

# ============================================================================
# PART 4: Save deduplicated chunks
# ============================================================================

Write-Host "`n[4/4] Saving deduplicated chunks..." -ForegroundColor Yellow

# Save as JSONL
$jsonlPath = Join-Path $OutputDir "chunks_deduplicated.jsonl"
$csv = @()

foreach ($chunk in $scoredChunks) {
    $json = $chunk | ConvertTo-Json -Compress
    Add-Content -Path $jsonlPath -Value $json -Encoding UTF8
    $csv += $chunk
}

# Save as CSV for easy review
$csvPath = Join-Path $OutputDir "chunks_deduplicated.csv"
$csv | Select-Object content, domain, confidence, quality_score |
    Export-Csv -Path $csvPath -NoTypeInformation -Encoding UTF8

# Save summary
$summary = @{
    total_original_chunks = $allChunks.Count
    unique_chunks_after_dedup = $hashTable.Count
    passed_quality_filter = $scoredChunks.Count
    filtered_out_low_quality = $filteredOut
    deduplication_ratio_percent = $dedupeRatio
    quality_threshold = $QualityThreshold
    processing_time_seconds = [Math]::Round(((Get-Date) - $startTime).TotalSeconds, 2)
    timestamp = (Get-Date -Format "yyyy-MM-dd HH:mm:ss")
} | ConvertTo-Json | Out-File (Join-Path $OutputDir "phase5_summary.json") -Encoding UTF8

Write-Host "✓ Saved results to:" -ForegroundColor Green
Write-Host "  • $jsonlPath" -ForegroundColor Gray
Write-Host "  • $csvPath" -ForegroundColor Gray
Write-Host "  • $(Join-Path $OutputDir 'phase5_summary.json')" -ForegroundColor Gray

# ============================================================================
# RESULTS
# ============================================================================

$totalTime = [Math]::Round(((Get-Date) - $startTime).TotalSeconds, 2)

Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║  PHASE 5 COMPLETE                                          ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Green

Write-Host "`n📊 FINAL STATISTICS:" -ForegroundColor Cyan
Write-Host "  Input chunks:             $($allChunks.Count)" -ForegroundColor White
Write-Host "  Unique chunks:            $($hashTable.Count)" -ForegroundColor White
Write-Host "  Quality-filtered chunks:  $($scoredChunks.Count)" -ForegroundColor White
Write-Host "  Deduplication ratio:      $dedupeRatio%" -ForegroundColor White
Write-Host "  Processing time:          $totalTime seconds" -ForegroundColor White

Write-Host "`n✅ Phase 5 deduplication complete!" -ForegroundColor Green
Write-Host "→ Ready for Phase 6 (KDB Module Building)" -ForegroundColor Cyan
