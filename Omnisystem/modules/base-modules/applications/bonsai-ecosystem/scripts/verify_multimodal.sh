#!/usr/bin/env bash
set -euo pipefail
echo "Running BonsAI multimodal verification checks..."
MODELS="$HOME/.bonsai/models"
if [ ! -d "$MODELS" ]; then
  echo "Models directory not found: $MODELS" >&2
  exit 1
fi
echo "Models:"; ls -1 "$MODELS"
echo "Done. Run 'cargo test -p bonsai-workspace' to run unit tests."