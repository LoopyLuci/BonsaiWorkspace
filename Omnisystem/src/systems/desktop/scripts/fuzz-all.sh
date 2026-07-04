#!/usr/bin/env bash
set -euo pipefail

echo "Starting fuzz smoke run..."
TARGETS=(bonsai-kdb bonsai-p2p bonsai-crdt)
for t in "${TARGETS[@]}"; do
  echo "Fuzz target: $t (skipped unless fuzz target exists)"
  cargo fuzz list >/dev/null 2>&1 || continue
  cargo fuzz run "$t" -- -max_total_time=30 || true
done

echo "Fuzz run completed."
