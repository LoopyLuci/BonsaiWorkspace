#!/bin/bash
# Verify Bonsai Remote Desktop installation and permissions

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=========================================="
echo "Bonsai Remote Desktop Setup Verification"
echo "=========================================="
echo ""

PASS_COUNT=0
FAIL_COUNT=0
WARN_COUNT=0

# Helper functions
print_pass() {
    echo "  ✓ $1"
    ((PASS_COUNT++))
}

print_fail() {
    echo "  ✗ $1"
    ((FAIL_COUNT++))
}

print_warn() {
    echo "  ⚠ $1"
    ((WARN_COUNT++))
}

# Check prerequisites
check_prerequisites() {
    echo "Checking Prerequisites..."

    if command -v adb &> /dev/null; then
        print_pass "ADB installed"
    else
        print_fail "ADB not found (Android Debug Bridge)"
    fi

    if command -v cargo &> /dev/null; then
        print_pass "Rust/Cargo installed"
    else
        print_fail "Cargo not found"
    fi

    if command -v java &> /dev/null; then
        print_pass "Java installed"
    else
        print_fail "Java not found"
    fi

    if command -v flutter &> /dev/null; then
        print_pass "Flutter installed"
    else
        print_warn "Flutter not found (optional for Android app)"
    fi
}

# Check device connection
check_device_connection() {
    echo ""
    echo "Checking Device Connection..."

    DEVICES=$(adb devices | grep -c "device$" || true)

    if [ "$DEVICES" -gt 0 ]; then
        print_pass "Android device connected ($DEVICES device(s))"

        DEVICE_MODEL=$(adb shell getprop ro.product.model)
        DEVICE_API=$(adb shell getprop ro.build.version.sdk)
        DEVICE_ANDROID=$(adb shell getprop ro.build.version.release)

        print_pass "Device: $DEVICE_MODEL (Android $DEVICE_ANDROID, API $DEVICE_API)"
    else
        print_fail "No Android device connected"
    fi
}

# Check device permissions
check_device_permissions() {
    echo ""
    echo "Checking Device Permissions..."

    PERMISSIONS=(
        "android.permission.CAMERA"
        "android.permission.READ_EXTERNAL_STORAGE"
        "android.permission.WRITE_EXTERNAL_STORAGE"
        "android.permission.INTERNET"
        "android.permission.ACCESS_NETWORK_STATE"
        "android.permission.SYSTEM_ALERT_WINDOW"
    )

    for perm in "${PERMISSIONS[@]}"; do
        GRANTED=$(adb shell pm list permissions -g | grep -c "$perm" || true)
        SHORT_PERM=$(echo "$perm" | cut -d. -f3)

        if [ "$GRANTED" -gt 0 ]; then
            print_pass "Permission: $SHORT_PERM"
        else
            print_warn "Permission not granted: $SHORT_PERM (may need manual grant)"
        fi
    done
}

# Check app installation
check_app_installation() {
    echo ""
    echo "Checking App Installation..."

    INSTALLED=$(adb shell pm list packages | grep -c "com.bonsai.remote_desktop" || true)

    if [ "$INSTALLED" -gt 0 ]; then
        print_pass "Bonsai Remote Desktop app installed"

        # Get app version
        APP_VERSION=$(adb shell dumpsys package com.bonsai.remote_desktop | \
            grep "versionName=" | cut -d= -f2)
        print_pass "App version: $APP_VERSION"

        # Check if app is enabled
        APP_ENABLED=$(adb shell pm list packages -d | grep -c "com.bonsai.remote_desktop" || true)
        if [ "$APP_ENABLED" -eq 0 ]; then
            print_pass "App is enabled"
        else
            print_warn "App is disabled"
        fi
    else
        print_warn "Bonsai Remote Desktop app not installed"
    fi
}

# Check accessibility service
check_accessibility_service() {
    echo ""
    echo "Checking Accessibility Service..."

    SERVICE_NAME="com.bonsai.remote_desktop/.accessibility.RemoteAccessibilityService"
    ENABLED=$(adb shell settings get secure enabled_accessibility_services | \
        grep -c "$SERVICE_NAME" || true)

    if [ "$ENABLED" -gt 0 ]; then
        print_pass "Accessibility service enabled"
    else
        print_warn "Accessibility service not enabled (needed for text input)"
        echo "    To enable: Go to Settings > Accessibility > Bonsai Remote Desktop"
    fi
}

# Check storage
check_storage() {
    echo ""
    echo "Checking Device Storage..."

    TOTAL_SPACE=$(adb shell df /data | tail -1 | awk '{print $2}')
    FREE_SPACE=$(adb shell df /data | tail -1 | awk '{print $4}')

    TOTAL_MB=$((TOTAL_SPACE / 1024))
    FREE_MB=$((FREE_SPACE / 1024))

    print_pass "Storage: $FREE_MB MB free / $TOTAL_MB MB total"

    if [ "$FREE_MB" -gt 500 ]; then
        print_pass "Sufficient storage for app (>500 MB required)"
    else
        print_warn "Low storage (recommend >500 MB free)"
    fi
}

