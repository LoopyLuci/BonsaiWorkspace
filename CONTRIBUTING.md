# Contributing to Bonsai Workspace

Thank you for considering a contribution. This document covers everything you need to get from a fresh clone to a passing test run.

## Prerequisites

| Tool | Minimum version |
|------|----------------|
| Rust (stable) | 1.78 |
| Node.js | 18 |
| npm | 9 |
| cargo-tauri | 2.x |

Optional but recommended: `sccache` (faster Rust rebuilds), `just` (task runner).

## Quick Start

```powershell
# 1. Install frontend deps
cd bonsai-workspace\src
npm install

# 2. Start the dev server (Tauri + Vite HMR)
npx tauri dev

# 3. In a separate terminal — start the daemon
cargo run -p bonsai-daemon

# 4. Run all tests
cargo test --workspace
npm --prefix bonsai-workspace\src run test
```

## Directory Layout

```
BonsaiWorkspace/
├── bonsai-workspace/   Tauri desktop app (Svelte frontend + Rust backend)
├── src-daemon/         Headless orchestration daemon
├── crates/             Shared Rust libraries and standalone binaries
│   └── bonsai-watchdog/  Survival-system watchdog (standalone workspace)
├── bonsai-bot/         CLI companion tool
├── config/             Runtime feature flags (features.yaml)
├── docs/               All documentation (architecture, plans, user guide)
├── scripts/            Build, launch, and developer utility scripts
└── tests/              Unit, integration, e2e, and manual smoke tests
```

See [docs/README.md](docs/README.md) for full documentation index.

## Coding Conventions

- **Rust**: `cargo fmt` and `cargo clippy -- -D warnings` must pass.
- **TypeScript/Svelte**: `npm run lint` must pass. No `any` casts without a comment.
- **Comments**: only when the *why* is non-obvious. No doc-comments that just restate the name.
- **Security**: the pipeline is **100% offline by default** — never add code that downloads model weights or external resources unless gated behind a user-confirmed UI button.

## Submitting a Pull Request

1. Fork the repo and create a feature branch off `main`.
2. Make your changes, add tests for new behaviour.
3. Run `cargo test --workspace` and `npm --prefix bonsai-workspace\src run test`.
4. Open a PR against `main`. The CI will run the full survival-system and workspace checks automatically.

## Reporting Bugs

Open an issue and include:
- OS and version
- Steps to reproduce
- The contents of `tool_test/launcher/latest.json` (if the launcher failed)
- Any `hs_err_pid*.log` files (JVM crash dumps, if present)

## License

By contributing you agree that your contributions will be licensed under the same license as the project (see `LICENSE`).
