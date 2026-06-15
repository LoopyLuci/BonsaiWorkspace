#!/bin/bash
# migrate_crate.sh - Migrate a single Rust crate to Omnisystem module
# Usage: ./migrate_crate.sh --crate my-crate --language titan --action generate
# Actions: analyze, generate, migrate, verify, cleanup

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
CRATE_NAME=""
LANGUAGE=""
ACTION="analyze"
PARALLEL=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --crate)
            CRATE_NAME="$2"
            shift 2
            ;;
        --language)
            LANGUAGE="$2"
            shift 2
            ;;
        --action)
            ACTION="$2"
            shift 2
            ;;
        --parallel)
            PARALLEL=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate inputs
if [ -z "$CRATE_NAME" ]; then
    echo "Error: --crate is required"
    exit 1
fi

if [ -z "$LANGUAGE" ]; then
    echo "Error: --language is required (titan|aether|sylva|axiom)"
    exit 1
fi

CRATE_PATH="Omnisystem/crates/$CRATE_NAME"
MODULE_PATH="Omnisystem/$LANGUAGE"

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if crate exists
check_crate_exists() {
    if [ ! -d "$CRATE_PATH" ]; then
        log_error "Crate not found: $CRATE_PATH"
        exit 1
    fi
}

# Phase 1: Analyze crate
analyze_crate() {
    log_info "Analyzing crate: $CRATE_NAME"

    check_crate_exists

    # Count LOC
    local loc=$(find "$CRATE_PATH/src" -name "*.rs" -type f 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")
    local files=$(find "$CRATE_PATH/src" -name "*.rs" -type f 2>/dev/null | wc -l || echo "0")
    local deps=$(grep -c "^\[dependencies\]" "$CRATE_PATH/Cargo.toml" 2>/dev/null || echo "0")

    log_info "Crate statistics:"
    echo "    Files: $files"
    echo "    Lines of code: $loc"
    echo "    Target language: $LANGUAGE"

    # Extract key structs/functions
    log_info "Extracting public API..."
    grep -oP "pub (?:struct|fn|trait|enum) \K\w+" "$CRATE_PATH/src/lib.rs" 2>/dev/null | head -20 || log_warning "Could not extract API"

    log_success "Analysis complete"
}

# Phase 2: Generate module structure
generate_module() {
    log_info "Generating module structure for: $CRATE_NAME"

    check_crate_exists

    # Determine module category based on crate name
    local category=$(echo "$CRATE_NAME" | sed 's/-[0-9].*$//' | sed 's/-[a-z]*$//')

    # Create module directory structure
    local module_dir="$MODULE_PATH/$category/$(echo $CRATE_NAME | sed "s/.*-//")"
    mkdir -p "$module_dir/tests"
    mkdir -p "$module_dir/docs"

    log_info "Creating module at: $module_dir"

    # Create module stubs based on language
    case $LANGUAGE in
        titan)
            create_titan_stub "$module_dir" "$CRATE_NAME"
            ;;
        aether)
            create_aether_stub "$module_dir" "$CRATE_NAME"
            ;;
        sylva)
            create_sylva_stub "$module_dir" "$CRATE_NAME"
            ;;
        axiom)
            create_axiom_stub "$module_dir" "$CRATE_NAME"
            ;;
    esac

    # Create migration documentation
    cat > "$module_dir/docs/MIGRATION.md" << EOF
# Migration: $CRATE_NAME

