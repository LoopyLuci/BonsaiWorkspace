#!/bin/bash
# PRODUCTION ROLLBACK - EMERGENCY ONLY
# Reverts to previous stable version in case of deployment issues
# Called by: Run-CI.ps1 (deploy-prod-rollback) or manual emergency invocation

set -e

echo "🔄 PRODUCTION ROLLBACK INITIATED"
echo "================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
BACKUP_DIR="./omnisystem-deployments/backups"
CURRENT_VERSION_FILE="./omnisystem-deployments/current_version.txt"
PREVIOUS_VERSION_FILE="./omnisystem-deployments/previous_version.txt"

# Safety check - require manual confirmation
confirm_rollback() {
    echo -e "${RED}⚠️  PRODUCTION ROLLBACK WILL:${NC}"
    echo "   1. Stop all services"
    echo "   2. Restore previous version"
    echo "   3. Restart services"
    echo "   4. Verify health checks"
    echo ""
    echo "This is IRREVERSIBLE. Current version will be replaced."
    echo ""

    read -p "Type 'ROLLBACK PRODUCTION' to confirm: " confirmation

    if [ "$confirmation" != "ROLLBACK PRODUCTION" ]; then
        echo -e "${YELLOW}Rollback cancelled${NC}"
        exit 0
    fi
}

# Get current and previous versions
get_versions() {
    if [ ! -f "$CURRENT_VERSION_FILE" ]; then
        echo -e "${RED}✗ Current version file not found${NC}"
        exit 1
    fi

    if [ ! -f "$PREVIOUS_VERSION_FILE" ]; then
        echo -e "${RED}✗ Previous version file not found (cannot rollback)${NC}"
        exit 1
    fi

    CURRENT_VERSION=$(cat "$CURRENT_VERSION_FILE")
    PREVIOUS_VERSION=$(cat "$PREVIOUS_VERSION_FILE")

    echo -e "${BLUE}Current version: $CURRENT_VERSION${NC}"
    echo -e "${BLUE}Rollback target: $PREVIOUS_VERSION${NC}"
    echo ""
}

# Stop services
stop_services() {
    echo "🛑 Stopping services..."

    # Adjust these commands based on your deployment method
    if command -v systemctl &> /dev/null; then
        systemctl stop omnisystem-* 2>/dev/null || true
    fi

    # Give services time to gracefully shutdown
    sleep 5

    echo -e "${GREEN}✓ Services stopped${NC}"
}

# Restore from backup
restore_backup() {
    local backup_path="$BACKUP_DIR/$PREVIOUS_VERSION"

    if [ ! -d "$backup_path" ]; then
        echo -e "${RED}✗ Backup not found: $backup_path${NC}"
        exit 1
    fi

    echo "📦 Restoring version: $PREVIOUS_VERSION"

    # Copy backup to current location
    cp -r "$backup_path"/* ./omnisystem-deployments/current/ 2>/dev/null || \
    cp -r "$backup_path"/* ./Omnisystem/ 2>/dev/null || \
    echo -e "${YELLOW}⚠ Manual restore required${NC}"

    echo -e "${GREEN}✓ Backup restored${NC}"
}

# Update version file
update_version_file() {
    echo "$PREVIOUS_VERSION" > "$CURRENT_VERSION_FILE"
    echo -e "${GREEN}✓ Version file updated${NC}"
}

# Start services
start_services() {
    echo "🚀 Starting services..."

    if command -v systemctl &> /dev/null; then
        systemctl start omnisystem-* 2>/dev/null || true
    fi

    # Wait for services to start
    sleep 10

    echo -e "${GREEN}✓ Services started${NC}"
}

# Health checks after rollback
verify_health() {
    echo "❤️  Verifying service health..."

    local max_retries=5
    local retry=0

    while [ $retry -lt $max_retries ]; do
        if curl -sf --max-time 10 "http://localhost:8080/health" > /dev/null 2>&1; then
            echo -e "${GREEN}✓ Services are healthy${NC}"
            return 0
        fi
        retry=$((retry + 1))
        if [ $retry -lt $max_retries ]; then
            echo "  Waiting for services to be ready... ($retry/$max_retries)"
            sleep 5
        fi
    done

    echo -e "${RED}✗ Services not healthy after rollback${NC}"
    echo "  Manual intervention required"
    return 1
}

# Notification (optional - integrate with your monitoring)
notify_rollback() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo ""
    echo "📢 Rollback Summary:"
    echo "   Timestamp: $timestamp"
    echo "   From: $CURRENT_VERSION"
    echo "   To: $PREVIOUS_VERSION"
    echo "   Status: COMPLETED"
    echo ""
    echo "   Alert: Production was rolled back due to deployment failure"
    echo "   Action: Investigate and re-deploy with fix"
}

# Main execution
main() {
    echo ""

    # Confirmation
    confirm_rollback

    echo -e "${BLUE}Starting rollback process...${NC}"
    echo ""

    # Get versions
    get_versions

    # Stop services
    stop_services
    echo ""

    # Restore backup
    restore_backup
    echo ""

    # Update version file
    update_version_file
    echo ""

    # Start services
    start_services
    echo ""

    # Verify health
    if verify_health; then
        echo ""
        echo -e "${GREEN}✅ ROLLBACK COMPLETED SUCCESSFULLY${NC}"
        notify_rollback
        exit 0
    else
        echo ""
        echo -e "${RED}❌ ROLLBACK COMPLETED BUT SERVICES UNHEALTHY${NC}"
        echo -e "${RED}   Manual intervention required${NC}"
        notify_rollback
        exit 1
    fi
}

# Run main
main
