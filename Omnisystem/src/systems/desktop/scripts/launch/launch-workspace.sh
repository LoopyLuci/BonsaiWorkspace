#!/usr/bin/env bash
# Launch Omnisystem Workspace (IDE only)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

EXE="$WORKSPACE_ROOT/omnisystem-workspace/src-tauri/target/release/omnisystem-workspace"
if [ ! -f "$EXE" ]; then
    echo "Executable not found at: $EXE"
    echo "Run 'just build' or 'cd omnisystem-workspace/src && npx tauri build' first."
    exit 1
fi

exec "$EXE" --mode workspace "$@"