## Original Location
\`\`\`
Omnisystem/crates/$CRATE_NAME/
\`\`\`

## New Location
\`\`\`
$module_dir/
\`\`\`

## Status
- [ ] Core implementation ported
- [ ] Tests ported
- [ ] Documentation updated
- [ ] Verified against original

## Key Changes
- Language: Rust → $LANGUAGE
- Module structure: crate-based → omnisystem module-based
- Dependencies: external crates → omnisystem modules

## Migration Date
Generated: $(date)

## Verifier Notes
(To be filled in during verification phase)
EOF

    log_success "Module generated at: $module_dir"
    echo "Next: Run --action migrate to port implementation"
}

# Phase 3: Migrate implementation
migrate_crate_impl() {
    log_info "Migrating implementation: $CRATE_NAME → $LANGUAGE"

    check_crate_exists

    local module_dir="$MODULE_PATH/$category/$(echo $CRATE_NAME | sed "s/.*-//")"

    if [ ! -d "$module_dir" ]; then
        log_error "Module directory not found. Run --action generate first"
        exit 1
    fi

    log_info "Phase 1: Porting core logic..."
    # This would call language-specific converters
    case $LANGUAGE in
        titan)
            log_warning "Manual port needed for Rust → Titan"
            ;;
        aether)
            log_warning "Manual port needed for Rust → Aether"
            ;;
        sylva)
            log_warning "Manual port needed for Rust → Sylva"
            ;;
        axiom)
            log_warning "Manual port needed for Rust → Axiom"
            ;;
    esac

    log_info "Phase 2: Porting tests..."
    # Copy and convert test files
    if [ -d "$CRATE_PATH/src/tests" ] || [ -d "$CRATE_PATH/tests" ]; then
        log_success "Found tests to port"
    else
        log_warning "No tests found"
    fi

    log_info "Phase 3: Handling dependencies..."
    # Resolve and map dependencies
    log_warning "Manual dependency resolution may be needed"

    log_success "Migration template created. Manual implementation needed."
}

# Phase 4: Verify migration
verify_migration() {
    log_info "Verifying migration: $CRATE_NAME"

    check_crate_exists

    local module_dir="$MODULE_PATH/$category/$(echo $CRATE_NAME | sed "s/.*-//")"

    if [ ! -d "$module_dir" ]; then
        log_error "Module not found"
        exit 1
    fi

    # Check module file exists
    if [ -f "$module_dir/module.ti" ] || [ -f "$module_dir/module.ae" ] || [ -f "$module_dir/module.sy" ] || [ -f "$module_dir/module.ax" ]; then
        log_success "Module file exists"
    else
        log_error "Module file not found"
        exit 1
    fi

    # Check tests exist
    if [ -f "$module_dir/tests/tests.ti" ] || [ -f "$module_dir/tests/tests.ae" ] || [ -f "$module_dir/tests/tests.sy" ] || [ -f "$module_dir/tests/tests.ax" ]; then
        log_success "Tests found"
    else
        log_warning "No tests found - add them!"
    fi

    # Check documentation
    if [ -f "$module_dir/docs/MIGRATION.md" ] && [ -f "$module_dir/docs/API.md" ]; then
        log_success "Documentation found"
    else
        log_warning "Documentation incomplete"
    fi

    log_success "Verification complete"
}

# Phase 5: Cleanup
cleanup_crate() {
    log_info "Cleaning up: $CRATE_NAME"

    check_crate_exists

    # Create archive
    mkdir -p ".archive/crates"
    mv "$CRATE_PATH" ".archive/crates/$CRATE_NAME"

    log_success "Crate archived to .archive/crates/$CRATE_NAME"
    log_info "Remove from Cargo.toml: \"crates/$CRATE_NAME\""
}

# Language-specific stub creators
create_titan_stub() {
    local dir=$1
    local crate=$2
    cat > "$dir/module.ti" << EOF
// $dir/module.ti
// Migrated from: Omnisystem/crates/$crate/

// Type definitions
pub struct Example {
    value: i64,
}

// Public API
pub fn create() -> Example {
    return Example { value: 0 };
}

pub fn process(ex: &Example) -> i64 {
    return ex.value * 2;
}

// Tests
pub fn main() -> i64 {
    let ex = create();
    if process(&ex) == 0 {
        return 111;
    }
    return 1;
}
EOF
    log_success "Titan stub created"
}

create_aether_stub() {
    local dir=$1
    local crate=$2
    cat > "$dir/module.ae" << EOF
// $dir/module.ae
// Migrated from: Omnisystem/crates/$crate/

pub actor Example {
    state: i64,
}

pub fn create_actor() -> ActorRef {
    // Create actor
    return ActorRef { id: 1 };
}

pub fn send_message(actor: ActorRef, msg: Message) {
    // Send message to actor
}
EOF
    log_success "Aether stub created"
}

create_sylva_stub() {
    local dir=$1
    local crate=$2
    cat > "$dir/module.sy" << EOF
// $dir/module.sy
// Migrated from: Omnisystem/crates/$crate/

pub struct Model {
    parameters: Vec<f64>,
}

pub fn train(data: &Vec<f64>) -> Model {
    return Model { parameters: vec![0.0] };
}

pub fn predict(model: &Model, input: &f64) -> f64 {
    return *input;
}
EOF
    log_success "Sylva stub created"
}

create_axiom_stub() {
    local dir=$1
    local crate=$2
    cat > "$dir/module.ax" << EOF
// $dir/module.ax
// Migrated from: Omnisystem/crates/$crate/

pub struct Proof {
    statement: String,
    verified: bool,
}

pub fn create_proof(stmt: String) -> Proof {
    return Proof {
        statement: stmt,
        verified: false,
    };
}

pub fn verify(proof: &mut Proof) {
    proof.verified = true;
}
EOF
    log_success "Axiom stub created"
}

# Main execution
echo ""
echo "======================================================================"
echo "OMNISYSTEM CRATE MIGRATION TOOL"
echo "======================================================================"
echo ""
echo "Crate:    $CRATE_NAME"
echo "Language: $LANGUAGE"
echo "Action:   $ACTION"
echo ""

case $ACTION in
    analyze)
        analyze_crate
        ;;
    generate)
        generate_module
        ;;
    migrate)
        migrate_crate_impl
        ;;
    verify)
        verify_migration
        ;;
    cleanup)
        cleanup_crate
        ;;
    *)
        log_error "Unknown action: $ACTION"
        echo "Valid actions: analyze, generate, migrate, verify, cleanup"
        exit 1
        ;;
esac

echo ""
echo "======================================================================"
log_success "Complete"
echo "======================================================================"
echo ""
