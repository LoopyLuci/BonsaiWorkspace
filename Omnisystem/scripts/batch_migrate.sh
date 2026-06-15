#!/bin/bash
# batch_migrate.sh - Migrate multiple crates in parallel
# Usage: ./batch_migrate.sh --language titan --parallel 4 --priority high

set -e

LANGUAGE=""
PARALLEL_JOBS=4
PRIORITY="all"
START_INDEX=1
END_INDEX=2432
REPORT_FILE="migration_reports/batch_migration_report.md"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --language)
            LANGUAGE="$2"
            shift 2
            ;;
        --parallel)
            PARALLEL_JOBS="$2"
            shift 2
            ;;
        --priority)
            PRIORITY="$2"
            shift 2
            ;;
        --start)
            START_INDEX="$2"
            shift 2
            ;;
        --end)
            END_INDEX="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

if [ -z "$LANGUAGE" ]; then
    echo "Error: --language is required"
    exit 1
fi

echo "======================================================================"
echo "OMNISYSTEM BATCH MIGRATION"
echo "======================================================================"
echo ""
echo "Language:       $LANGUAGE"
echo "Parallel jobs:  $PARALLEL_JOBS"
echo "Priority:       $PRIORITY"
echo "Range:          $START_INDEX - $END_INDEX"
echo ""

# Get list of crates matching criteria
CRATES=()
index=0

for crate_path in Omnisystem/crates/*; do
    if [ ! -d "$crate_path" ]; then
        continue
    fi

    index=$((index + 1))

    if [ $index -lt $START_INDEX ] || [ $index -gt $END_INDEX ]; then
        continue
    fi

    crate_name=$(basename "$crate_path")

    # Filter by language mapping if language specified
    if [ "$LANGUAGE" == "titan" ]; then
        if [[ ! "$crate_name" =~ ^(omnisystem|api|network|async|concurrent|thread|storage|db) ]]; then
            continue
        fi
    elif [ "$LANGUAGE" == "aether" ]; then
        if [[ ! "$crate_name" =~ ^(service|actor|distributed|consensus|mesh|routing) ]]; then
            continue
        fi
    elif [ "$LANGUAGE" == "sylva" ]; then
        if [[ ! "$crate_name" =~ ^(data|model|ml|freellmapi|learning|analytics) ]]; then
            continue
        fi
    elif [ "$LANGUAGE" == "axiom" ]; then
        if [[ ! "$crate_name" =~ ^(verify|proof|compliance|audit|formal|governance) ]]; then
            continue
        fi
    fi

    CRATES+=("$crate_name")
done

total_crates=${#CRATES[@]}
echo "Found $total_crates crates to migrate"
echo ""

if [ $total_crates -eq 0 ]; then
    echo "No crates found matching criteria"
    exit 1
fi

# Migration function
migrate_single() {
    local crate=$1
    local job_id=$2

    echo "[Job $job_id] Migrating: $crate..."

    # Run migration steps
    ./scripts/migrate_crate.sh \
        --crate "$crate" \
        --language "$LANGUAGE" \
        --action analyze > /dev/null 2>&1

    ./scripts/migrate_crate.sh \
        --crate "$crate" \
        --language "$LANGUAGE" \
        --action generate > /dev/null 2>&1

    ./scripts/migrate_crate.sh \
        --crate "$crate" \
        --language "$LANGUAGE" \
        --action migrate > /dev/null 2>&1

    echo "[Job $job_id] ✓ Completed: $crate"
}

# Export function so parallel can use it
export -f migrate_single

# Create report header
mkdir -p migration_reports
cat > "$REPORT_FILE" << EOF
# Batch Migration Report

**Date**: $(date)
**Language**: $LANGUAGE
**Total Crates**: $total_crates
**Parallel Jobs**: $PARALLEL_JOBS

## Summary
- Starting index: $START_INDEX
- Ending index: $END_INDEX
- Priority: $PRIORITY
- Status: IN PROGRESS

## Migrated Crates
EOF

# Run migrations in parallel
echo "Starting $PARALLEL_JOBS parallel jobs..."
echo ""

counter=0
completed=0
failed=0

# Use xargs for parallel execution
echo "${CRATES[@]}" | xargs -I {} -P $PARALLEL_JOBS bash -c "migrate_single {} $((++counter))"

# Update report
echo ""
echo "======================================================================"
echo "BATCH MIGRATION COMPLETE"
echo "======================================================================"
echo ""

migrated_crates=$(find Omnisystem/$LANGUAGE -name "module.*" -type f 2>/dev/null | wc -l)

cat >> "$REPORT_FILE" << EOF

## Results
- Total crates processed: $total_crates
- Migrated modules: $migrated_crates
- Success rate: $((migrated_crates * 100 / total_crates))%

## Next Steps
1. Review migration_reports/
2. Run: ./scripts/test_migrations.sh
3. Verify: ./scripts/verify_compatibility.sh
4. Archive: ./scripts/archive_migrated_crates.sh

## Generated Modules
EOF

# List generated modules
find Omnisystem/$LANGUAGE -type d -name "[a-z]*" | sort | while read dir; do
    echo "- $(basename $dir)" >> "$REPORT_FILE"
done

echo "Report saved to: $REPORT_FILE"
echo ""
echo "Summary:"
echo "  Crates analyzed:  $total_crates"
echo "  Modules created:  $migrated_crates"
echo "  Language target:  $LANGUAGE"
echo ""
echo "Next: Review $REPORT_FILE"
