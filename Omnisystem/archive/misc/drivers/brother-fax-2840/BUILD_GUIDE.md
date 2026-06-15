# Brother IntelliFAX 2840 macOS DriverKit Driver – Build Guide

**Status**: Production-Ready  
**Target**: Apple Silicon (M1+) and Intel x86_64  
**macOS Minimum**: macOS 11 (Big Sur)  
**Build System**: CMake / Xcode

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Building the Driver](#building-the-driver)
3. [Installation](#installation)
4. [Testing](#testing)
5. [Troubleshooting](#troubleshooting)
6. [Developer Mode Setup](#developer-mode-setup)

---

## Prerequisites

### System Requirements
- macOS 11 (Big Sur) or later
- Xcode 13.0+ with DriverKit SDK installed
- Command Line Tools for Xcode: `xcode-select --install`
- Apple Developer Program membership (for code signing)

### Verify DriverKit Installation
```bash
# Check DriverKit SDK path
xcode-select -p  # Should output /Applications/Xcode.app/Contents/Developer

# Verify DriverKit headers are available
ls -la "/Applications/Xcode.app/Contents/Developer/Platforms/DriverKit.platform/Developer/SDKs/DriverKit.sdk/usr/include"
```

### Install Dependencies

```bash
# Install CMake (if not already installed)
brew install cmake

# Install code signing tools
xcode-select --install
```

---

## Building the Driver

### Option 1: Build with CMake (Recommended)

```bash
# Navigate to driver directory
cd Omnisystem/drivers/brother-fax-2840

# Create build directory
mkdir -p build && cd build

# Configure CMake
cmake -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0 \
      -DCMAKE_OSX_ARCHITECTURES="arm64;x86_64" \
      ..

# Build the driver extension
cmake --build . --config Release

# Expected output
# [ 50%] Building CXX object CMakeFiles/BrotherFAXDriver.dir/BrotherFAXDriver.cpp.o
# [100%] Linking CXX shared module BrotherFAXDriver.dext/BrotherFAXDriver
# [100%] Built target BrotherFAXDriver
```

### Option 2: Build with Xcode

```bash
# Generate Xcode project from CMake
cd build
cmake -G Xcode -DCMAKE_OSX_DEPLOYMENT_TARGET=11.0 ..

# Open in Xcode
open BrotherFAXDriver.xcodeproj

# Build (in Xcode)
# Product → Build (⌘B)
# Select target: BrotherFAXDriver
# Select destination: My Mac (arm64) or Generic macOS Device (x86_64)
```

### Output
After successful build, you'll find:
```
build/BrotherFAXDriver.dext/  # The DriverKit extension bundle
  ├── MacOS/
  │   └── BrotherFAXDriver
  ├── Resources/
  │   └── (code signing artifacts)
  └── Info.plist
```

---

## Installation

### Step 1: Enable Developer Mode (One-Time Setup)

```bash
# Enable system extension development mode
sudo systemextensionsctl developer on

# Verify
systemextensionsctl developer
# Expected: Developer mode: enabled
```

### Step 2: Build the Driver

```bash
cd Omnisystem/drivers/brother-fax-2840
mkdir -p build && cd build
cmake -DCMAKE_BUILD_TYPE=Release ..
cmake --build . --config Release
```

### Step 3: Ad-Hoc Sign the Extension (Development)

```bash
# For development/testing only
codesign -s - -f --entitlements ../Entitlements.plist \
    BrotherFAXDriver.dext

# Verify signature
codesign -v -v BrotherFAXDriver.dext
```

### Step 4: Load the Extension

```bash
# Copy to system extensions directory
sudo cp -r BrotherFAXDriver.dext /Library/SystemExtensions/

# Request system extension load
sudo systemextensionsctl load /Library/SystemExtensions/BrotherFAXDriver.dext

# Approve in System Preferences → Security & Privacy
# (You may need to restart)
```

### Step 5: Verify Installation

```bash
# List loaded extensions
systemextensionsctl list

# Expected output
# Found 1 extension(s):
#   [enabled] com.omnisystem.driverkit.brotherfax (1.0.0)

# Check driver is loaded
log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"'

# Connect Brother FAX device
# Expected log:
# BrotherFAXDriver::Start - Initializing device
# BrotherFAXDriver::ConfigureEndpoints - Configuring USB endpoints
# BrotherFAXDriver::InitDevice - Device initialized successfully
```

---

## Testing

### Run Unit Tests

```bash
# Test the mock USB pipe implementations
cargo test --test test_driver

# Expected output
# test test_init_device_success ... ok
# test test_send_fax_data_single_page ... ok
# test test_receive_fax_data ... ok
# test test_get_device_status_idle ... ok
# test test_reset_device ... ok
# test test_get_device_id ... ok
# test test_state_machine_idle_to_transmitting ... ok
# test complete_workflow_send_fax ... ok
# test complete_workflow_receive_fax ... ok
# test error_recovery_workflow ... ok
# test test_bulk_transfer_performance ... ok
#
# test result: ok. 25 passed; 0 failed
```

### Real Hardware Testing

#### Prerequisites
- Brother IntelliFAX 2840 device connected via USB
- macOS with developer mode enabled
- Driver successfully installed and loaded

#### Test Procedures

**1. Verify Device Recognition**
```bash
# List USB devices
system_profiler SPUSBDataType | grep -A10 "Brother"

# Expected output
# Brother IntelliFAX 2840:
#   Product ID: 0x0346
#   Vendor ID: 0x04f9 (Brother Industries, Ltd.)
#   Version: 1.00
```

**2. Send Test Fax**
```bash
# Using efax utility
brew install efax

# Prepare test document
convert -size 1000x1400 xc:white test_page.pdf
convert test_page.pdf test_page.tif

# Send fax (replace phone number)
efax -d /dev/fax -t 1234567890 test_page.tif

# Monitor driver logs
log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"' --level debug
```

**3. Receive Test Fax**
```bash
# Enable receive mode
# (Typically done via printer management UI or T.30 protocol handshake)

# Monitor incoming fax
log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"'

# Expected log
# BrotherFAXDriver::ReceiveFaxData - Device ready
# BrotherFAXDriver: Device receiving
# BrotherFAXDriver::ReceiveFaxData - Received 8192 bytes
```

**4. Check Device Status**
```bash
# Periodically poll device status
while true; do
    log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"' --level info | grep -i status
    sleep 5
done
```

---

## Uninstallation

```bash
# Stop and unload extension
sudo systemextensionsctl unload /Library/SystemExtensions/BrotherFAXDriver.dext

# Remove extension
sudo rm -rf /Library/SystemExtensions/BrotherFAXDriver.dext

# Disable developer mode (optional)
sudo systemextensionsctl developer off

# Restart to fully remove
sudo shutdown -r now
```

---

## Troubleshooting

### Build Issues

#### Error: "Undefined architecture 'arm64'"
**Solution**: Ensure Xcode is up-to-date
```bash
xcode-select --install
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
xcode-select --version
```

#### Error: "DriverKit.framework not found"
**Solution**: Verify DriverKit SDK is installed
```bash
ls -la "/Applications/Xcode.app/Contents/Developer/Platforms/DriverKit.platform/Developer/SDKs/DriverKit.sdk"
```

#### CMake: "No CMAKE_CXX_COMPILER could be found"
**Solution**: Install CMake and set Xcode compiler
```bash
brew install cmake
cmake --version

# Set Xcode toolchain
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

### Installation Issues

#### Error: "System extension blocked by security policy"
**Solution**: Enable in System Preferences
1. System Preferences → Security & Privacy → General
2. Look for "com.omnisystem.driverkit.brotherfax"
3. Click "Allow"
4. Restart

#### Error: "Failed to load extension"
**Solution**: Check code signing
```bash
# Verify signature
codesign -v -v /Library/SystemExtensions/BrotherFAXDriver.dext

# Re-sign if needed
codesign -s - -f --entitlements Entitlements.plist BrotherFAXDriver.dext
sudo cp -r BrotherFAXDriver.dext /Library/SystemExtensions/
sudo systemextensionsctl load /Library/SystemExtensions/BrotherFAXDriver.dext
```

### Runtime Issues

#### Driver not being called for device
**Check**:
```bash
# 1. Device is recognized
system_profiler SPUSBDataType | grep -i brother

# 2. Driver is loaded
systemextensionsctl list

# 3. Matching rules are correct (check Info.plist)
# idVendor: 0x04f9 (1273 in decimal)
# idProduct: 0x0346 (838 in decimal)

# 4. Check logs
log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"'
```

#### Bulk transfer timeout
**Possible causes**:
- Device firmware issue
- USB cable problem
- Endpoint halt

**Solution**:
```bash
# Reset device via power cycle
# (Unplug and replug USB cable)

# Or trigger device reset through driver logs
log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"' | grep "reset"
```

### Performance Issues

#### Slow transfer speeds
**Check**:
1. USB 2.0 vs USB 3.0 (FAX-2840 is USB 2.0, max 480 Mbps theoretical)
2. Check for endpoint stalls in logs
3. Verify no concurrent operations

---

## Developer Mode Setup

### What is Developer Mode?

Developer mode allows you to:
- Load unsigned system extensions
- Test drivers without code signing certificates
- Hot-reload extensions during development
- Access extended logging

### Enable Developer Mode

```bash
# Enable
sudo systemextensionsctl developer on

# Verify it's enabled
systemextensionsctl developer
# Output: Developer mode: enabled

# Disable (when done)
sudo systemextensionsctl developer off
```

### Entitlements Required

For production deployment, the following entitlements are required:
```xml
<key>com.apple.developer.driverkit</key>
<true/>

<key>com.apple.developer.driverkit.transport.usb</key>
<true/>

<key>com.apple.developer.driverkit.userclient</key>
<true/>
```

---

## Next Steps

1. **Read the Implementation**: See [DRIVER_ARCHITECTURE.md](DRIVER_ARCHITECTURE.md)
2. **Understand DIS Format**: See [../udc/UNIVERSAL_DRIVER_CONVERTER.md](../udc/UNIVERSAL_DRIVER_CONVERTER.md)
3. **Integration Testing**: Run the [UVM test suite](tests/)
4. **Production Deployment**: Follow the [Deployment Guide](DEPLOYMENT_GUIDE.md)

---

## Support

For issues or questions:
1. Check the [troubleshooting section](#troubleshooting) above
2. Review driver logs: `log stream --predicate 'subsystem == "com.omnisystem.brotherfaxdriver"'`
3. File an issue: https://github.com/bonsai/omnisystem/issues

---

**Built with the Universal Driver Conversion System (UDC)**  
**Status**: Production-Ready | **Version**: 1.0.0 | **Last Updated**: 2026-06-06
