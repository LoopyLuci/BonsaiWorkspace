# OmniPM — Package Manager

OmniPM is the official package manager for OmniOS. It manages discovery, installation, versioning, and security auditing of packages for all Omni-Language projects.

---

## Package Format

An OmniPM package is a `.omnipkg` file — a ZIP archive with the following structure:

```
my-package-1.2.0.omnipkg
├── manifest.json          Package metadata and dependency declarations
├── src/                   Source files (Omni-Languages)
│   ├── lib.titan
│   └── utils.titan
├── ir/                    Pre-compiled IR (optional, for faster installs)
│   └── my-package.ir.json
└── README.md
```

### `manifest.json`
```json
{
  "name": "omni-http",
  "version": "1.2.0",
  "description": "HTTP client and server for OmniOS",
  "authors": ["Omnisystem Team <team@omnisystem.dev>"],
  "license": "MIT",
  "homepage": "https://github.com/omnisystem/omni-http",
  "keywords": ["http", "networking", "server", "client"],
  "languages": ["titan", "aether"],
  "entry": "src/lib.titan",
  "dependencies": {
    "omni-net": "^1.0.0",
    "omni-json": "^2.0.0"
  },
  "sha256": "abc123def456...",
  "publishedAt": "2026-06-01T10:00:00Z",
  "verified": true
}
```

---

## Registry

The OmniPM registry is hosted at `https://registry.omnisystem.dev`.

### Registry API

| Endpoint | Method | Description |
|---|---|---|
| `/packages` | GET | List all packages (paginated) |
| `/packages?q={query}` | GET | Search packages |
| `/packages/{name}` | GET | Package metadata and all versions |
| `/packages/{name}/{version}` | GET | Specific version metadata |
| `/packages/{name}/{version}/download` | GET | Download `.omnipkg` file |
| `/audit` | POST | Vulnerability check for a list of packages |
| `/publish` | POST | Publish a new package (requires auth token) |

### Verification Badge
Packages reviewed and approved by the Omnisystem team receive a "Verified" badge. Verified packages:
- Source code has been reviewed for malicious patterns
- Build output is reproducible (registry can re-build and match the published checksum)
- Dependencies are all also verified (no unverified transitive deps)
- No outbound network calls except documented ones (e.g., a DNS package may call DNS servers)

---

## Installation

### Install a Package
```bash
omnicc pm add omni-http
omnicc pm add omni-http@1.2.0    # specific version
omnicc pm add omni-http@^1.0.0   # version range
```

This command:
1. Resolves the version range against available versions in the registry
2. Downloads the `.omnipkg` file over HTTPS
3. Verifies the SHA-256 checksum against the registry's published checksum
4. Extracts to `~/.omnisystem/packages/omni-http/1.2.0/`
5. Recursively installs all declared dependencies
6. Updates `BUILD.omnisystem` to add the dependency
7. Updates `omnisystem.lock` with the exact pinned versions of all packages

### `omnisystem.lock`
The lock file pins all transitive dependencies to exact versions for reproducible builds:

```
# omnisystem.lock
# DO NOT EDIT MANUALLY

[[package]]
name = "omni-http"
version = "1.2.0"
sha256 = "abc123def456..."
dependencies = ["omni-net@1.1.0", "omni-json@2.0.1"]

[[package]]
name = "omni-net"
version = "1.1.0"
sha256 = "def789abc123..."
dependencies = []

[[package]]
name = "omni-json"
version = "2.0.1"
sha256 = "789def123abc..."
dependencies = []
```

Commit `omnisystem.lock` to version control. Do not commit `~/.omnisystem/packages/` (that is the extracted cache, like `node_modules`).

### Restore from Lock File
```bash
omnicc pm install    # installs exactly what is in omnisystem.lock, no network resolution
```

If `omnisystem.lock` exists, `pm install` with no arguments uses pinned versions only. This is what CI/CD systems should use.

---

## Version Specifiers

