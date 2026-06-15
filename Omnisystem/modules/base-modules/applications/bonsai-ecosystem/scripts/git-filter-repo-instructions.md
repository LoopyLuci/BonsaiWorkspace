# git-filter-repo cleanup — safe instructions

This document explains how to safely remove large build artifacts (for example `target/` directories and large binaries) from a Git branch using `git-filter-repo`. This is a destructive operation (rewrites history) and must be used only when necessary. Prefer creating a clean branch (non-destructive) whenever possible — a clean branch was created for this work: `fix/warnings-wasm-example-patch`.

## Prerequisites

- Install `git-filter-repo` (recommended):
  - pip: `pip install git-filter-repo`
  - Or follow the official install steps: https://github.com/newren/git-filter-repo
- A remote backup of the branch you will rewrite (we create one below).

## Summary of steps (safe, recommended)

1. Create a backup of the branch and push it to `origin` so the original is preserved:

```bash
git fetch origin
git checkout fix/warnings-wasm-example
git branch -m fix/warnings-wasm-example backup/fix-warnings-wasm-example-YYYYMMDD
git push origin backup/fix-warnings-wasm-example-YYYYMMDD
```

2. Use the helper script (recommended) from the repository root. Two helpers are provided:

- `scripts/git-filter-repo-cleanup.sh` (for macOS / Linux / WSL)
- `scripts/git-filter-repo-cleanup.ps1` (for Windows PowerShell)

Run the appropriate script (careful — it force-pushes rewritten history):

```bash
# Linux / macOS / WSL
bash scripts/git-filter-repo-cleanup.sh

# Windows PowerShell (run as normal user)
pwsh -File .\scripts\git-filter-repo-cleanup.ps1
```

3. The helper will:
- Create a backup branch and push it to the remote.
- Create a mirror clone and run `git-filter-repo --invert-paths` to remove matching paths.
- Force-push rewritten refs and tags to `origin`.

## What the helper currently removes (edit as needed)
- All `*/target/**` paths (including `bonsai-runtime/target/**`)
- Files matching: `*.pdb`, `*.rlib`, `*.exe`, `*.dll`, `*.so`, `*.dylib`

If you need a narrower sweep, modify the `--path-glob` list in the script before running.

## After a successful rewrite — collaborator instructions

Because the repository history was rewritten, **all collaborators must update their local clones**. Recommended steps for each collaborator:

```bash
# Fetch rewritten history
git fetch origin

# Reset the local branch to the rewritten branch
git checkout fix/warnings-wasm-example
git reset --hard origin/fix/warnings-wasm-example

# Optional: clean reflogs and unreachable objects
git reflog expire --expire=now --all
git gc --prune=now --aggressive
```

If someone has local work based on the old branch, they must rebase their unpushed commits onto the rewritten branch or re-create them in a fresh branch.

## Alternatives

- Use the [BFG Repo-Cleaner](https://rtyley.github.io/bfg-repo-cleaner/) as a simpler option for removing large files by name/pattern. It is less flexible than `git-filter-repo` but easier for basic use cases.

## Warnings

- Rewriting public history is disruptive. Coordinate and notify everyone who may have cloned or forked the repository.
- Always keep a backup branch before pushing a rewritten branch to remote.

## If you need help

If you'd like, I can:

- Run the helper in a temporary mirror (locally) and show the list of removed blobs/commits before pushing.
- Prepare a list of the largest blobs in the current repo to target only the problematic objects.

