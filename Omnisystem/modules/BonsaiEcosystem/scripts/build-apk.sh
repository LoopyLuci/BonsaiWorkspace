#!/bin/bash
# Build release APK for Bonsai Remote Desktop

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MOBILE_DIR="$PROJECT_ROOT/android-app"
BUILD_TYPE="${1:-debug}"

echo "=========================================="
echo "Bonsai Remote Desktop APK Builder"
echo "=========================================="
echo "Build Type: $BUILD_TYPE"
echo "Project Root: $PROJECT_ROOT"
echo ""

# Check prerequisites
check_prerequisites() {
    local missing=0

    if ! command -v flutter &> /dev/null; then
        echo "ERROR: Flutter not found. Install from https://flutter.dev/docs/get-started/install"
        missing=1
    fi

    if ! command -v java &> /dev/null; then
        echo "ERROR: Java not found. Install JDK 11+"
        missing=1
    fi

    if ! command -v gradle &> /dev/null && ! [ -f "gradlew" ]; then
        echo "ERROR: Gradle not found"
        missing=1
    fi

    if [ $missing -eq 1 ]; then
        exit 1
    fi

    echo "✓ All prerequisites met"
}

# Get version from pubspec.yaml
get_version() {
    grep "version:" "$MOBILE_DIR/pubspec.yaml" | head -1 | cut -d' ' -f2
}

# Build APK
build_apk() {
    echo ""
    echo "Building APK ($BUILD_TYPE)..."

    cd "$MOBILE_DIR"

    case $BUILD_TYPE in
        debug)
            flutter build apk \
                --debug \
                --verbose
            ;;
        release)
            flutter build apk \
                --release \
                --verbose \
                --split-per-abi
            ;;
        *)
            echo "ERROR: Unknown build type: $BUILD_TYPE"
            exit 1
            ;;
    esac

    if [ $? -eq 0 ]; then
        echo "✓ APK built successfully"
    else
        echo "ERROR: APK build failed"
        exit 1
    fi
}

# Optimize APK
optimize_apk() {
    echo ""
    echo "Optimizing APK..."

    # Use zipalign to optimize the APK
    if command -v zipalign &> /dev/null; then
        APK_PATH="$MOBILE_DIR/build/app/outputs/flutter-apk/app-${BUILD_TYPE}.apk"
        ALIGNED_APK="${APK_PATH%.apk}-aligned.apk"

        zipalign -v 4 "$APK_PATH" "$ALIGNED_APK"

        if [ $? -eq 0 ]; then
            mv "$ALIGNED_APK" "$APK_PATH"
            echo "✓ APK optimized"
        fi
    fi

    # Sign APK if release build
    if [ "$BUILD_TYPE" = "release" ]; then
        sign_apk
    fi
}

# Sign APK with debug key
sign_apk() {
    echo ""
    echo "Signing APK..."

    KEYSTORE="$HOME/.android/debug.keystore"
    APK_PATH="$MOBILE_DIR/build/app/outputs/flutter-apk/app-release.apk"

    if [ ! -f "$KEYSTORE" ]; then
        echo "DEBUG: Creating debug keystore..."
        keytool -genkey -v \
            -keystore "$KEYSTORE" \
            -keyalg RSA \
            -keysize 2048 \
            -validity 10000 \
            -alias android \
            -storepass android \
            -keypass android \
            -dname "CN=Bonsai,O=Bonsai,C=US"
    fi

    jarsigner -verbose \
        -sigalg SHA256withRSA \
        -digestalg SHA-256 \
        -keystore "$KEYSTORE" \
        -storepass android \
        -keypass android \
        "$APK_PATH" \
        android

    echo "✓ APK signed"
}

# Calculate checksums
calculate_checksums() {
    echo ""
    echo "Calculating checksums..."

    APK_PATTERN="$MOBILE_DIR/build/app/outputs/flutter-apk/app-*.apk"

    for apk in $APK_PATTERN; do
        if [ -f "$apk" ]; then
            sha256sum "$apk" > "${apk}.sha256"
            md5sum "$apk" > "${apk}.md5"
            echo "✓ Checksums for $(basename "$apk")"
        fi
    done
}

# Display summary
show_summary() {
    echo ""
    echo "=========================================="
    echo "Build Summary"
    echo "=========================================="

    VERSION=$(get_version)
    echo "Version: $VERSION"
    echo "Build Type: $BUILD_TYPE"

    APK_PATTERN="$MOBILE_DIR/build/app/outputs/flutter-apk/app-*.apk"
    echo ""
    echo "Generated APKs:"

    for apk in $APK_PATTERN; do
        if [ -f "$apk" ]; then
            SIZE=$(du -h "$apk" | cut -f1)
            echo "  - $(basename "$apk") ($SIZE)"

            if [ -f "${apk}.sha256" ]; then
                echo "    SHA256: $(cut -d' ' -f1 "${apk}.sha256")"
            fi
        fi
    done

    echo ""
    echo "✓ Build complete!"
}

# Main execution
main() {
    check_prerequisites
    build_apk
    optimize_apk
    calculate_checksums
    show_summary
}

main
