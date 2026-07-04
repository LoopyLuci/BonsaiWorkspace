#!/bin/bash
# Deploy APK to connected Android device

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MOBILE_DIR="$PROJECT_ROOT/android-app"

echo "=========================================="
echo "Bonsai Remote Desktop APK Deployer"
echo "=========================================="
echo ""

# Check prerequisites
check_prerequisites() {
    if ! command -v adb &> /dev/null; then
        echo "ERROR: Android Debug Bridge (adb) not found"
        echo "Install Android SDK Platform Tools from:"
        echo "https://developer.android.com/studio/releases/platform-tools"
        exit 1
    fi

    echo "✓ ADB found: $(adb --version | head -1)"
}

# Check device connection
check_device() {
    echo ""
    echo "Checking for connected devices..."

    DEVICES=$(adb devices | grep -v "List of attached" | grep "device$" | wc -l)

    if [ "$DEVICES" -eq 0 ]; then
        echo "ERROR: No Android devices found"
        echo ""
        echo "Troubleshooting:"
        echo "1. Connect your Android device via USB"
        echo "2. Enable USB debugging: Settings > Developer options > USB debugging"
        echo "3. Grant USB debugging permission when prompted"
        echo "4. Run: adb devices"
        exit 1
    fi

    echo "✓ Found $DEVICES device(s)"
    adb devices -l
}

# Find APK to deploy
find_apk() {
    echo ""
    echo "Looking for APK files..."

    # Look for release APK first
    if [ -f "$MOBILE_DIR/build/app/outputs/flutter-apk/app-release.apk" ]; then
        APK_PATH="$MOBILE_DIR/build/app/outputs/flutter-apk/app-release.apk"
    elif [ -f "$MOBILE_DIR/build/app/outputs/flutter-apk/app-debug.apk" ]; then
        APK_PATH="$MOBILE_DIR/build/app/outputs/flutter-apk/app-debug.apk"
    else
        echo "ERROR: No APK found in build/app/outputs/"
        echo "Run: ./build-apk.sh [release|debug]"
        exit 1
    fi

    echo "✓ Found APK: $(basename "$APK_PATH")"
    APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
    echo "  Size: $APK_SIZE"
}

# Install APK
install_apk() {
    echo ""
    echo "Installing APK..."

    adb install -r "$APK_PATH"

    if [ $? -eq 0 ]; then
        echo "✓ APK installed successfully"
    else
        echo "ERROR: APK installation failed"
        exit 1
    fi
}

# Grant permissions
grant_permissions() {
    echo ""
    echo "Granting required permissions..."

    PERMISSIONS=(
        "android.permission.CAMERA"
        "android.permission.READ_EXTERNAL_STORAGE"
        "android.permission.WRITE_EXTERNAL_STORAGE"
        "android.permission.INTERNET"
        "android.permission.ACCESS_NETWORK_STATE"
        "android.permission.SYSTEM_ALERT_WINDOW"
    )

    for permission in "${PERMISSIONS[@]}"; do
        adb shell pm grant com.bonsai.remote_desktop "$permission" 2>/dev/null || true
    done

    echo "✓ Permissions granted"
}

# Enable accessibility service
enable_accessibility() {
    echo ""
    echo "Enabling accessibility service..."

    adb shell settings put secure enabled_accessibility_services \
        com.bonsai.remote_desktop/.accessibility.RemoteAccessibilityService

    echo "✓ Accessibility service enabled"
}

# Start app
start_app() {
    echo ""
    echo "Starting Bonsai Remote Desktop..."

    adb shell am start -n \
        com.bonsai.remote_desktop/.MainActivity

    echo "✓ App started"
}

# Show device info
show_device_info() {
    echo ""
    echo "Device Information:"
    echo "  Model: $(adb shell getprop ro.product.model)"
    echo "  Brand: $(adb shell getprop ro.product.brand)"
    echo "  Android Version: $(adb shell getprop ro.build.version.release)"
    echo "  API Level: $(adb shell getprop ro.build.version.sdk)"
}

# Verify installation
verify_installation() {
    echo ""
    echo "Verifying installation..."

    INSTALLED=$(adb shell pm list packages | grep "com.bonsai.remote_desktop")

    if [ -z "$INSTALLED" ]; then
        echo "WARNING: Package not found after installation"
        return 1
    fi

    echo "✓ Package verified: $INSTALLED"
    return 0
}

# Show deployment summary
show_summary() {
    echo ""
    echo "=========================================="
    echo "Deployment Complete!"
    echo "=========================================="
    echo ""
    echo "Next steps:"
    echo "1. Open Bonsai Remote Desktop on your device"
    echo "2. Follow the setup wizard to pair with your desktop"
    echo "3. Scan the QR code displayed on your desktop"
    echo ""
    echo "Need help?"
    echo "  Logs: adb logcat | grep -i bonsai"
    echo "  Crash logs: adb logcat | grep -i crash"
    echo ""
}

# Uninstall previous version
uninstall_previous() {
    echo ""
    echo "Checking for previous installation..."

    INSTALLED=$(adb shell pm list packages | grep "com.bonsai.remote_desktop")

    if [ ! -z "$INSTALLED" ]; then
        read -p "Previous version found. Uninstall before installing? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "Uninstalling previous version..."
            adb uninstall com.bonsai.remote_desktop
            echo "✓ Previous version uninstalled"
        fi
    fi
}

# Handle errors during installation
handle_install_error() {
    echo ""
    echo "Installation encountered an issue. Troubleshooting:"
    echo ""
    echo "If you see 'INSTALL_FAILED_INVALID_APK':"
    echo "  - APK might be corrupted"
    echo "  - Rebuild with: ./build-apk.sh release"
    echo ""
    echo "If you see 'INSTALL_FAILED_VERSION_DOWNGRADE':"
    echo "  - Trying to install older version"
    echo "  - Use: adb install -r --downgrade <apk>"
    echo ""
    echo "If you see 'INSTALL_FAILED_INSUFFICIENT_STORAGE':"
    echo "  - Device storage full"
    echo "  - Free up space or use: adb shell pm clear-space"
    echo ""
}

# Main execution
main() {
    check_prerequisites
    check_device
    show_device_info
    find_apk
    uninstall_previous

    if ! install_apk; then
        handle_install_error
        exit 1
    fi

    verify_installation
    grant_permissions
    enable_accessibility
    start_app
    show_summary
}

main
