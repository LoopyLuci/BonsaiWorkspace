# Bonsai Plugin & Agent Marketplace
## Manifest Format (`bonsai-plugin.toml`)
[plugin] name, version, author, description, license
[agent] trait = "Agent", capabilities = ["TextGeneration","CodeEditing"]
[security] sandbox = "wasm" | "venv" | "microvm", permissions = ["read_file","write_file"]
## Discovery API
GET /api/v1/plugins — list installed
POST /api/v1/plugins/install — from URL or path
DELETE /api/v1/plugins/{id}
## Registry
Plugins stored in ~/.bonsai/plugins/.wasm. One-click install from git URLs or marketplace. Sandboxed by default.
