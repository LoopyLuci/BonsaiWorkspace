# Troubleshooting Guide - Bonsai Mobile FFI

## Build Issues

### 1. "cargo-ndk not found"

**Problem:**
```
error: 'cargo-ndk' is not installed or not in PATH
```

**Solution:**
```bash
# Install cargo-ndk
cargo install cargo-ndk

# Verify installation
cargo ndk --version

# Add to PATH if needed
export PATH="$HOME/.cargo/bin:$PATH"
```

---

### 2. "Cannot find Android NDK"

**Problem:**
```
error: NDK not found at path: /path/to/ndk
```

**Solutions:**

**Option A: Set environment variable**
```bash
# Linux/macOS
export ANDROID_NDK_HOME=/Users/username/Android/ndk/25.1.8937393

# Windows (PowerShell)
$env:ANDROID_NDK_HOME = "C:\Android\ndk\25.1.8937393"

# Windows (Command Prompt)
set ANDROID_NDK_HOME=C:\Android\ndk\25.1.8937393
```

**Option B: Specify explicitly in command**
```bash
cargo ndk --ndk /path/to/android-ndk-r25 -t arm64-v8a build --release
```

**Option C: Find NDK path**
```bash
# Android Studio on macOS
~/Library/Android/sdk/ndk/25.1.8937393

# Android Studio on Linux
~/Android/Sdk/ndk/25.1.8937393

# Android Studio on Windows
C:\Users\Username\AppData\Local\Android\Sdk\ndk\25.1.8937393
```

---

### 3. "Target not found: aarch64-linux-android"

**Problem:**
```
error: toolchain 'stable-aarch64-linux-android' is not installed
```

**Solution:**
```bash
# Add missing Android targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-android
rustup target add x86_64-linux-android

# Verify installation
rustup target list | grep android
```

---

### 4. Linker errors when building

**Problem:**
```
error: linker `aarch64-linux-android-clang` not found
```

**Solution:**

The linker is part of the NDK. Verify NDK is properly installed:

```bash
# Check for clang
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android*

# If missing, reinstall NDK version 25.1.8937393 or compatible
```

---

### 5. "error: the wasm32-unknown-unknown target is not installed"

**Problem:**
```
error: failed to run custom build command for 'bonsai-mobile-ffi'
```

**Solution:**

This is a misconfiguration in build.rs. Ensure you're targeting Android, not WASM:

```bash
# Clear build artifacts
cargo clean

# Explicitly specify Android target
cargo ndk -t arm64-v8a build --release
```

---

### 6. Out of memory during compilation

**Problem:**
```
error: could not compile 'bonsai-mobile-ffi'
```

**Solution:**

```bash
# Reduce parallel jobs
export CARGO_BUILD_JOBS=1

# Disable incremental compilation
export CARGO_INCREMENTAL=0

# Increase available memory and try again
cargo ndk -t arm64-v8a build --release

# Alternative: Build on a machine with more RAM
```

---

## Runtime Issues

### 7. "UnsatisfiedLinkError: Failed to load bonsai_mobile_ffi"

**Problem:**
```
java.lang.UnsatisfiedLinkError: Failed to load bonsai_mobile_ffi
```

**Diagnosis:**
```bash
# Check if .so exists
adb shell "find /data/app -name 'libbonsai_mobile_ffi.so' 2>/dev/null"

# Check library dependencies
adb push libbonsai_mobile_ffi.so /data/local/tmp/
adb shell "ldd /data/local/tmp/libbonsai_mobile_ffi.so"

# Check logcat for detailed error
adb logcat | grep -E "libbonsai|linker|library"
```

**Solutions:**

**1. Verify library exists in APK**
```bash
# Extract APK
unzip app/build/outputs/apk/debug/app-debug.apk -d apk_contents

# Check jniLibs
ls apk_contents/lib/arm64-v8a/libbonsai_mobile_ffi.so

# If missing, rebuild
./gradlew buildRustFFI
./gradlew assembleDebug
```

**2. Check library architecture**
```bash
# On build machine
file crates/bonsai-mobile-ffi/target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
# Output should be: ELF 64-bit LSB shared object, ARM aarch64

# On device
adb shell "file /data/app/.../lib/arm64-v8a/libbonsai_mobile_ffi.so"
```

**3. Verify JNI naming**
```bash
# Library MUST be named exactly: libbonsai_mobile_ffi.so
# System.loadLibrary("bonsai_mobile_ffi") will look for:
# - libbonsai_mobile_ffi.so on Android

ls -la app/src/main/jniLibs/arm64-v8a/
```

