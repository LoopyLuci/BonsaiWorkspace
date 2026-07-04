#!/usr/bin/env bash
set -euo pipefail

# Smoke test for Bonsai Workspace swarm + RAG behavior.
# Usage:
#   scripts/smoke-test-swarm.sh
#   BUDDY_PORT=11420 API_PORT=11369 scripts/smoke-test-swarm.sh

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_DIR="${ROOT_DIR}/bonsai-workspace"

API_HOST="${API_HOST:-127.0.0.1}"
API_PORT="${API_PORT:-11369}"
BUDDY_HOST="${BUDDY_HOST:-127.0.0.1}"
BUDDY_PORT="${BUDDY_PORT:-11420}"

API_HEALTH_URL="http://${API_HOST}:${API_PORT}/health"
BUDDY_HEALTH_URL="http://${BUDDY_HOST}:${BUDDY_PORT}/health"
BUDDY_CHAT_URL="http://${BUDDY_HOST}:${BUDDY_PORT}/v1/chat/completions"

WAIT_TIMEOUT_SEC="${WAIT_TIMEOUT_SEC:-240}"
LOG_FILE="${ROOT_DIR}/tool_test/launcher/smoke-swarm-latest.log"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

APP_PID=""
PASS_COUNT=0
FAIL_COUNT=0

say() {
  printf "${BLUE}[%s]${NC} %s\n" "smoke" "$*"
}

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf "${GREEN}[PASS]${NC} %s\n" "$*"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf "${RED}[FAIL]${NC} %s\n" "$*"
}

warn() {
  printf "${YELLOW}[WARN]${NC} %s\n" "$*"
}

cleanup() {
  if [[ -n "${APP_PID}" ]] && kill -0 "${APP_PID}" >/dev/null 2>&1; then
    say "Stopping tauri dev (pid ${APP_PID})"
    kill "${APP_PID}" >/dev/null 2>&1 || true
    wait "${APP_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

extract_content() {
  local json="$1"
  if command -v jq >/dev/null 2>&1; then
    printf '%s' "$json" | jq -r '.choices[0].message.content // ""'
  else
    printf '%s' "$json" | sed -n 's/.*"content":"\([^"]*\)".*/\1/p' | sed 's/\\n/\n/g' | sed 's/\\"/"/g'
  fi
}

wait_for_url() {
  local url="$1"
  local label="$2"
  local elapsed=0

  while (( elapsed < WAIT_TIMEOUT_SEC )); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      pass "${label} is healthy: ${url}"
      return 0
    fi
    sleep 2
    elapsed=$((elapsed + 2))
  done

  fail "${label} did not become healthy within ${WAIT_TIMEOUT_SEC}s (${url})"
  return 1
}

smoke_swarm_sequential_gate() {
  say "Submitting sequential_gate smoke request via Buddy API"

  local payload
  payload=$(cat <<'JSON'
{
  "model": "bonsai-buddy",
  "stream": false,
  "max_tokens": 600,
  "messages": [
    {
      "role": "system",
      "content": "Smoke test mode. Prefer multi-agent swarm if available and use sequential_gate strategy. Always append <worker_assessment>{\"confidence\":90,\"evidence_sources\":[\"smoke-script\"],\"gaps\":[]}</worker_assessment> at the end of your response."
    },
    {
      "role": "user",
      "content": "Provide two merged robustness improvements and one verification hint. Keep it short."
    }
  ]
}
JSON
)

  local response
  if ! response=$(curl -fsS -X POST "$BUDDY_CHAT_URL" -H 'content-type: application/json' -d "$payload"); then
    fail "Swarm submission request failed"
    return
  fi

  local content
  content="$(extract_content "$response")"

  if [[ -z "$content" ]]; then
    fail "Swarm submission response content was empty"
    return
  fi

  if grep -q '<worker_assessment>' <<<"$content"; then
    pass "Swarm-style response contains <worker_assessment> block"
  else
    fail "Swarm-style response missing <worker_assessment> block"
  fi

  if grep -Eiq 'sequential_gate|sequential|tier|gate' <<<"$content"; then
    pass "Sequential strategy evidence found in response text"
  else
    warn "No explicit sequential keyword found; swarm mode may be unavailable in this endpoint"
  fi
}

smoke_rag_search() {
  say "Submitting RAG smoke request (search_knowledge prompt)"

  local payload
  payload=$(cat <<'JSON'
{
  "model": "bonsai-buddy",
  "stream": false,
  "max_tokens": 600,
  "messages": [
    {
      "role": "user",
      "content": "Use search_knowledge for query: Bonsai Buddy API listening on http://127.0.0.1:11420. Return the best matching file path and one-sentence summary."
    }
  ]
}
JSON
)

  local response
  if ! response=$(curl -fsS -X POST "$BUDDY_CHAT_URL" -H 'content-type: application/json' -d "$payload"); then
    fail "RAG search request failed"
    return
  fi

  local content
  content="$(extract_content "$response")"

  if [[ -z "$content" ]]; then
    fail "RAG search response content was empty"
    return
  fi

  if grep -Eiq '(buddy_api_server\.rs|README\.md|user_manual\.md|/src/|\.rs|\.md)' <<<"$content"; then
    pass "RAG search returned a plausible file/path reference"
  else
    fail "RAG search did not return a recognizable file/path reference"
  fi
}

main() {
  require_cmd curl
  require_cmd npm

  mkdir -p "$(dirname "$LOG_FILE")"

  say "Starting app in dev mode"
  (
    cd "$APP_DIR"
    npm run tauri dev
  ) >"$LOG_FILE" 2>&1 &
  APP_PID=$!
  say "tauri dev pid=${APP_PID}, log=${LOG_FILE}"

  wait_for_url "$API_HEALTH_URL" "Core API" || true
  wait_for_url "$BUDDY_HEALTH_URL" "Buddy API" || true

  if ! curl -fsS "$BUDDY_HEALTH_URL" >/dev/null 2>&1; then
    fail "Buddy API unavailable; skipping swarm and RAG request checks"
  else
    smoke_swarm_sequential_gate
    smoke_rag_search
  fi

  echo
  if (( FAIL_COUNT == 0 )); then
    printf "${GREEN}Smoke test complete: %d PASS, %d FAIL${NC}\n" "$PASS_COUNT" "$FAIL_COUNT"
    exit 0
  else
    printf "${RED}Smoke test complete: %d PASS, %d FAIL${NC}\n" "$PASS_COUNT" "$FAIL_COUNT"
    exit 1
  fi
}

main "$@"
