#!/usr/bin/env bash
# Recycle smoke test — forces a model slot recycle and verifies post-recycle inference.
# Usage: bash tool_test/recycle_smoke.sh [model_id]
set -euo pipefail

MODEL_ID="${1:-}"
WS_PORT=11369
BUDDY_PORT=11420

echo "=== Bonsai Recycle Smoke Test ==="
echo "Workspace API: http://127.0.0.1:${WS_PORT}"
echo "Buddy API:     http://127.0.0.1:${BUDDY_PORT}"
echo ""

# 1. Pre-recycle health check
echo "[1] Pre-recycle health..."
curl -sf "http://127.0.0.1:${WS_PORT}/health" | jq '{ status, port }'

# 2. Force recycle
echo ""
echo "[2] Forcing recycle${MODEL_ID:+ for model: $MODEL_ID}..."
RECYCLE_BODY="{}"
if [ -n "$MODEL_ID" ]; then
  RECYCLE_BODY="{\"model_id\":\"${MODEL_ID}\"}"
fi
curl -sf -X POST "http://127.0.0.1:${WS_PORT}/v1/admin/recycle" \
  -H 'Content-Type: application/json' \
  -d "$RECYCLE_BODY" | jq '.'

# 3. Wait for recovery (poll health up to 60s)
echo ""
echo "[3] Waiting for slot recovery (up to 60s)..."
for i in $(seq 1 30); do
  sleep 2
  STATUS=$(curl -sf "http://127.0.0.1:${WS_PORT}/health" 2>/dev/null || echo '{"status":"unreachable"}')
  echo "  attempt ${i}: $(echo "$STATUS" | jq -r '.status // "unreachable"')"
  if echo "$STATUS" | grep -q '"ok"'; then
    break
  fi
done

# 4. Post-recycle Buddy inference
echo ""
echo "[4] Post-recycle inference via Buddy API..."
RESPONSE=$(curl -sf "http://127.0.0.1:${BUDDY_PORT}/v1/chat/completions" \
  -H 'Content-Type: application/json' \
  -d '{"model":"bonsai-buddy","messages":[{"role":"user","content":"Reply with only the word OK"}],"stream":false,"max_tokens":10}' \
  2>&1) || { echo "FAIL: Buddy inference request failed"; echo "$RESPONSE"; exit 1; }

CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content // empty')
if [ -z "$CONTENT" ]; then
  echo "FAIL: No content in response"
  echo "$RESPONSE" | jq '.'
  exit 1
fi

echo "  Response: $CONTENT"
echo ""
echo "=== PASS: Post-recycle inference succeeded ==="
