<#
.SYNOPSIS
    Generate DreamAgent fine-tuning data for EternalWorkshop consolidation.

.DESCRIPTION
    Reads raw MemoryNodes from the Bonsai SQLite database, chunks them into
    batches, sends each batch to the teacher model (Qwen3-35B via llama-server)
    and asks it to produce a consolidated, high-value summary.
    Output: training_data/dreamagent.jsonl
    Format: {"messages": [{"role":"user","content": <nodes_json>}, {"role":"assistant","content": <consolidated_json>}]}

    Teacher must be running on port 8080 before calling this script.

.PARAMETER DbPath
    Path to the Bonsai memory database. Default: ~\.bonsai\memory.db

.PARAMETER Output
    Output JSONL path. Default: ~\.bonsai\training_export\dreamagent.jsonl

.PARAMETER TeacherUrl
    llama-server URL. Default: http://127.0.0.1:8080

.PARAMETER BatchSize
    Nodes per batch sent to teacher. Default: 20

.PARAMETER MaxBatches
    Maximum number of batches to process (0 = all). Default: 0

.EXAMPLE
    # Start teacher first, then:
    .\scripts\generate_dreamagent_data.ps1
#>

param(
    [string]$DbPath     = "$env:USERPROFILE\.bonsai\memory.db",
    [string]$Output     = "$env:USERPROFILE\.bonsai\training_export\dreamagent.jsonl",
    [string]$TeacherUrl = "http://127.0.0.1:8080",
    [int]   $BatchSize  = 20,
    [int]   $MaxBatches = 0
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Offline enforcement ────────────────────────────────────────────────────────
$env:TRANSFORMERS_OFFLINE        = "1"
$env:HF_HUB_OFFLINE              = "1"
$env:HF_DATASETS_OFFLINE         = "1"
$env:HF_HUB_DISABLE_TELEMETRY   = "1"

$SystemPrompt = @"
You are the Bonsai Memory Consolidator. You receive a JSON array of raw activity nodes from a programming session.
Your job is to:
1. Merge related information into concise, high-value insights.
2. Remove duplicate or trivial entries (repeated keystrokes, minor cursor moves).
3. Preserve important facts: errors fixed, decisions made, code patterns learned, architectural choices.
4. Output ONLY a JSON array of consolidated MemoryNode objects with these fields:
   { "id": string, "node_type": string, "source": string, "content": string, "tags": [string] }
Be concise. Prefer one rich insight over five shallow ones.
"@

function Test-TeacherHealth {
    try {
        $r = Invoke-WebRequest -Uri "$TeacherUrl/health" -TimeoutSec 5 -UseBasicParsing -ErrorAction Stop
        return $r.StatusCode -eq 200
    } catch {
        return $false
    }
}

function Invoke-Teacher {
    param([string]$UserContent)

    $body = @{
        messages    = @(
            @{ role = "system"; content = $SystemPrompt },
            @{ role = "user";   content = "Consolidate these activity nodes:\n\n$UserContent" }
        )
        temperature = 0.1
        max_tokens  = 2048
    } | ConvertTo-Json -Depth 10

    $resp = Invoke-RestMethod -Uri "$TeacherUrl/v1/chat/completions" `
        -Method POST `
        -Body $body `
        -ContentType "application/json" `
        -TimeoutSec 120

    return $resp.choices[0].message.content
}

function Get-MemoryNodes {
    param([string]$Db)
    if (-not (Test-Path $Db)) {
        Write-Warning "Database not found at $Db. Run the Bonsai app first to populate memory nodes."
        return @()
    }
    # Use Python + sqlite3 (stdlib) — no extra deps needed
    $py = @"
import sqlite3, json, sys
db = sys.argv[1]
conn = sqlite3.connect(db)
rows = conn.execute(
    "SELECT id, timestamp_ms, node_type, source, content, tags FROM memory_nodes ORDER BY timestamp_ms ASC"
).fetchall()
nodes = [
    {"id": r[0], "timestamp_ms": r[1], "node_type": r[2], "source": r[3], "content": r[4], "tags": json.loads(r[5] or "[]")}
    for r in rows
]
print(json.dumps(nodes))
"@
    $json = python -c $py $Db
    return $json | ConvertFrom-Json
}

# ── Main ────────────────────────────────────────────────────────────────────────

Write-Host "[dreamagent] Checking teacher health at $TeacherUrl…"
if (-not (Test-TeacherHealth)) {
    Write-Error "Teacher not reachable at $TeacherUrl. Start llama-server first:`n  llama-server -m D:/Models/general/Qwen3-35B-A22B-Q4_K_M.gguf -ngl 99 --port 8080"
}
Write-Host "[dreamagent] Teacher ready."

Write-Host "[dreamagent] Reading memory nodes from $DbPath"
$nodes = Get-MemoryNodes -Db $DbPath
if ($nodes.Count -eq 0) {
    Write-Warning "No memory nodes found. The output file will be empty."
    New-Item -ItemType Directory -Force -Path (Split-Path $Output) | Out-Null
    "" | Set-Content $Output
    exit 0
}
Write-Host "[dreamagent] Loaded $($nodes.Count) nodes."

# Split into batches
$batches = [System.Collections.Generic.List[object[]]]::new()
for ($i = 0; $i -lt $nodes.Count; $i += $BatchSize) {
    $end  = [Math]::Min($i + $BatchSize, $nodes.Count) - 1
    $batches.Add($nodes[$i..$end])
}

$limit = if ($MaxBatches -gt 0) { [Math]::Min($MaxBatches, $batches.Count) } else { $batches.Count }
Write-Host "[dreamagent] Processing $limit of $($batches.Count) batches (batch_size=$BatchSize)"

New-Item -ItemType Directory -Force -Path (Split-Path $Output) | Out-Null
$written = 0

for ($b = 0; $b -lt $limit; $b++) {
    $batch   = $batches[$b]
    $batchJson = $batch | ConvertTo-Json -Depth 10

    Write-Host "  [batch $($b+1)/$limit] $($batch.Count) nodes…" -NoNewline

    try {
        $response = Invoke-Teacher -UserContent $batchJson

        # Extract JSON array from response (model may prepend prose)
        $arrayStart = $response.IndexOf('[')
        $arrayEnd   = $response.LastIndexOf(']')
        if ($arrayStart -ge 0 -and $arrayEnd -gt $arrayStart) {
            $jsonPart = $response.Substring($arrayStart, $arrayEnd - $arrayStart + 1)
            # Validate it parses
            $null = $jsonPart | ConvertFrom-Json
        } else {
            $jsonPart = "[]"
        }

        $record = @{
            messages = @(
                @{ role = "user";      content = "Consolidate these activity nodes:`n`n$batchJson" },
                @{ role = "assistant"; content = $jsonPart }
            )
        }
        $record | ConvertTo-Json -Depth 10 -Compress | Add-Content -Path $Output -Encoding UTF8
        $written++
        Write-Host " OK ($($jsonPart.Length) chars)"
    } catch {
        Write-Host " FAILED: $_"
    }

    Start-Sleep -Milliseconds 200
}

Write-Host ""
Write-Host "[dreamagent] Wrote $written training examples → $Output"
Write-Host "[dreamagent] Fine-tune with:"
Write-Host "  just finetune-sft data=`"$Output`" output=`"`$env:USERPROFILE\.bonsai\adapters\bonsai-dreamagent-v1`""
