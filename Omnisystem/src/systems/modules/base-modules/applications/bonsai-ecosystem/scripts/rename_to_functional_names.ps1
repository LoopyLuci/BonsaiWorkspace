# Comprehensive Functional Naming Rename
# Removes ALL branding prefixes and applies descriptive, functional names
# Handles: bonsai-*, build-*, usos-*, uosc-*
# Result: Components named by what they do, not what brand they belong to

param(
    [switch]$DryRun = $false,
    [switch]$VerifyOnly = $false
)

$ErrorActionPreference = "Stop"

Write-Host "═════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "Functional Naming Refactor - Remove All Branding Prefixes" -ForegroundColor Cyan
Write-Host "═════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# Comprehensive mapping: old → new (functional names)
$renameMap = @{
    # Kernel
    "kernel" = "kernel"
    "kernel" = "kernel"

    # Crates (bonsai-* → functional)
    "driver-converter" = "driver-converter"
    "test-orchestrator" = "test-orchestrator"
    "validation-mesh" = "validation-mesh"
    "lang-registry" = "lang-registry"
    "audit-log" = "audit-log"
    "discovery" = "discovery"
    "sandbox" = "sandbox"
    "ai-advisor" = "ai-advisor"
    "p2p-core" = "p2p-core"
    "p2p-crypto" = "p2p-crypto"
    "p2p-identity" = "p2p-identity"
    "inc-compile" = "inc-compile"
    "container" = "container"
    "msg-core" = "msg-core"
    "msg-smtp" = "msg-smtp"
    "msg-imap" = "msg-imap"
    "msg-p2p" = "msg-p2p"
    "msg-server" = "msg-server"
    "mcp-server" = "mcp-server"
    "android-runtime" = "android-runtime"
    "compiler-cache" = "compiler-cache"

    # Build tool
    "build" = "build"
    "build.toml" = "build.toml"
}

Write-Host "Mapping: $($renameMap.Count) components" -ForegroundColor Yellow
$renameMap.GetEnumerator() | ForEach-Object { Write-Host "  $($_.Key) → $($_.Value)" -ForegroundColor Gray }
Write-Host ""

# Phase 0: Audit
Write-Host "Phase 0: Audit" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────" -ForegroundColor Yellow

$excludePatterns = @('\.venv*', '\.git', 'node_modules', 'target', 'dist', '__pycache__')
$allFiles = @()

# Find all text files
$textExtensions = @('*.ti', '*.rs', '*.md', '*.toml', '*.json', '*.sh', '*.ps1', '*.txt', '*.yml', '*.yaml', 'Makefile', 'Cargo.lock')
foreach ($ext in $textExtensions) {
    $allFiles += Get-ChildItem -Path . -Recurse -Filter $ext -ErrorAction SilentlyContinue |
                 Where-Object { $_.FullName -notmatch ($excludePatterns -join '|') }
}

Write-Host "Files to scan: $($allFiles.Count)" -ForegroundColor Green

# Find all occurrences
$occurrences = @{}
foreach ($file in $allFiles) {
    try {
        $content = Get-Content -Path $file.FullName -Raw -ErrorAction SilentlyContinue
        if ($null -eq $content) { continue }

        $fileHasMatch = $false
        foreach ($old in $renameMap.Keys) {
            # Case-sensitive word boundary matching
            $pattern = "\b" + [regex]::Escape($old) + "\b"
            if ($content -match $pattern) {
                if (-not $occurrences.ContainsKey($file.FullName)) {
                    $occurrences[$file.FullName] = 0
                }
                $occurrences[$file.FullName] += ([regex]::Matches($content, $pattern)).Count
                $fileHasMatch = $true
            }
        }

        if ($fileHasMatch) {
            $relPath = $file.FullName.Substring((Get-Location).Path.Length + 1)
            Write-Host "  ✓ $relPath - $($occurrences[$file.FullName]) occurrence(s)" -ForegroundColor Green
        }
    } catch {
        # Skip files that can't be read
    }
}

$totalOccurrences = ($occurrences.Values | Measure-Object -Sum).Sum
Write-Host ""
Write-Host "Total files with matches: $($occurrences.Count)" -ForegroundColor Green
Write-Host "Total occurrences: $totalOccurrences" -ForegroundColor Green
Write-Host ""

if ($VerifyOnly) {
    Write-Host "Audit complete. Exiting." -ForegroundColor Green
    exit 0
}

if ($DryRun) {
    Write-Host "DRY-RUN MODE - No actual changes will be made." -ForegroundColor Yellow
    Write-Host ""
}

# Phase 1: Directory & File Renames
Write-Host "Phase 1: Directory & File Renames" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────" -ForegroundColor Yellow

$dirsRenamed = 0
$filesRenamed = 0

# Rename directories (must do parents first)
$dirsToRename = Get-ChildItem -Path . -Recurse -Directory -ErrorAction SilentlyContinue |
                Where-Object {
                    $name = $_.Name
                    $renameMap.Keys | Where-Object { $name -eq $_ }
                } | Sort-Object -Property FullName -Descending

foreach ($dir in $dirsToRename) {
    $oldName = $dir.Name
    $newName = $renameMap[$oldName]

    if ($newName -ne $oldName) {
        $newPath = Join-Path -Path $dir.Parent.FullName -ChildPath $newName

        if (-not $DryRun) {
            Move-Item -Path $dir.FullName -Destination $newPath -Force
            Write-Host "  ✓ Renamed directory: $oldName → $newName" -ForegroundColor Green
            $dirsRenamed++
        } else {
            Write-Host "  [DRY-RUN] Would rename: $oldName → $newName" -ForegroundColor Cyan
        }
    }
}

