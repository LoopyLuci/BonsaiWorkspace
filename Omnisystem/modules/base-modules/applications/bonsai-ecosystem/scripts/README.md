# Scripts

## Directory Structure

| Directory | Purpose |
|-----------|---------|
| `build/`   | Build scripts — Tauri app, watchdog binary |
| `launch/`  | Launch scripts — start Bonsai Workspace locally |
| `dev/`     | Developer utilities — desktop shortcut, autopilot, babashka tools |
| `archive/` | Deprecated one-off scripts kept for reference |

Root-level `Launch-BonsaiWorkspace.ps1` and `BonsaiExeLauncherBuilder.ps1` are thin shims that delegate to the real scripts in `scripts/launch/` and `scripts/build/` respectively.

---

## CI/CD Automation Scripts

The following scripts automate validation, documentation generation, and quality checks for the Bonsai Ecosystem:

### Quality & Security Checks

#### `check_no_private_names.ps1` / `check_no_private_names.sh`

**Purpose**: Verify that no private or internal model names appear anywhere in the repository.

**Private Names** (must not appear):
- `Psychopathy Octopus` → use `Custom Octopus AI` or `Server-Specific Model`
- `Guardrail` → use `Safety Model` or `Internal Research Model`
- `Flowers` → use `Fine-Tuned Model` or `User-Specific LoRA`

**Usage**:
```powershell
.\scripts\check_no_private_names.ps1
```

**CI/CD**: Run on every commit; fail build if private names found.

---

### Documentation Generation

#### `generate_language_docs.ps1`

**Purpose**: Auto-generate `docs/LANGUAGE_SUPPORT.md` from `languages.yaml` manifest.

Produces documentation with:
- 750+ languages grouped by family
- Version and compiler information
- Instructions for adding new languages

**Usage**:
```powershell
.\scripts\generate_language_docs.ps1
```

**CI/CD**: Run when `languages.yaml` changes.

---

### Documentation Validation

#### `check_links.ps1`

**Purpose**: Validate all links in Markdown documentation.

Checks for:
- Broken internal file links
- Invalid anchor references
- Unreachable external URLs (optional)

**Usage**:
```powershell
# Check internal links only
.\scripts\check_links.ps1

# Also validate external URLs
.\scripts\check_links.ps1 -CheckExternal
```

**CI/CD**: Run on every commit; warn (don't fail) on broken links.

---

#### `validate_docs.ps1`

**Purpose**: Enforce rustdoc coverage for all public APIs.

Ensures:
- All public items have `///` doc comments
- No documentation build warnings
- Code examples in docs compile

**Usage**:
```powershell
.\scripts\validate_docs.ps1
```

**CI/CD**: Run on every commit; fail if docs are incomplete.

---

## Integration Examples

### GitHub Actions Workflow

```yaml
name: Quality Checks

on: [push, pull_request]

jobs:
  quality:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Check private names
        run: .\scripts\check_no_private_names.ps1
      
      - name: Generate language docs
        run: .\scripts\generate_language_docs.ps1
      
      - name: Check documentation links
        run: .\scripts\check_links.ps1
        continue-on-error: true
      
      - name: Validate rustdoc
        run: .\scripts\validate_docs.ps1
```

### Pre-Commit Hook

Save as `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "Running pre-commit checks..."
./scripts/check_no_private_names.sh || exit 1
echo "✅ Pre-commit checks passed"
```

---

## All Scripts in This Directory

Run any of these scripts for CI/CD automation:

- `check_no_private_names.ps1` — Verify no private names
- `check_no_private_names.sh` — Bash version
- `generate_language_docs.ps1` — Generate language documentation
- `check_links.ps1` — Validate documentation links
- `validate_docs.ps1` — Check rustdoc coverage
- `README.md` — This file

---

**Last Updated**: 2026-06-04  
**Status**: CI/CD automation scripts ready for production