**4. Check device ABI compatibility**
```bash
# Get device ABI
adb shell "getprop ro.product.cpu.abi"
# Should output: arm64-v8a

# Get supported ABIs
adb shell "getprop ro.product.cpu.abilist"
```

---

### 8. "Codec not available: video/avc"

**Problem:**
```
java.lang.IllegalStateException: Codec not available: video/avc
```

**Diagnosis:**
```kotlin
// Test codec support on device
val codecs = MediaCodecList(MediaCodecList.REGULAR_CODECS)
val format = MediaFormat.createVideoFormat("video/avc", 1920, 1080)
val codecName = codecs.findDecoderForFormat(format)
if (codecName == null) {
    Log.e("Decoder", "H.264 not supported!")
} else {
    Log.i("Decoder", "Using codec: $codecName")
}
```

**Solutions:**

**1. Try HEVC/H.265 instead**
```kotlin
val bridge = BrdfNativeBridge("video/hevc", 1920, 1080)
```

**2. Reduce resolution for codec compatibility**
```kotlin
// Try lower resolution first
val bridge = BrdfNativeBridge("video/avc", 640, 480)
```

**3. List all available codecs**
```bash
adb shell "dumpsys media.codec | grep 'video/avc'"
```

**4. Check API level**
```kotlin
// H.265 requires API 21+
if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.L) {
    // Can use HEVC
}
```

---

### 9. Decoder crashes immediately

**Problem:**
```
SIGSEGV in libbonsai_mobile_ffi.so
Process crashed: native crash
```

**Diagnosis:**
```bash
# Capture full crash log
adb logcat > crash.log
# Reproduce crash, then analyze

# Get stack trace
adb shell "logcat -d | grep -A 20 'signal 11'"

# Use addr2line to get source locations (requires debug symbols)
aarch64-linux-android-addr2line -e libbonsai_mobile_ffi.so 0x1234567
```

**Solutions:**

**1. Verify decoder initialization**
```kotlin
try {
    val bridge = BrdfNativeBridge("video/avc", 1920, 1080)
    Log.i("Decoder", "Initialization successful")
} catch (e: Exception) {
    Log.e("Decoder", "Initialization failed", e)
    // Check error message
}
```

**2. Check for null pointers**
```kotlin
// Ensure decoder is not null before use
val bridge: BrdfNativeBridge? = try {
    BrdfNativeBridge("video/avc", 1920, 1080)
} catch (e: Exception) {
    null
}

if (bridge == null) {
    Log.e("Decoder", "Failed to create decoder")
    return
}
```

**3. Verify input data**
```kotlin
// Ensure input is valid NAL unit data
val nalData = ByteArray(2048)
if (nalData.isEmpty()) {
    Log.e("Decoder", "Empty input")
    return -1
}

// Check NAL header
if (nalData[0] != 0.toByte()) {
    Log.w("Decoder", "Unexpected NAL header: ${nalData[0]}")
}
```

**4. Reduce resolution for testing**
```kotlin
// Start with small resolution
val bridge = BrdfNativeBridge("video/avc", 320, 240)
```

**5. Enable debug symbols**
```gradle
android {
    buildTypes {
        debug {
            debuggable true
            minifyEnabled false
            proguardFiles getDefaultProguardFile('proguard-android-optimize.txt')
        }
    }
}
```

---

## Performance Issues

### 10. High decode latency (>20ms)

**Problem:**
```
Expected: <10ms
Actual: >20ms per frame
```

**Diagnosis:**
```kotlin
// Check latency
val metrics = bridge.getMetrics()
Log.i("Decoder", "Avg latency: ${metrics.avgDecodeLatencyUs}us")
Log.i("Decoder", "Max latency: ${metrics.maxDecodeLatencyUs}us")

// Check device thermal state
val temp = File("/sys/class/thermal/thermal_zone0/temp").readText()
Log.i("Decoder", "Temperature: ${temp.toLong() / 1000}°C")
```

**Solutions:**

**1. Enable low-latency mode**
```kotlin
bridge.setLowLatencyMode(true)  // ~2-3ms improvement
```

**2. Reduce input buffer size**
```kotlin
// Instead of large NAL units, send slice-aligned NAL units
val sliceSize = 1024  // Smaller chunks
val slices = nalData.chunked(sliceSize)
for (slice in slices) {
    bridge.decodeFrame(slice.toByteArray(), timestamp)
}
```

