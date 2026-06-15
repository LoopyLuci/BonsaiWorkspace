#!/bin/bash
# ci_pipeline.sh – Master CI/CD pipeline orchestrator
# Runs all test stages for a commit; blocks merge on failure

set -euo pipefail

# ============================================================================
# Configuration
# ============================================================================

COMMIT_HASH="${1:-$(git rev-parse HEAD)}"
BRANCH="${2:-$(git rev-parse --abbrev-ref HEAD)}"
RUN_ID="run-$(date +%s)-$(head -c 8 /dev/urandom | xxd -p)"

echo "=================================================="
echo "CI/CD Pipeline Run"
echo "=================================================="
echo "Commit: $COMMIT_HASH"
echo "Branch: $BRANCH"
echo "Run ID: $RUN_ID"
echo "=================================================="

# ============================================================================
# Helper functions
# ============================================================================

log_stage() {
    local stage=$1
    echo ""
    echo "├─ [$stage] Starting..."
}

log_success() {
    local stage=$1
    local duration=$2
    echo "└─ [$stage] ✅ PASSED (${duration}s)"
}

log_failure() {
    local stage=$1
    echo "└─ [$stage] ❌ FAILED"
    exit 1
}

# ============================================================================
# Stage 0: Pre-flight checks
# ============================================================================

log_stage "Pre-flight"
START=$(date +%s)

# Check for forbidden patterns
if grep -r -i -E "(Psychopathy|Guardrail|Flowers)" --include="*.ti" --include="*.rs" --include="*.toml" --include="*.yaml" . >/dev/null 2>&1; then
    echo "ERROR: Forbidden term found in source code"
    log_failure "Pre-flight"
fi

# Check for formatting
# build fmt --check  # Would be implemented

END=$(date +%s)
DURATION=$((END - START))
log_success "Pre-flight" "$DURATION"

# ============================================================================
# Stage 1: Build
# ============================================================================

log_stage "Build"
START=$(date +%s)

build build --all --release 2>&1 | tail -10

END=$(date +%s)
DURATION=$((END - START))
log_success "Build" "$DURATION"

# ============================================================================
# Stage 2: Unit Tests (Parallel)
# ============================================================================

log_stage "Unit Tests"
START=$(date +%s)

build test --unit --parallel 8 2>&1 | grep -E "(PASS|FAIL|test|error)" | tail -20

END=$(date +%s)
DURATION=$((END - START))
log_success "Unit Tests" "$DURATION"

# ============================================================================
# Stage 3: Integration Tests (Parallel)
# ============================================================================

log_stage "Integration Tests"
START=$(date +%s)

build test --integration --parallel 4 2>&1 | grep -E "(PASS|FAIL|service)" | tail -20

END=$(date +%s)
DURATION=$((END - START))
log_success "Integration Tests" "$DURATION"

# ============================================================================
# Stage 4: Formal Verification
# ============================================================================

log_stage "Formal Verification"
START=$(date +%s)

axiom verify --workspace 2>&1 | grep -E "(Verified|Failed|error)" | tail -10

END=$(date +%s)
DURATION=$((END - START))
log_success "Formal Verification" "$DURATION"

# ============================================================================
# Stage 5: Performance Benchmarks
# ============================================================================

log_stage "Performance Benchmarks"
START=$(date +%s)

build bench --suite micro 2>&1 | grep -E "(throughput|latency|regression)" | tail -15

END=$(date +%s)
DURATION=$((END - START))
log_success "Performance Benchmarks" "$DURATION"

# ============================================================================
# Stage 6: Security & Fuzzing
# ============================================================================

log_stage "Security & Fuzzing"
START=$(date +%s)

# Limited fuzzing for CI (full fuzzing in nightly)
timeout 300 build fuzz --duration 300 --suite sandbox 2>&1 | grep -E "(found|crash|safe)" | tail -10

END=$(date +%s)
DURATION=$((END - START))
log_success "Security & Fuzzing" "$DURATION"

# ============================================================================
# Stage 7: UI Visual Regression
# ============================================================================

log_stage "UI Visual Regression"
START=$(date +%s)

build test --ui --headless 2>&1 | grep -E "(pass|fail|regression)" | tail -10

END=$(date +%s)
DURATION=$((END - START))
log_success "UI Visual Regression" "$DURATION"

# ============================================================================
# Stage 8: Polyglot & Asset Tests
# ============================================================================

log_stage "Polyglot Tests"
START=$(date +%s)

build test --polyglot --languages representative 2>&1 | grep -E "(language|fidelity|passed)" | tail -15

END=$(date +%s)
DURATION=$((END - START))
log_success "Polyglot Tests" "$DURATION"

# ============================================================================
# Merge Gate
# ============================================================================

echo ""
echo "=================================================="
echo "✅ ALL TESTS PASSED – READY TO MERGE"
echo "=================================================="

# Upload PGO profiles if all tests passed
if [ "$BRANCH" == "main" ]; then
    echo "Uploading PGO profiles to UMS..."
    build pgo upload --profile default
fi

exit 0
