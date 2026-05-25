# BonsAI-Core Smoke Test Guide

Manual verification steps for the BonsAI-Core Phase 1 orchestrator.

## Prerequisites

- Bonsai Workspace running (`cargo tauri dev` or installed build)
- Pair token from Settings or `bonsai-config.json`
- `curl` available

Set shell variables:

```bash
BASE=http://localhost:11373
TOKEN=<your_pair_token>
AUTH="Authorization: Bearer $TOKEN"
```

## 1. Health check

```bash
curl -s $BASE/health
# Expected: {"service":"bonsai-workspace","status":"ok"}
```

## 2. Core stats

```bash
curl -s -H "$AUTH" $BASE/api/v1/core/stats
# Expected: {"adapter_loaded":false,"avg_latency_ms":0.0,"fallback_rate":0.0,"memory_entries":0,"queue_depth":0,"active_tasks":0}
```

## 3. Tool execution

```bash
# list_files
curl -s -H "$AUTH" -H "Content-Type: application/json" \
  -X POST $BASE/api/v1/tools/run \
  -d '{"tool":"list_files","params":{"path":".","recursive":false}}'
# Expected: {"ok":true,"result":"..."}

# grep_files
curl -s -H "$AUTH" -H "Content-Type: application/json" \
  -X POST $BASE/api/v1/tools/run \
  -d '{"tool":"grep_files","params":{"pattern":"fn main","path":"."}}'
# Expected: {"ok":true,"result":"N matches..."}

# run_command
curl -s -H "$AUTH" -H "Content-Type: application/json" \
  -X POST $BASE/api/v1/tools/run \
  -d '{"tool":"run_command","params":{"command":"echo hello"}}'
# Expected: {"ok":true,"result":"hello\n"}
```

## 4. Swarm submit (prompt shorthand)

```bash
curl -s -H "$AUTH" -H "Content-Type: application/json" \
  -X POST $BASE/api/v1/swarm/submit \
  -d '{"prompt":"What is 2+2?"}'
# Expected: {"ok":true,"result":"..."} with answer in result
```

## 5. Feature flags

```bash
# Read
curl -s -H "$AUTH" $BASE/api/v1/features

# Toggle a flag
curl -s -H "$AUTH" -H "Content-Type: application/json" \
  -X POST $BASE/api/v1/features \
  -d '{"sandbox_system":false}'
```

## 6. Refusal check (policy)

```bash
curl -s -H "$AUTH" -H "Content-Type: application/json" \
  -X POST $BASE/api/v1/tools/run \
  -d '{"tool":"run_command","params":{"command":"rm -rf /"}}'
# Expected: HTTP 400, {"error":"Command not allowed: rm -rf /"}
```

## 7. Data split verification

```bash
# From bonsai-workspace root
wc -l data/bonsai_core/bonsai_core_train.jsonl
wc -l data/bonsai_core/bonsai_core_val.jsonl
wc -l data/bonsai_core/bonsai_core_test.jsonl
# Expected: ~450 / ~25 / ~26 lines (90/5/5 split of ~505 examples)
```

## Pass criteria

| Check | Expected |
|-------|----------|
| `/health` | `status: ok` |
| `/api/v1/core/stats` | HTTP 200, JSON with all fields |
| `list_files` tool | HTTP 200, entries array |
| `run_command echo` | HTTP 200, stdout returned |
| `run_command rm -rf /` | HTTP 400, policy error |
| Swarm submit with `prompt` | HTTP 200, result present |
| Data files present | 3 JSONL files, ~505 total lines |