# Rename files
$filesToRename = Get-ChildItem -Path . -Recurse -File -ErrorAction SilentlyContinue |
                 Where-Object {
                     $name = $_.BaseName
                     $renameMap.Keys | Where-Object { $name -eq $_ }
                 }

foreach ($file in $filesToRename) {
    $oldName = $file.Name
    $baseName = $file.BaseName
    $extension = $file.Extension
    $newName = $renameMap[$baseName] + $extension

    if ($newName -ne $oldName) {
        $newPath = Join-Path -Path $file.Directory.FullName -ChildPath $newName

        if (-not $DryRun) {
            Move-Item -Path $file.FullName -Destination $newPath -Force
            Write-Host "  ✓ Renamed file: $oldName → $newName" -ForegroundColor Green
            $filesRenamed++
        } else {
            Write-Host "  [DRY-RUN] Would rename file: $oldName → $newName" -ForegroundColor Cyan
        }
    }
}

Write-Host ""
Write-Host "Phase 1 Summary: $dirsRenamed directories, $filesRenamed files renamed" -ForegroundColor Green
Write-Host ""

# Phase 2: Content Replacements
Write-Host "Phase 2: Content Replacement" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────" -ForegroundColor Yellow

$filesUpdated = 0
$totalReplacements = 0

# Refresh file list after directory renames
$allFiles = @()
foreach ($ext in $textExtensions) {
    $allFiles += Get-ChildItem -Path . -Recurse -Filter $ext -ErrorAction SilentlyContinue |
                 Where-Object { $_.FullName -notmatch ($excludePatterns -join '|') }
}

foreach ($file in $allFiles) {
    try {
        $content = Get-Content -Path $file.FullName -Raw -ErrorAction SilentlyContinue
        if ($null -eq $content) { continue }

        $originalContent = $content
        $fileReplacements = 0

        # Apply all replacements
        foreach ($old in $renameMap.Keys) {
            $new = $renameMap[$old]

            # Word boundary replacements
            $pattern = "\b" + [regex]::Escape($old) + "\b"
            $newContent = [regex]::Replace($content, $pattern, $new)
            $matches = [regex]::Matches($content, $pattern).Count

            if ($matches -gt 0) {
                $content = $newContent
                $fileReplacements += $matches
                $totalReplacements += $matches
            }
        }

        if ($content -ne $originalContent) {
            if (-not $DryRun) {
                Set-Content -Path $file.FullName -Value $content -NoNewline -Force
                $filesUpdated++
                Write-Host "  ✓ Updated: $($file.Name) ($fileReplacements replacements)" -ForegroundColor Green
            } else {
                Write-Host "  [DRY-RUN] Would update: $($file.Name) ($fileReplacements replacements)" -ForegroundColor Cyan
            }
        }
    } catch {
        # Skip files that can't be written
    }
}

Write-Host ""
Write-Host "Phase 2 Summary: Updated $filesUpdated files with $totalReplacements replacements" -ForegroundColor Green
Write-Host ""

# Phase 3: Verification
Write-Host "Phase 3: Verification" -ForegroundColor Yellow
Write-Host "─────────────────────────────────────────────────────────" -ForegroundColor Yellow

$residualBrand = @()

foreach ($file in $allFiles) {
    try {
        $content = Get-Content -Path $file.FullName -Raw -ErrorAction SilentlyContinue
        if ($null -eq $content) { continue }

        # Look for remaining brand prefixes
        if ($content -match '\bbonsai-|\bomni-|\busos-|\buosc-') {
            $lines = @()
            $lineNum = 0
            foreach ($line in (Get-Content -Path $file.FullName -ErrorAction SilentlyContinue)) {
                $lineNum++
                if ($line -match '\bbonsai-|\bomni-|\busos-|\buosc-') {
                    $lines += "$($lineNum): $line"
                }
            }

            if ($lines.Count -gt 0) {
                $residualBrand += @{
                    File = $file.FullName.Substring((Get-Location).Path.Length + 1)
                    Lines = $lines
                }
            }
        }
    } catch {
        # Skip
    }
}

if ($residualBrand.Count -eq 0) {
    Write-Host "  ✓ Verification passed: No residual brand prefixes found" -ForegroundColor Green
} else {
    Write-Host "  ⚠ Found $($residualBrand.Count) files with potential residual references:" -ForegroundColor Yellow
    foreach ($item in $residualBrand | Select-Object -First 10) {
        Write-Host "    - $($item.File)" -ForegroundColor Yellow
        foreach ($line in $item.Lines | Select-Object -First 3) {
            Write-Host "      $line" -ForegroundColor Gray
        }
    }
}

Write-Host ""
Write-Host "═════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "Rename Complete" -ForegroundColor Cyan
Write-Host "═════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

if ($DryRun) {
    Write-Host "[DRY-RUN MODE] No actual changes were made." -ForegroundColor Yellow
    Write-Host "Run without -DryRun to apply changes." -ForegroundColor Yellow
} else {
    Write-Host "Summary:" -ForegroundColor Green
    Write-Host "  • Directories renamed: $dirsRenamed" -ForegroundColor Green
    Write-Host "  • Files renamed: $filesRenamed" -ForegroundColor Green
    Write-Host "  • Files updated: $filesUpdated" -ForegroundColor Green
    Write-Host "  • Total replacements: ~$totalReplacements" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Green
    Write-Host "  1. Review changes: git diff" -ForegroundColor Green
    Write-Host "  2. Build: cargo build" -ForegroundColor Green
    Write-Host "  3. Test: cargo test" -ForegroundColor Green
    Write-Host "  4. Commit with message about functional naming" -ForegroundColor Green
}

Write-Host ""
