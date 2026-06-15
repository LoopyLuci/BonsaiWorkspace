#!/bin/bash
# analyze_crates.sh - Analyze all Omnisystem crates for migration
# Purpose: Scan crates directory, extract metadata, build inventory
# Output: crates_analysis.json, crates_inventory.csv, dependency_graph.dot

set -e

CRATES_DIR="Omnisystem/crates"
OUTPUT_DIR="migration_reports"
ANALYSIS_FILE="${OUTPUT_DIR}/crates_analysis.json"
INVENTORY_FILE="${OUTPUT_DIR}/crates_inventory.csv"
DEPENDENCY_FILE="${OUTPUT_DIR}/dependency_graph.dot"

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "======================================================================"
echo "OMNISYSTEM CRATE ANALYSIS"
echo "======================================================================"
echo ""

# Count total crates
total_crates=$(find "$CRATES_DIR" -maxdepth 1 -type d | wc -l)
total_crates=$((total_crates - 1))  # Subtract the parent directory

echo "Found $total_crates crates"
echo "Starting analysis..."
echo ""

# Initialize JSON and CSV files
cat > "$ANALYSIS_FILE" << 'EOF'
{
  "metadata": {
    "analysis_date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
    "total_crates": 0,
    "total_lines_of_code": 0,
    "total_dependencies": 0
  },
  "crates": []
}
EOF

cat > "$INVENTORY_FILE" << 'EOF'
crate_name,category,language_target,loc,files,dependencies,priority,complexity
EOF

# Initialize dependency graph
cat > "$DEPENDENCY_FILE" << 'EOF'
digraph crate_dependencies {
  rankdir=LR;
  node [shape=box];
EOF

# Process each crate
crate_count=0
total_loc=0
total_deps=0

for crate_path in "$CRATES_DIR"/*; do
    if [ ! -d "$crate_path" ]; then
        continue
    fi

    crate_name=$(basename "$crate_path")
    crate_count=$((crate_count + 1))

    # Show progress
    if [ $((crate_count % 100)) -eq 0 ]; then
        echo "  Processed $crate_count crates..."
    fi

    # Extract metadata from Cargo.toml
    if [ -f "$crate_path/Cargo.toml" ]; then
        # Count LOC
        loc=$(find "$crate_path/src" -name "*.rs" -type f 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")

        # Count files
        file_count=$(find "$crate_path/src" -name "*.rs" -type f 2>/dev/null | wc -l || echo "0")

        # Extract dependencies from Cargo.toml
        deps=$(grep -oP '^\s*\K[a-z0-9_-]+(?=\s*=)' "$crate_path/Cargo.toml" 2>/dev/null | grep -v "^name$\|^version$" | wc -l || echo "0")

        total_loc=$((total_loc + loc))
        total_deps=$((total_deps + deps))

        # Classify by category (use prefix before first hyphen)
        category=$(echo "$crate_name" | sed 's/-.*$//')

        # Determine language target based on category
        if [[ "$category" =~ ^(omnisystem|api|network|async|concurrent|thread|storage|db|crypto)$ ]]; then
            language_target="titan"
        elif [[ "$category" =~ ^(service|actor|distributed|consensus|mesh|routing|agent)$ ]]; then
            language_target="aether"
        elif [[ "$category" =~ ^(data|model|ml|freellmapi|learning|analytics)$ ]]; then
            language_target="sylva"
        elif [[ "$category" =~ ^(verify|proof|compliance|audit|formal|governance)$ ]]; then
            language_target="axiom"
        else
            language_target="utility"
        fi

        # Calculate complexity (heuristic: LOC + files + deps)
        complexity=$(( (loc / 1000) + file_count + (deps / 2) ))

        # Determine priority
        if [ $complexity -lt 10 ]; then
            priority="low"
        elif [ $complexity -lt 50 ]; then
            priority="medium"
        else
            priority="high"
        fi

        # Add to CSV
        echo "$crate_name,$category,$language_target,$loc,$file_count,$deps,$priority,$complexity" >> "$INVENTORY_FILE"
    fi
done

echo ""
echo "======================================================================"
echo "ANALYSIS COMPLETE"
echo "======================================================================"
echo ""
echo "Results:"
echo "  Total crates analyzed:     $crate_count"
echo "  Total lines of code:       $total_loc"
echo "  Total dependencies:        $total_deps"
echo ""
echo "Output files:"
echo "  - $ANALYSIS_FILE"
echo "  - $INVENTORY_FILE"
echo "  - $DEPENDENCY_FILE"
echo ""
echo "Next steps:"
echo "  1. Review crates_inventory.csv for migration priorities"
echo "  2. Run: ./scripts/classify_crate.sh for detailed classification"
echo "  3. Run: ./scripts/build_dependency_graph.sh for dependency analysis"
echo ""