**3. Process frames without delay**
```kotlin
// Don't defer frame processing
val frame = bridge.getDecodedFrame()
if (frame != null) {
    renderFrameImmediately(frame)  // Process synchronously
    bridge.releaseBuffer()
}
```

**4. Check device power state**
```bash
# Set high-performance mode
adb shell "settings put global power_profile 2"

# Check current frequency
adb shell "cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq"
```

**5. Monitor thermal throttling**
```bash
# Check if thermal throttling is active
adb shell "cat /sys/class/thermal/thermal_zone0/trip_point_0_temp"
adb shell "cat /sys/devices/virtual/thermal/cooling_device0/type"

# Reduce workload if throttling detected
```

**6. Verify no other heavy processes**
```bash
# Check running processes
adb shell "top -n 1"

# Close unnecessary apps
adb shell "am force-stop com.example.other_app"
```

---

### 11. Frame drops (>1%)

**Problem:**
```
Frames decoded: 300
Frames dropped: 5
Drop rate: 1.67%
```

**Solutions:**

**1. Dequeue frames faster**
```kotlin
// Process frames immediately
while (true) {
    val frame = bridge.getDecodedFrame() ?: break
    processFrame(frame)  // Don't defer
    bridge.releaseBuffer()
}
```

**2. Increase output buffer queue** (requires rebuild)
Edit `src/decoder.rs`:
```rust
pub fn with_max_buffers(mut self, count: usize) -> Self {
    self.max_output_buffers = count; // Increase from 16 to 32
    self
}
```

**3. Reduce frame rate**
```kotlin
// Instead of 60 FPS, try 30 FPS
val timestamp = frameNumber * 33_333  // 30 FPS
// Or: frameNumber * 16_667  // 60 FPS
```

**4. Monitor queue depth**
```kotlin
// Check metrics periodically
val metrics = bridge.getMetrics()
if (metrics.dropRatePercent() > 0.5) {
    Log.w("Decoder", "High drop rate: ${metrics.dropRatePercent()}%")
    // Reduce input rate or optimize processing
}
```

**5. Profile CPU usage**
```bash
# Use Android Profiler
# Android Studio > Profiler > CPU > Record
# Look for frame drops and spikes
```

---

### 12. Memory leak

**Problem:**
```
Memory usage increases continuously
After N frames, app crashes with OutOfMemoryError
```

**Diagnosis:**
```bash
# Check memory usage
adb shell "dumpsys meminfo | grep TOTAL"

# Profile heap
adb shell "am dumpheap com.yourapp.app /data/local/tmp/heap.dump"
adb pull /data/local/tmp/heap.dump
# Analyze with Android Studio MAT
```

**Solutions:**

**1. Ensure decoder is destroyed**
```kotlin
override fun onDestroy() {
    bridge?.close()  // CRITICAL - must call close()
    bridge = null
    super.onDestroy()
}

// Or use try-with-resources
try {
    bridge.use {
        // decoder operations
    }
}
```

**2. Release frames immediately**
```kotlin
val frame = bridge.getDecodedFrame()
if (frame != null) {
    processFrame(frame)
    bridge.releaseBuffer()  // Don't forget to release
}
```

**3. Clear output buffers on reset**
```kotlin
// Before seeking or reinitializing
bridge.reset()  // Clears internal queues
```

**4. Monitor native memory**
```bash
# Check native heap usage
adb shell "dumpsys meminfo | grep 'NATIVE HEAP'"

# Profile with malloc tracking
adb shell "setprop libc.malloc.debug all"
adb logcat | grep malloc
```

---

## MediaCodec-Specific Issues

### 13. "Cannot create more than 32 instances"

**Problem:**
```
MediaCodec creation failed - too many instances
```

**Solution:**
```kotlin
// Reuse single decoder instance
// Don't create new decoders in a loop

// Wrong:
for (i in 0..100) {
    val bridge = BrdfNativeBridge(...)  // WRONG!
    bridge.close()
}

// Correct:
val bridge = BrdfNativeBridge(...)
try {
    for (i in 0..100) {
        // Reuse same decoder
        bridge.decodeFrame(data[i], timestamps[i])
    }
} finally {
    bridge.close()
}
```

---

### 14. "Codec input/output buffer errors"

**Problem:**
```
InputBufferError: Cannot find available buffer
OutputBufferError: Queue full
```

**Solutions:**

