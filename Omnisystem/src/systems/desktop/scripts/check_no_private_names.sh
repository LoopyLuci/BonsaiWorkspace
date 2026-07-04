#!/bin/bash
# Check for Private Names – CI/CD Automation
#
# This script verifies that no private or internal model names appear
# anywhere in the public repository (source code, documentation, config).
#
# Usage: ./scripts/check_no_private_names.sh [--fix]
#
# Private Names (MUST NOT APPEAR):
#   - Psychopathy Octopus (use: Custom Octopus AI, Server-Specific Model)
#   - Guardrail (use: Safety Model, Internal Research Model)
#   - Flowers (use: Fine-Tuned Model, User-Specific LoRA)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
FIX_MODE=${1:-""}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Private names to check (case-insensitive)
PRIVATE_NAMES=(
  "Psychopathy"
  "Guardrail"
  "Flowers"
)

# File patterns to check
FILE_PATTERNS=(
  "*.md"
  "*.rs"
  "*.toml"
  "*.yaml"
  "*.yml"
  "*.json"
  "*.sh"
  "*.py"
)

echo "🔍 Checking for private names in repository..."
echo "Repository root: $REPO_ROOT"
echo ""

FOUND=0
MATCHES=()

# Search for each private name
for name in "${PRIVATE_NAMES[@]}"; do
  echo "Checking for '$name'..."

  # Use git grep if in a git repo, otherwise use find + grep
  if [ -d "$REPO_ROOT/.git" ]; then
    if git -C "$REPO_ROOT" grep -i -n "$name" -- \
      '*.md' '*.rs' '*.toml' '*.yaml' '*.yml' '*.json' '*.sh' '*.py' 2>/dev/null | \
      grep -v "Binary file" > /tmp/private_names_$$.txt; then

      FOUND=1
      while IFS= read -r match; do
        MATCHES+=("$match")
        echo "  ❌ $match"
      done < /tmp/private_names_$$.txt
      rm -f /tmp/private_names_$$.txt
    fi
  else
    # Fallback for non-git repos
    if find "$REPO_ROOT" -type f \( -name "*.md" -o -name "*.rs" -o -name "*.toml" \
      -o -name "*.yaml" -o -name "*.yml" -o -name "*.json" -o -name "*.sh" -o -name "*.py" \) \
      -exec grep -l -i "$name" {} \; 2>/dev/null | head -20; then

      FOUND=1
      while IFS= read -r file; do
        grep -i -n "$name" "$file" | while IFS= read -r match; do
          MATCHES+=("$file: $match")
          echo "  ❌ $file: $match"
        done
      done < <(find "$REPO_ROOT" -type f \( -name "*.md" -o -name "*.rs" -o -name "*.toml" \
        -o -name "*.yaml" -o -name "*.yml" -o -name "*.json" -o -name "*.sh" -o -name "*.py" \) \
        -exec grep -l -i "$name" {} \; 2>/dev/null | head -20)
    fi
  fi
done

echo ""

if [ $FOUND -eq 0 ]; then
  echo -e "${GREEN}✅ No private names found. Repository is clean.${NC}"
  exit 0
else
  echo -e "${RED}❌ FAILED: Found ${#MATCHES[@]} reference(s) to private names.${NC}"
  echo ""
  echo "Private names MUST NOT appear in public code/docs. Replace with:"
  echo "  - 'Psychopathy Octopus' → 'Custom Octopus AI' or 'Server-Specific Model'"
  echo "  - 'Guardrail' → 'Safety Model' or 'Internal Research Model'"
  echo "  - 'Flowers' → 'Fine-Tuned Model' or 'User-Specific LoRA'"
  echo ""

  if [ "$FIX_MODE" = "--fix" ]; then
    echo "🔧 Attempting automated fixes..."
    # This would require manual intervention for now
    echo "   (Automated fixes not implemented; please replace manually)"
    exit 1
  else
    echo "Run with --fix flag to attempt automated replacement (not implemented yet)."
    exit 1
  fi
fi
