#!/bin/bash
# Android Bridge Integration Verification Script
# Verify all components are properly integrated

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASSED=0
FAILED=0

# Function to check if file exists
check_file() {
    local file=$1
    local desc=$2
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $desc"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $desc"
        ((FAILED++))
    fi
}

# Function to check if string appears in file
check_content() {
    local file=$1
    local content=$2
    local desc=$3
    if grep -q "$content" "$file"; then
        echo -e "${GREEN}✓${NC} $desc"
        ((PASSED++))
    else
        echo -e "${RED}✗${NC} $desc"
        ((FAILED++))
    fi
}

echo "============================================"
echo "Android Bridge Integration Verification"
echo "============================================"
echo ""

echo "1. Checking Rust Crate Structure..."
check_file "crates/bonsai-android-bridge/Cargo.toml" "Crate Cargo.toml"
check_file "crates/bonsai-android-bridge/src/lib.rs" "Crate lib.rs"
check_file "crates/bonsai-android-bridge/src/connection.rs" "connection.rs module"
check_file "crates/bonsai-android-bridge/src/device.rs" "device.rs module"
check_file "crates/bonsai-android-bridge/src/discovery.rs" "discovery.rs module"
check_file "crates/bonsai-android-bridge/src/capability.rs" "capability.rs module"
check_file "crates/bonsai-android-bridge/src/security.rs" "security.rs module"
check_file "crates/bonsai-android-bridge/src/streaming.rs" "streaming.rs module"
check_file "crates/bonsai-android-bridge/src/input.rs" "input.rs module"
check_file "crates/bonsai-android-bridge/src/file_sync.rs" "file_sync.rs module"
check_file "crates/bonsai-android-bridge/src/telemetry.rs" "telemetry.rs module"
check_file "crates/bonsai-android-bridge/src/error.rs" "error.rs module"
echo ""

echo "2. Checking Tauri Integration..."
check_file "bonsai-workspace/src-tauri/src/android_bridge_commands.rs" "android_bridge_commands.rs"
check_content "bonsai-workspace/src-tauri/Cargo.toml" "bonsai-android-bridge" "Cargo.toml dependency"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "mod android_bridge_commands" "Module declaration"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_list_devices" "android_list_devices command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_connect" "android_connect command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_start_screen_stream" "android_start_screen_stream command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_stop_screen_stream" "android_stop_screen_stream command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_inject_touch" "android_inject_touch command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_inject_key" "android_inject_key command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_install_app" "android_install_app command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "android_bridge_commands::android_hot_reload" "android_hot_reload command"
check_content "bonsai-workspace/src-tauri/src/lib.rs" "AndroidBridgeState::new()" "AndroidBridgeState initialization"
echo ""

echo "3. Checking Svelte Panel..."
check_file "bonsai-workspace/src/lib/components/AndroidDevicesPanel.svelte" "AndroidDevicesPanel.svelte"
check_content "bonsai-workspace/src/lib/components/AndroidDevicesPanel.svelte" "android_list_devices" "Svelte list_devices call"
check_content "bonsai-workspace/src/lib/components/AndroidDevicesPanel.svelte" "android_connect" "Svelte connect call"
check_content "bonsai-workspace/src/lib/components/AndroidDevicesPanel.svelte" "android_start_screen_stream" "Svelte streaming call"
echo ""

echo "4. Checking MCP Tools..."
check_content "crates/mcp-server/src/tools.rs" "android_list_devices" "MCP android_list_devices"
check_content "crates/mcp-server/src/tools.rs" "android_connect" "MCP android_connect"
check_content "crates/mcp-server/src/tools.rs" "android_start_screen_stream" "MCP android_start_screen_stream"
check_content "crates/mcp-server/src/tools.rs" "android_stop_screen_stream" "MCP android_stop_screen_stream"
check_content "crates/mcp-server/src/tools.rs" "android_inject_touch" "MCP android_inject_touch"
check_content "crates/mcp-server/src/tools.rs" "android_inject_key" "MCP android_inject_key"
check_content "crates/mcp-server/src/tools.rs" "android_install_app" "MCP android_install_app"
check_content "crates/mcp-server/src/tools.rs" "android_hot_reload" "MCP android_hot_reload"
echo ""

echo "5. Checking Tests..."
check_file "crates/bonsai-android-bridge/tests/integration_tests.rs" "integration_tests.rs"
check_content "crates/bonsai-android-bridge/tests/integration_tests.rs" "#\[tokio::test\]" "Async test support"
check_content "crates/bonsai-android-bridge/tests/integration_tests.rs" "test_bridge_initialization" "Bridge init test"
check_content "crates/bonsai-android-bridge/tests/integration_tests.rs" "test_device_connection" "Device connection test"
echo ""

echo "6. Checking Documentation..."
check_file "ANDROID_BRIDGE_INTEGRATION_COMPLETE.md" "Integration report"
check_file "ANDROID_BRIDGE_FINAL_SUMMARY.txt" "Final summary"
check_content "ANDROID_BRIDGE_INTEGRATION_COMPLETE.md" "✅ INTEGRATION COMPLETE" "Status confirmed in report"
echo ""

echo "7. Checking Workspace Configuration..."
check_content "Cargo.toml" "bonsai-android-bridge" "Workspace members"
echo ""

echo "============================================"
echo "Verification Results"
echo "============================================"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. cd /z/Projects/BonsaiWorkspace"
    echo "2. cargo check --workspace"
    echo "3. cargo test --workspace"
    exit 0
else
    echo -e "${RED}✗ Some checks failed!${NC}"
    echo ""
    echo "Please review the output above and fix any issues."
    exit 1
fi
