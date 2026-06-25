#!/usr/bin/env bash
set -euo pipefail

# git-filter-repo cleanup helper
# Usage: run from repo root: bash scripts/git-filter-repo-cleanup.sh
# This script makes a backup branch, creates a mirror clone, runs git-filter-repo
# to remove common build artifacts and large binaries, and force-pushes the
# rewritten history back to origin. READ the instructions file before running.

OLD_BRANCH="fix/warnings-wasm-example"
REMOTE="origin"
BACKUP_BRANCH="backup/${OLD_BRANCH}-$(date -u +%Y%m%dT%H%M%SZ)"

echo "==> Backing up $OLD_BRANCH to $BACKUP_BRANCH on $REMOTE"
git fetch "$REMOTE"
git checkout -B "$BACKUP_BRANCH" "$OLD_BRANCH"
git push "$REMOTE" "$BACKUP_BRANCH:$BACKUP_BRANCH"

ORIG_URL=$(git remote get-url "$REMOTE")

TMPDIR=$(mktemp -d)
echo "==> Creating mirror clone in $TMPDIR"
git clone --mirror "$PWD" "$TMPDIR/repo.git"
cd "$TMPDIR/repo.git"

echo "==> Running git-filter-repo to remove /target/ and common large binary extensions"
# Adjust path-globs as needed. --invert-paths removes matching paths from history.
git filter-repo \
  --invert-paths \
  --path-glob "bonsai-runtime/target/**" \
  --path-glob "**/target/**" \
  --path-glob "**/target/*" \
  --path-glob "*.pdb" \
  --path-glob "*.rlib" \
  --path-glob "*.exe" \
  --path-glob "*.dll" \
  --path-glob "*.so" \
  --path-glob "*.dylib"

# Push rewritten history back to origin
echo "==> Pushing rewritten history back to origin (force)"
if [ -z "$ORIG_URL" ]; then
  echo "ERROR: origin URL not found. Aborting."
  exit 2
fi

git remote set-url origin "$ORIG_URL"

echo "Pushing refs..."
git push --force origin --all
git push --force origin --tags

echo "Done. IMPORTANT: This rewrote history. Coordinate with collaborators and follow the instructions in scripts/git-filter-repo-instructions.md to reset local clones."