**1. Check buffer availability before queue**
```kotlin
try {
    val result = bridge.decodeFrame(nalData, timestamp)
    if (result < 0) {
        Log.e("Decoder", "Decode error: $result")
    }
} catch (e: Exception) {
    Log.e("Decoder", "Buffer error", e)
}
```

**2. Reduce input rate**
```kotlin
// Add small delay between frames
Thread.sleep(10)  // 10ms between NAL units
val result = bridge.decodeFrame(nalData, timestamp)
```

**3. Drain output queue regularly**
```kotlin
// Periodically dequeue all available frames
while (true) {
    val frame = bridge.getDecodedFrame() ?: break
    frames.add(frame)
}
```

---

## Device-Specific Issues

### 15. Works on some devices, not others

**Problem:**
```
Works on Redmi Note 12 Pro 5G
Fails on budget phone
```

**Diagnosis:**
```bash
# Check device capabilities
adb shell "getprop | grep hardware"
adb shell "getprop | grep ro.soc"
adb shell "dumpsys media.codec | grep 'video/avc' -A 2"
```

**Solutions:**

**1. Check API level compatibility**
```kotlin
if (Build.VERSION.SDK_INT < 21) {
    // H.265 not supported, use H.264
    val bridge = BrdfNativeBridge("video/avc", 1920, 1080)
}
```

**2. Fallback to software codec**
```kotlin
// If hardware codec unavailable
val format = MediaFormat.createVideoFormat("video/avc", 1920, 1080)
val codecs = MediaCodecList(MediaCodecList.ALL_CODECS)
val codecName = codecs.findDecoderForFormat(format)

if (codecName == null) {
    Log.w("Decoder", "No hardware decoder, using software")
    // Fallback to FFMPEG or software decoder
}
```

**3. Reduce resolution for low-end devices**
```kotlin
val (width, height) = when (Build.MODEL) {
    "Redmi Note 12 Pro 5G" -> Pair(1920, 1080)
    "Redmi Note 11" -> Pair(1280, 720)
    else -> Pair(640, 480)  // Conservative default
}

val bridge = BrdfNativeBridge("video/avc", width, height)
```

**4. Test on Android emulator**
```bash
# Create emulator with specific config
emulator -avd Pixel_API_30 -gpu host

# Deploy and test
./gradlew installDebug
adb shell am start -n com.yourapp/com.yourapp.MainActivity
```

---

## Testing & Validation

### 16. How to verify decoder works correctly

```bash
# 1. Check build succeeded
ls -lh app/build/outputs/apk/debug/app-debug.apk

# 2. Install on device
adb install -r app/build/outputs/apk/debug/app-debug.apk

# 3. Run unit tests
adb shell am instrument -w \
  com.yourapp.test/androidx.test.runner.AndroidJUnitRunner

# 4. Capture logcat
adb logcat | grep -E "BrdfNativeBridge|DecoderMetrics"

# 5. Monitor performance
adb shell top -p $(adb shell pidof com.yourapp) -n 1

# 6. Check for crashes
adb shell logcat | grep -E "SIGSEGV|SEGV_ACCERR"
```

---

## Quick Diagnosis Commands

```bash
# All-in-one diagnostic
echo "=== Device Info ===" && \
adb shell getprop ro.product.model && \
adb shell getprop ro.product.cpu.abi && \
echo "=== Library Check ===" && \
adb shell ls /data/app/com.yourapp-*/lib/arm64-v8a/libbonsai_mobile_ffi.so && \
echo "=== Codec Support ===" && \
adb shell dumpsys media.codec | grep -c "video/avc" && \
echo "=== Logcat Errors ===" && \
adb logcat -d | grep -E "ERROR|FATAL|CRASH" | tail -5
```

---

## Getting Help

If the above solutions don't work:

1. **Collect diagnostics:**
   ```bash
   adb logcat > full_log.txt
   adb dumpsys media.codec > codec_info.txt
   adb shell dumpsys meminfo > memory_info.txt
   ```

2. **Include in bug report:**
   - Device model and Android version
   - Full logcat output
   - Exact steps to reproduce
   - Expected vs actual behavior
   - Timestamp when issue occurred

3. **Check resources:**
   - README.md FAQ section
   - INTEGRATION.md troubleshooting section
   - Android MediaCodec documentation
   - NDK documentation

4. **File issue with details:**
   - Minimal reproducible example
   - All diagnostic outputs
   - Build/runtime environment
