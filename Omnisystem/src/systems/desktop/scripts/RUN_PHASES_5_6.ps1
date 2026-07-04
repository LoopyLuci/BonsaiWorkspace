# ============================================================================
# COMPLETE PIPELINE: Phase 5 (Deduplication) + Phase 6 (KDB Building)
# ============================================================================
# Orchestrates deduplication and KDB module creation for the full extraction

param(
    [string]$ChunksDir = "Z:\Projects\OmnisystemWorkspace\extraction-output\chunks",
    [string]$ModelsDir = "D:\Models\general",
    [double]$QualityThreshold = 0.6,
    [string]$OutputDir = "Z:\Projects\OmnisystemWorkspace\extraction-output"
)

$pipelineStart = Get-Date
$phases = @()

Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║  OMNISYSTEM KNOWLEDGE EXTRACTION PIPELINE                      ║" -ForegroundColor Cyan
Write-Host "║  Phases 5 & 6: Deduplication → KDB Module Building        ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

Write-Host "`n📍 Configuration:" -ForegroundColor Yellow
Write-Host "  Chunks directory:      $ChunksDir" -ForegroundColor Gray
Write-Host "  Models directory:      $ModelsDir" -ForegroundColor Gray
Write-Host "  Quality threshold:     $QualityThreshold" -ForegroundColor Gray
Write-Host "  Output directory:      $OutputDir" -ForegroundColor Gray

Write-Host "`n⏱️  Starting pipeline execution..." -ForegroundColor Cyan

# ============================================================================
# PHASE 5: DEDUPLICATION & QUALITY SCORING
# ============================================================================

Write-Host "`n$('='*62)" -ForegroundColor Magenta
Write-Host "PHASE 5: DEDUPLICATION & QUALITY SCORING" -ForegroundColor Magenta
Write-Host "$('='*62)" -ForegroundColor Magenta

$phase5Start = Get-Date

try {
    & "Z:\Projects\OmnisystemWorkspace\scripts\PHASE5_DEDUP_FIXED.ps1" `
        -ChunksDir $ChunksDir `
        -QualityThreshold $QualityThreshold `
        -OutputDir "$OutputDir\deduplicated"

    $phase5Duration = [Math]::Round(((Get-Date) - $phase5Start).TotalSeconds, 2)
    $phases += "Phase 5: Deduplication ✅ ($phase5Duration s)"
    Write-Host "`n✅ Phase 5 SUCCESSFUL" -ForegroundColor Green

} catch {
    $phases += "Phase 5: Deduplication ❌"
    Write-Host "`n❌ Phase 5 FAILED: $_" -ForegroundColor Red
    exit 1
}

# ============================================================================
# PHASE 6: KDB MODULE BUILDING
# ============================================================================

Write-Host "`n$('='*62)" -ForegroundColor Magenta
Write-Host "PHASE 6: KDB MODULE BUILDING" -ForegroundColor Magenta
Write-Host "$('='*62)" -ForegroundColor Magenta

$phase6Start = Get-Date

try {
    & "Z:\Projects\OmnisystemWorkspace\scripts\PHASE6_BUILD_KDB.ps1" `
        -ModelsDir $ModelsDir `
        -ChunksDir $ChunksDir `
        -DeduplicatedChunksPath "$OutputDir\deduplicated\chunks_deduplicated.jsonl" `
        -OutputDir "Z:\Projects\OmnisystemWorkspace\kdb-modules"

    $phase6Duration = [Math]::Round(((Get-Date) - $phase6Start).TotalSeconds, 2)
    $phases += "Phase 6: KDB Building ✅ ($phase6Duration s)"
    Write-Host "`n✅ Phase 6 SUCCESSFUL" -ForegroundColor Green

} catch {
    $phases += "Phase 6: KDB Building ❌"
    Write-Host "`n❌ Phase 6 FAILED: $_" -ForegroundColor Red
    exit 1
}

# ============================================================================
# PIPELINE SUMMARY
# ============================================================================

$totalDuration = [Math]::Round(((Get-Date) - $pipelineStart).TotalSeconds, 2)

Write-Host "`n╔════════════════════════════════════════════════════════════╗" -ForegroundColor Green
Write-Host "║  PIPELINE COMPLETE - ALL PHASES SUCCESSFUL                 ║" -ForegroundColor Green
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Green

Write-Host "`n📊 PHASE EXECUTION SUMMARY:" -ForegroundColor Cyan
foreach ($phase in $phases) {
    Write-Host "  $phase" -ForegroundColor White
}

Write-Host "`n⏱️  Total Pipeline Time: $totalDuration seconds" -ForegroundColor Yellow

# Check output files
$dedupFiles = @()
if (Test-Path "$OutputDir\deduplicated") {
    $dedupFiles = Get-ChildItem "$OutputDir\deduplicated"
}

$kdbModules = @()
if (Test-Path "Z:\Projects\OmnisystemWorkspace\kdb-modules") {
    $kdbModules = Get-ChildItem "Z:\Projects\OmnisystemWorkspace\kdb-modules" -Filter "*.kmod"
}

Write-Host "`n📁 OUTPUT FILES:" -ForegroundColor Cyan
Write-Host "  Deduplicated chunks:" -ForegroundColor White
foreach ($file in $dedupFiles) {
    $size = if ($file.Length) { "$([Math]::Round($file.Length/1KB, 1)) KB" } else { "-- " }
    Write-Host "    ✓ $($file.Name) ($size)" -ForegroundColor Green
}

Write-Host "`n  KDB Modules:" -ForegroundColor White
foreach ($module in $kdbModules) {
    $size = if ($module.Length) { "$([Math]::Round($module.Length/1KB, 1)) KB" } else { "0.0 KB" }
    Write-Host "    ✓ $($module.Name) ($size)" -ForegroundColor Green
}

Write-Host "`n🎯 NEXT STEPS:" -ForegroundColor Cyan
Write-Host "  1. Verify deduplicated chunks:" -ForegroundColor White
Write-Host "     Get-Content 'Z:\Projects\OmnisystemWorkspace\extraction-output\deduplicated\chunks_deduplicated.csv' | head -5" -ForegroundColor Gray
Write-Host "  2. Inspect KDB modules:" -ForegroundColor White
Write-Host "     Get-ChildItem 'Z:\Projects\OmnisystemWorkspace\kdb-modules\' -Filter '*.kmod'" -ForegroundColor Gray
Write-Host "  3. Register with Omnisystem KDB:" -ForegroundColor White
Write-Host "     omnisystem kdb register --modules Z:\Projects\OmnisystemWorkspace\kdb-modules\*.kmod" -ForegroundColor Gray

Write-Host "`n✨ Knowledge extraction pipeline complete!" -ForegroundColor Green
Write-Host "   All chunks deduplicated, quality-scored, and packaged into KDB modules." -ForegroundColor Green
