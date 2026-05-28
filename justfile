# Bonsai Workspace — common tasks
# Install: cargo install just
# Usage:   just <recipe>

workspace_root := justfile_directory()

# List available recipes
default:
    @just --list

# ── Build ─────────────────────────────────────────────────────────────────────

# Build the full desktop app (Tauri release)
build:
    powershell -NoProfile -ExecutionPolicy Bypass \
        -File "{{workspace_root}}/scripts/build/BonsaiExeLauncherBuilder.ps1"

# Build only the survival watchdog binary
build-watchdog:
    powershell -NoProfile -ExecutionPolicy Bypass \
        -File "{{workspace_root}}/scripts/build/Build-Watchdog.ps1"

# ── Run ───────────────────────────────────────────────────────────────────────

# Launch Bonsai Workspace (desktop mode)
launch:
    powershell -NoProfile -ExecutionPolicy Bypass \
        -File "{{workspace_root}}/scripts/launch/Launch-BonsaiWorkspace.ps1"

# Start the Tauri dev server (HMR)
dev:
    cd bonsai-workspace/src && npx tauri dev

# Start the headless daemon
daemon:
    cargo run -p bonsai-daemon

# ── Test ──────────────────────────────────────────────────────────────────────

# Run all Rust tests
test:
    cargo test --workspace

# Run watchdog tests only
test-watchdog:
    cargo test --manifest-path crates/bonsai-watchdog/Cargo.toml -- --nocapture

# Run frontend tests
test-frontend:
    npm --prefix bonsai-workspace/src run test

# Run integration tests
test-integration:
    python tests/integration/test_daemon_local.py

# ── Lint / Check ──────────────────────────────────────────────────────────────

# Check entire workspace (fast, no codegen)
check:
    cargo check --workspace

# Clippy + fmt check
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace -- -D warnings
    npm --prefix bonsai-workspace/src run lint

# ── Release ───────────────────────────────────────────────────────────────────

# Tag and push a release (requires VERSION env var, e.g. just release VERSION=v0.2.0)
release VERSION="":
    #!/usr/bin/env sh
    if [ -z "{{VERSION}}" ]; then echo "Usage: just release VERSION=v0.x.y"; exit 1; fi
    git tag -a {{VERSION}} -m "{{VERSION}}"
    git push origin {{VERSION}}
    gh release create {{VERSION}} --title "{{VERSION}}" --notes-file CHANGELOG.md
