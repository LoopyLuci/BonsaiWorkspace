#!/bin/bash
# SMOKE TESTS FOR STAGING/PRODUCTION DEPLOYMENTS
# Quick validation that services are responding and healthy
# Called by: Run-CI.ps1 (deploy-staging stage)

set -e

ENVIRONMENT=${1:-staging}
TIMEOUT=30
MAX_RETRIES=5

echo "🧪 Running smoke tests for $ENVIRONMENT environment..."
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test health endpoint
test_health_endpoint() {
    local url=$1
    local retry=0

    echo -n "Testing health endpoint: $url ... "

    while [ $retry -lt $MAX_RETRIES ]; do
        if curl -sf --max-time $TIMEOUT "$url" > /dev/null 2>&1; then
            echo -e "${GREEN}✓${NC}"
            return 0
        fi
        retry=$((retry + 1))
        if [ $retry -lt $MAX_RETRIES ]; then
            echo -n "retry ($retry/$MAX_RETRIES)..."
            sleep 2
        fi
    done

    echo -e "${RED}✗${NC}"
    return 1
}

# Test API endpoint
test_api_endpoint() {
    local url=$1
    local expected_status=$2
    local retry=0

    echo -n "Testing API endpoint: $url ... "

    while [ $retry -lt $MAX_RETRIES ]; do
        response=$(curl -s -o /dev/null -w "%{http_code}" --max-time $TIMEOUT "$url")
        if [ "$response" = "$expected_status" ]; then
            echo -e "${GREEN}✓ (HTTP $response)${NC}"
            return 0
        fi
        retry=$((retry + 1))
        if [ $retry -lt $MAX_RETRIES ]; then
            echo -n "retry ($retry/$MAX_RETRIES)..."
            sleep 2
        fi
    done

    echo -e "${RED}✗ (HTTP $response)${NC}"
    return 1
}

# Test response time
test_response_time() {
    local url=$1
    local max_time=$2

    echo -n "Testing response time: $url (<${max_time}ms) ... "

    response_time=$(curl -w '%{time_total}' -o /dev/null -s "$url" | awk '{print int($1 * 1000)}')

    if [ "$response_time" -lt "$max_time" ]; then
        echo -e "${GREEN}✓ (${response_time}ms)${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠ (${response_time}ms, expected <${max_time}ms)${NC}"
        return 0  # Warning only
    fi
}

# Main test execution
main() {
    local failed=0

    if [ "$ENVIRONMENT" = "staging" ]; then
        echo "📋 Staging Environment Tests"
        echo "================================"
        echo ""

        # Test local services
        test_health_endpoint "http://localhost:8080/health" || ((failed++))
        test_health_endpoint "http://localhost:8081/health" || ((failed++))
        test_api_endpoint "http://localhost:8080/api/v1/status" "200" || ((failed++))
        test_response_time "http://localhost:8080/api/v1/status" "500" || ((failed++))

    elif [ "$ENVIRONMENT" = "production" ]; then
        echo "📋 Production Environment Tests"
        echo "================================"
        echo ""

        echo -e "${YELLOW}⚠ Production smoke tests disabled in local CI/CD${NC}"
        echo "   Connect to production environment manually to verify"

    else
        echo -e "${RED}✗ Unknown environment: $ENVIRONMENT${NC}"
        echo "   Usage: $0 <staging|production>"
        exit 1
    fi

    echo ""
    if [ $failed -eq 0 ]; then
        echo -e "${GREEN}✓ All smoke tests passed${NC}"
        exit 0
    else
        echo -e "${RED}✗ $failed smoke tests failed${NC}"
        exit 1
    fi
}

main
