#!/usr/bin/env bash
set -euo pipefail

CI_MODE=0
if [[ ${1-} == "--ci" ]]; then
  CI_MODE=1
fi

die() { echo "ERROR: $*" >&2; exit 1; }

check_cmd() {
  command -v "$1" >/dev/null 2>&1 || return 1
}

echo "[clojurewasm] Checking prerequisites..."
MISSING=()
for cmd in git java rustup cargo; do
  if ! check_cmd "$cmd"; then
    MISSING+=("$cmd")
  fi
done

if [ ${#MISSING[@]} -ne 0 ]; then
  echo "Missing prerequisites: ${MISSING[*]}"
  echo "Please install them and re-run. See runtimes/clojure-wasm/README.md for details."
  exit 1
fi

echo "rustup detected; ensuring wasm32-wasi target is available..."
if ! rustup target list --installed | grep -q "wasm32-wasi"; then
  if [ "$CI_MODE" -eq 1 ]; then
    echo "CI mode: adding wasm32-wasi target"
    rustup target add wasm32-wasi
  else
    echo "wasm32-wasi not installed. Run: rustup target add wasm32-wasi"
  fi
fi

BUILD_DIR="build/clojurewasm"
REPO_URL="https://github.com/clojurewasm/ClojureWasm.git"

if [ -d "$BUILD_DIR/.git" ]; then
  echo "[clojurewasm] Updating existing clone in $BUILD_DIR"
  (cd "$BUILD_DIR" && git fetch --all --prune && git pull --ff-only)
else
  echo "[clojurewasm] Cloning $REPO_URL into $BUILD_DIR"
  git clone --depth 1 "$REPO_URL" "$BUILD_DIR"
fi

if [ "$CI_MODE" -eq 1 ]; then
  echo "CI mode: prerequisites and clone succeeded; skipping actual build."
  exit 0
fi

echo "Looking for repository build helpers..."
if [ -f "$BUILD_DIR/build.sh" ]; then
  echo "Found build.sh; running it"
  chmod +x "$BUILD_DIR/build.sh"
  (cd "$BUILD_DIR" && ./build.sh)
elif [ -f "$BUILD_DIR/Makefile" ]; then
  echo "Found Makefile; running make"
  (cd "$BUILD_DIR" && make)
elif [ -f "$BUILD_DIR/gradlew" ]; then
  echo "Found gradlew; running ./gradlew assemble"
  (cd "$BUILD_DIR" && ./gradlew assemble)
else
  echo "No obvious build entrypoint found in $BUILD_DIR. Inspect the cloned repo and follow its README for building modules."
  echo "You can re-run with --ci from CI to only validate prerequisites."
fi

echo "Done. If a build ran, look for built Wasm artifacts under $BUILD_DIR or its subdirectories."
