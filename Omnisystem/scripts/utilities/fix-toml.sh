#!/bin/bash

# Fix Phase 4 migration TOML syntax errors
# This script fixes the malformed Cargo.toml files created by the migration

DRY_RUN="${1:-false}"
VERBOSE="${2:-false}"

CRATES_DIR="crates"

fixed_count=0
skipped_count=0
error_count=0
total_count=0

echo "Phase 4 TOML Syntax Fix Script"
if [ "$DRY_RUN" = "true" ]; then
    echo "Mode: DRY RUN (no changes will be made)"
fi
echo ""

for crate_dir in "$CRATES_DIR"/*; do
    if [ ! -d "$crate_dir" ]; then
        continue
    fi

    toml_file="$crate_dir/Cargo.toml"
    if [ ! -f "$toml_file" ]; then
        continue
    fi

    total_count=$((total_count + 1))

    # Check if file has issues
    if ! grep -q "} }" "$toml_file" && ! grep -q "}, features" "$toml_file"; then
        skipped_count=$((skipped_count + 1))
        continue
    fi

    # Create temporary file
    temp_file="${toml_file}.tmp"
    cp "$toml_file" "$temp_file"

    # Fix 1: Remove the extra closing brace from dependency lines
    # Pattern: omnisystem-* = { path = "..." } } → omnisystem-* = { path = "..." }
    sed -i 's/} }$/}/' "$temp_file"

    # Fix 2: Remove malformed dev-dependencies entries
    # Pattern: omnisystem-async-runtime = { path = "..." }, features = ["full"] }
    # Replace with empty (we don't want old tokio dev-deps)
    sed -i '/^omnisystem-.*= { path.*}, features.*}$/d' "$temp_file"

    if [ "$DRY_RUN" != "true" ]; then
        # Backup original
        cp "$toml_file" "${toml_file}.backup"
        # Apply fixes
        mv "$temp_file" "$toml_file"
    fi

    # Check if it was actually changed
    if ! diff -q "$toml_file" "$temp_file" > /dev/null 2>&1 || [ "$DRY_RUN" != "true" ]; then
        crate_name=$(grep '^name = ' "$toml_file" | sed 's/name = "\(.*\)"/\1/')
        echo "✓ Fixed: $crate_name"
        fixed_count=$((fixed_count + 1))
    fi

    # Cleanup temp file if dry run
    if [ "$DRY_RUN" = "true" ] && [ -f "$temp_file" ]; then
        rm "$temp_file"
    fi
done

echo ""
echo "Fix Summary:"
echo "  Crates processed: $total_count"
echo "  Crates fixed: $fixed_count"
echo "  Crates skipped: $skipped_count"

if [ "$DRY_RUN" = "true" ]; then
    echo ""
    echo "This was a DRY RUN. Run './fix-toml.sh' without argument to apply fixes."
else
    echo ""
    echo "All fixes applied! Run 'cargo check --all' to verify."
fi