# Check network
check_network() {
    echo ""
    echo "Checking Network..."

    # Check if connected to network
    NETWORK=$(adb shell dumpsys telephony.registry | grep -c "mDataConnected=true" || true)

    if [ "$NETWORK" -gt 0 ]; then
        print_pass "Device has network connectivity"
    else
        print_warn "Device may not have network connectivity"
    fi

    # Check WiFi
    WIFI=$(adb shell dumpsys wifi | grep -c "mWifiEnabled = true" || true)

    if [ "$WIFI" -gt 0 ]; then
        print_pass "WiFi enabled"

        # Get WiFi signal strength
        RSSI=$(adb shell dumpsys wifi | grep "mRssi" | tail -1 | awk '{print $NF}')
        print_pass "WiFi signal strength: $RSSI dBm"

        if [ "$RSSI" -gt -70 ]; then
            print_pass "WiFi signal is strong"
        elif [ "$RSSI" -gt -80 ]; then
            print_warn "WiFi signal is fair"
        else
            print_warn "WiFi signal is weak"
        fi
    else
        print_warn "WiFi not enabled"
    fi
}

# Check battery
check_battery() {
    echo ""
    echo "Checking Battery..."

    BATTERY_LEVEL=$(adb shell dumpsys battery | grep "level:" | awk '{print $2}')
    BATTERY_HEALTH=$(adb shell dumpsys battery | grep "health:" | awk '{print $2}')
    BATTERY_TEMP=$(adb shell dumpsys battery | grep "temperature:" | awk '{print $2}')

    TEMP_C=$((BATTERY_TEMP / 10))

    print_pass "Battery level: $BATTERY_LEVEL%"
    print_pass "Battery temperature: ${TEMP_C}°C"

    if [ "$BATTERY_LEVEL" -lt 20 ]; then
        print_warn "Battery low (<20%)"
    elif [ "$BATTERY_LEVEL" -lt 50 ]; then
        print_warn "Battery moderate (<50%)"
    else
        print_pass "Battery level adequate"
    fi

    if [ "$TEMP_C" -gt 50 ]; then
        print_warn "Battery temperature high (>50°C)"
    else
        print_pass "Battery temperature normal"
    fi
}

# Check daemon setup
check_daemon() {
    echo ""
    echo "Checking Desktop Daemon..."

    if curl -s http://localhost:8080/health &> /dev/null; then
        print_pass "Daemon is running on localhost:8080"
    else
        print_warn "Daemon not accessible on localhost:8080"
        echo "    To start: bonsai daemon --enable-remote-desktop"
    fi

    if curl -s http://localhost:8000/health &> /dev/null; then
        print_pass "MCP server is running on localhost:8000"
    else
        print_warn "MCP server not accessible on localhost:8000"
    fi
}

# Check logs
check_logs() {
    echo ""
    echo "Checking Logs for Errors..."

    ERRORS=$(adb logcat -d | grep -i "error\|crash\|exception" | wc -l)

    if [ "$ERRORS" -eq 0 ]; then
        print_pass "No errors in recent logs"
    else
        print_warn "Found $ERRORS errors in logs (this may be normal)"
        echo "    View logs: adb logcat | grep -i bonsai"
    fi
}

# Generate test report
generate_report() {
    echo ""
    echo "=========================================="
    echo "Verification Summary"
    echo "=========================================="
    echo ""
    echo "  Passed:  $PASS_COUNT"
    echo "  Failed:  $FAIL_COUNT"
    echo "  Warnings: $WARN_COUNT"
    echo ""

    if [ "$FAIL_COUNT" -eq 0 ] && [ "$WARN_COUNT" -eq 0 ]; then
        echo "✓ Setup verification passed! You're ready to use Bonsai Remote Desktop."
    elif [ "$FAIL_COUNT" -eq 0 ]; then
        echo "⚠ Setup complete with warnings. Some features may be limited."
    else
        echo "✗ Setup verification failed. Please address the issues above."
    fi

    echo ""
    echo "Next steps:"
    echo "  1. Connect to desktop: :remote connect <peer_id>"
    echo "  2. List available peers: :remote list"
    echo "  3. View help: :remote help"
    echo ""
    echo "For detailed documentation:"
    echo "  - MOBILE_REMOTE_DESKTOP.md"
    echo "  - REDMI_SETUP_GUIDE.md"
    echo "  - MOBILE_REMOTE_API_REFERENCE.md"
}

# Main execution
main() {
    check_prerequisites
    check_device_connection
    check_device_permissions
    check_app_installation
    check_accessibility_service
    check_storage
    check_network
    check_battery
    check_daemon
    check_logs
    generate_report

    return $FAIL_COUNT
}

main