| Specifier | Meaning | Example |
|---|---|---|
| `1.2.0` | Exact version | Only 1.2.0 |
| `^1.2.0` | Compatible with 1.x.x | >= 1.2.0, < 2.0.0 |
| `~1.2.0` | Patch-level compatible | >= 1.2.0, < 1.3.0 |
| `>=1.0.0` | Minimum version | 1.0.0 or higher |
| `*` | Any version | Latest stable |
| `1.x` | Major-pinned | Any 1.x.x |

---

## Using Packages in Code

After installation, import packages in any Omni-Language file:

**Titan:**
```titan
import omni_http::{HttpServer, Request, Response};
import omni_json::Json;

fn handle_request(req: Request) -> Response {
    let body = Json::parse(req.body);
    return Response::ok(Json::stringify(body));
}
```

**Aether:**
```aether
import omni_http::HttpServer;

actor WebServer {
    message Start { port: i32 }

    handler Start(msg) {
        HttpServer::bind("0.0.0.0", msg.port, |req| {
            self.send(HandleRequest { req });
        });
    }
}
```

The OmniCC compiler resolves package imports from `~/.omnisystem/packages/` automatically based on `BUILD.omnisystem` and `omnisystem.lock`.

---

## Removing Packages

```bash
omnicc pm remove omni-http
```

Removes the package from:
- `BUILD.omnisystem` (dependency declaration)
- `omnisystem.lock` (pinned version)
- `~/.omnisystem/packages/omni-http/` (extracted files)

OmniPM does **not** automatically remove packages that were installed as dependencies of the removed package (they may be needed by other installed packages). To clean unused transitive dependencies:

```bash
omnicc pm prune    # remove packages not referenced by any direct dependency
```

---

## Updating Packages

```bash
omnicc pm update                # update all packages to latest compatible versions
omnicc pm update omni-http      # update only omni-http
omnicc pm update --latest       # update all packages to absolute latest (may break compatibility)
```

After update, `omnisystem.lock` is rewritten with the new pinned versions. Run `omnicc build` to verify the updated dependencies still compile.

---

## Security Audit

```bash
omnicc pm audit
```

Sends the list of installed packages and versions to `https://registry.omnisystem.dev/audit`. The registry checks against its vulnerability database (maintained by the Omnisystem security team).

**Output:**
```
Auditing 4 installed packages...

✓ omni-http@1.2.0    no known vulnerabilities
✓ omni-json@2.0.1    no known vulnerabilities
⚠ omni-net@1.0.0     CVE-2026-1234 (MEDIUM) — buffer overread in TCP header parser
                     Fix: upgrade to omni-net@1.1.0
✓ omni-testing@1.0.0 no known vulnerabilities

1 vulnerability found. Run: omnicc pm update omni-net
```

---

## Publishing a Package

### Prepare
1. Ensure `manifest.json` has all required fields
2. Ensure `omnisystem.lock` is committed
3. Write a `README.md`
4. Run `omnicc test` — all tests must pass
5. Tag the release in git: `git tag v1.2.0`

### Authenticate
```bash
omnicc pm login
# Opens browser to https://registry.omnisystem.dev/login
# Paste the resulting token
```

### Publish
```bash
omnicc pm publish
```

OmniPM will:
1. Run `omnicc build --check` to verify the package compiles
2. Pack the `.omnipkg` archive
3. Compute the SHA-256 checksum
4. Upload to the registry
5. Publish the package under your account

The package appears in the registry immediately. Verified status is applied after manual review (typically within 48 hours).

---

## Private Registry

For enterprise or private package hosting:

```bash
# Set registry URL
omnicc pm config set registry-url https://packages.mycompany.com/omnisystem

# Authenticate
omnicc pm login --registry https://packages.mycompany.com/omnisystem
```

Or set in `BUILD.omnisystem`:
```
[registry]
url = "https://packages.mycompany.com/omnisystem"
```

---

## Built-in Packages (Pre-installed)

These packages come with OmniCC and do not require installation:

| Package | Description |
|---|---|
| `std` | Titan standard library (collections, string, math, I/O) |
| `std_vera` | Vera standard UI components |
| `std_aether` | Aether runtime and core actors |
| `std_sylva` | Sylva tensor operations and basic layers |
| `std_axiom` | Axiom formal verification runtime |
| `std_helix` | Helix shader standard library |
| `std_nexus` | Nexus layout engine |
