# Bonsai Mobile FFI Integration Guide

This guide covers detailed integration steps for building and deploying the hardware-accelerated video decoder to Android devices.

## Prerequisites

1. **Android Development Environment**
   - Android Studio 4.1+
   - Android SDK API 21+
   - Android NDK r25.1+
   - Gradle 7.0+

2. **Rust Toolchain**
   - Rust 1.70+
   - `cargo-ndk` for Android cross-compilation
   - Android targets: `aarch64-linux-android`, `armv7-linux-android`, `x86_64-linux-android`

3. **Build Tools**
   - Git
   - PowerShell (for Windows build scripts) or bash (macOS/Linux)

## Step 1: Setup Rust for Android

### 1.1 Install Rust Targets

```bash
# Core target for Redmi Note 12 Pro 5G (and most modern phones)
rustup target add aarch64-linux-android

# Optional: For older devices and emulators
rustup target add armv7-linux-android x86_64-linux-android
```

### 1.2 Install cargo-ndk

```bash
cargo install cargo-ndk
```

Verify installation:
```bash
cargo ndk --version
```

### 1.3 Configure Android NDK

Set up environment variables for your system:

**Windows (PowerShell):**
```powershell
$env:ANDROID_NDK_HOME = "C:\Android\ndk\25.1.8937393"
# Add to permanent env vars if needed
[Environment]::SetEnvironmentVariable("ANDROID_NDK_HOME", "C:\Android\ndk\25.1.8937393", "User")
```

**macOS/Linux (bash):**
```bash
export ANDROID_NDK_HOME=/Users/username/Android/ndk/25.1.8937393
# Add to ~/.bash_profile or ~/.zshrc for persistence
```

## Step 2: Build Rust FFI Library

### 2.1 Build for ARM64 (Primary Target)

```bash
cd crates/bonsai-mobile-ffi

# Clean any previous builds
cargo clean

# Build for Android ARM64 (aarch64-linux-android)
cargo ndk -t arm64-v8a --ndk $ANDROID_NDK_HOME build --release
```

This produces:
- `target/aarch64-linux-android/release/libbonsai_mobile_ffi.so`
- Size: ~2-4MB (release build, unstripped)

### 2.2 Optional: Build for Multiple Architectures

```bash
# ARM64 + ARMv7 + x86_64 emulator support
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 --ndk $ANDROID_NDK_HOME build --release
```

### 2.3 Verify Build Output

```powershell
# Windows
ls target/aarch64-linux-android/release/ | grep "libbonsai_mobile_ffi"

# macOS/Linux
file target/aarch64-linux-android/release/libbonsai_mobile_ffi.so
```

Expected output: `ELF 64-bit LSB shared object, ARM aarch64`

## Step 3: Setup Android Project Structure

### 3.1 Create JNI Library Directory

```bash
# From your Android app root
mkdir -p app/src/main/jniLibs/arm64-v8a
mkdir -p app/src/main/jniLibs/armeabi-v7a  # if building for ARMv7
mkdir -p app/src/main/jniLibs/x86_64        # if building for x86_64
```

### 3.2 Copy Native Libraries

```bash
# Copy compiled .so files
cp crates/bonsai-mobile-ffi/target/aarch64-linux-android/release/libbonsai_mobile_ffi.so \
   app/src/main/jniLibs/arm64-v8a/

# Optional: Copy other architectures
cp crates/bonsai-mobile-ffi/target/armv7-linux-android/release/libbonsai_mobile_ffi.so \
   app/src/main/jniLibs/armeabi-v7a/
cp crates/bonsai-mobile-ffi/target/x86_64-linux-android/release/libbonsai_mobile_ffi.so \
   app/src/main/jniLibs/x86_64/
```

### 3.3 Copy Kotlin Wrapper

```bash
# Create package structure
mkdir -p app/src/main/java/com/yourcompany/decoder/

# Copy wrapper
cp crates/bonsai-mobile-ffi/BrdfNativeBridge.kt \
   app/src/main/java/com/yourcompany/decoder/

# Update package name in the file
# Edit BrdfNativeBridge.kt and change:
# package com.bonsai.mobile.decoder
# to:
# package com.yourcompany.decoder
```

## Step 4: Update Android Gradle Configuration

### 4.1 Update app/build.gradle

Add the following to your app's `build.gradle`:

```gradle
android {
    compileSdk 33  // Or higher
    ndkVersion "25.1.8937393"

    defaultConfig {
        applicationId "com.yourcompany.app"
        minSdk 21      // HEVC support; use 16 for H.264-only
        targetSdk 33
        versionCode 1
        versionName "1.0"

        // Configure native build
        externalNativeBuild {
            cmake {
                // Optional: if using CMakeLists.txt
                // cppFlags "-std=c++17"
                // abiFilters 'arm64-v8a'
            }
        }

        // Configure for JNI
        ndk {
            // Specify ABIs to include in APK
            abiFilters 'arm64-v8a'
            // Add others if multi-arch: 'armeabi-v7a', 'x86_64'
        }
    }

    buildTypes {
        release {
            minifyEnabled true
            proguardFiles getDefaultProguardFile('proguard-android-optimize.txt'), 'proguard-rules.pro'
        }
        debug {
            minifyEnabled false
        }
    }

    packagingOptions {
        // Exclude duplicate libraries if present
        pickFirst 'lib/arm64-v8a/libbonsai_mobile_ffi.so'
    }
}

// Optional: Automated build task
task buildRustFFI {
    doLast {
        println "Building Rust FFI for Android..."
        exec {
            commandLine 'cargo', 'ndk', 
                '-t', 'arm64-v8a',
                '--ndk', System.getenv('ANDROID_NDK_HOME') ?: 'YOUR_NDK_PATH',
                'build', '--release'
            workingDir = file("${rootDir}/crates/bonsai-mobile-ffi")
        }

        // Copy to jniLibs
        copy {
            from "${rootDir}/crates/bonsai-mobile-ffi/target/aarch64-linux-android/release"
            include 'libbonsai_mobile_ffi.so'
            into "${projectDir}/src/main/jniLibs/arm64-v8a"
        }
        
        println "✓ Rust FFI built and copied"
    }
}

// Hook build process
preBuild.dependsOn buildRustFFI
```

### 4.2 Update ProGuard Rules (if minifying)

Add to `app/proguard-rules.pro`:

```
# Keep Kotlin classes
-keep class com.yourcompany.decoder.** { *; }
-keep class com.yourcompany.decoder.BrdfNativeBridge { *; }
-keep class com.yourcompany.decoder.DecodedFrame { *; }
-keep class com.yourcompany.decoder.DecoderMetrics { *; }

# Keep native method signatures
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep exception handling
-dontwarn java.lang.invoke.*
```

## Step 5: Use in Kotlin Code

### 5.1 Basic Video Decoder

```kotlin
package com.yourcompany.video

import android.util.Log
import com.yourcompany.decoder.BrdfNativeBridge
import com.yourcompany.decoder.DecodedFrame
import com.yourcompany.decoder.DecoderMetrics

class VideoDecoder(
    private val mimeType: String = "video/avc",
    private val width: Int,
    private val height: Int
) : AutoCloseable {
    companion object {
        private const val TAG = "VideoDecoder"
    }

    private var bridge: BrdfNativeBridge? = null

    init {
        try {
            bridge = BrdfNativeBridge(mimeType, width, height)
            Log.d(TAG, "Decoder initialized: $mimeType ${width}x${height}")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize decoder", e)
            throw e
        }
    }

    /**
     * Decode a NAL unit
     */
    fun decodeFrame(nalData: ByteArray, timestampUs: Long): List<DecodedFrame> {
        val bridge = bridge ?: throw IllegalStateException("Decoder not initialized")
        
        val readyFrames = mutableListOf<DecodedFrame>()
        
        try {
            val count = bridge.decodeFrame(nalData, timestampUs)
            for (i in 0 until count) {
                val frame = bridge.getDecodedFrame()
                if (frame != null) {
                    readyFrames.add(frame)
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Decode error", e)
            throw e
        }
        
        return readyFrames
    }

    /**
     * Get decoder metrics
     */
    fun getMetrics(): DecoderMetrics? {
        return try {
            bridge?.getMetrics()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to get metrics", e)
            null
        }
    }

    /**
     * Enable low-latency mode
     */
    fun setLowLatencyMode(enabled: Boolean) {
        try {
            bridge?.setLowLatencyMode(enabled)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to set low-latency mode", e)
        }
    }

    /**
     * Reset decoder
     */
    fun reset() {
        try {
            bridge?.reset()
            Log.d(TAG, "Decoder reset")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to reset decoder", e)
        }
    }

    /**
     * Cleanup resources
     */
    override fun close() {
        try {
            bridge?.close()
            bridge = null
            Log.d(TAG, "Decoder closed")
        } catch (e: Exception) {
            Log.e(TAG, "Error closing decoder", e)
        }
    }
}
```

### 5.2 Example: Real-time Video Streaming

```kotlin
package com.yourcompany.streaming

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.util.Log
import android.view.SurfaceView
import com.yourcompany.video.VideoDecoder
import kotlinx.coroutines.*

class StreamingVideoPlayer(
    private val surfaceView: SurfaceView,
    private val width: Int,
    private val height: Int
) : AutoCloseable {
    companion object {
        private const val TAG = "StreamingVideoPlayer"
    }

    private val decoder = VideoDecoder("video/avc", width, height)
    private var playbackJob: Job? = null
    private val scope = CoroutineScope(Dispatchers.Default + Job())

    init {
        decoder.setLowLatencyMode(true)  // <10ms latency
    }

    /**
     * Queue NAL unit for decoding
     */
    fun queueNalUnit(nalData: ByteArray, timestampUs: Long) {
        scope.launch {
            try {
                val frames = decoder.decodeFrame(nalData, timestampUs)
                frames.forEach { frame ->
                    renderFrame(frame.width, frame.height)
                }
            } catch (e: Exception) {
                Log.e(TAG, "Decode error", e)
            }
        }
    }

    /**
     * Log metrics periodically
     */
    fun startMetricsLogging(intervalMs: Long = 1000) {
        playbackJob?.cancel()
        playbackJob = scope.launch {
            while (isActive) {
                delay(intervalMs)
                val metrics = decoder.getMetrics()
                if (metrics != null) {
                    Log.d(TAG, """
                        Metrics:
                          FPS: ${metrics.fps().toInt()}
                          Avg Latency: ${metrics.avgDecodeLatencyUs}μs
                          Max Latency: ${metrics.maxDecodeLatencyUs}μs
                          Drop Rate: ${metrics.dropRatePercent().toInt()}%
                    """.trimIndent())
                }
            }
        }
    }

    private fun renderFrame(width: Int, height: Int) {
        val holder = surfaceView.holder
        val canvas = try {
            holder.lockCanvas()
        } catch (e: Exception) {
            Log.w(TAG, "Failed to lock canvas", e)
            return
        }

        canvas?.let {
            it.drawColor(android.graphics.Color.BLACK)
            val paint = Paint().apply {
                color = android.graphics.Color.GREEN
                textSize = 24f
            }
            it.drawText("${width}x${height}", 20f, 50f, paint)
            holder.unlockCanvasAndPost(it)
        }
    }

    override fun close() {
        playbackJob?.cancel()
        scope.cancel()
        decoder.close()
    }
}
```

## Step 6: Testing

### 6.1 Unit Tests

```kotlin
package com.yourcompany.decoder.test

import org.junit.Test
import org.junit.Assert.*
import com.yourcompany.decoder.BrdfNativeBridge

class DecoderTest {
    @Test
    fun testDecoderInitialization() {
        val decoder = BrdfNativeBridge("video/avc", 1920, 1080)
        assertTrue(decoder.isValid())
        decoder.close()
    }

    @Test
    fun testDecodeFrame() {
        val decoder = BrdfNativeBridge("video/avc", 1920, 1080)
        decoder.setLowLatencyMode(true)
        
        val nalData = ByteArray(2048)
        val result = decoder.decodeFrame(nalData, 33_333)
        assertTrue(result >= 0)
        
        decoder.close()
    }

    @Test
    fun testMetrics() {
        val decoder = BrdfNativeBridge("video/avc", 1920, 1080)
        
        val nalData = ByteArray(2048)
        decoder.decodeFrame(nalData, 33_333)
        
        val metrics = decoder.getMetrics()
        assertNotNull(metrics)
        assertTrue(metrics!!.framesDecoded > 0)
        
        decoder.close()
    }
}
```

### 6.2 Integration Tests on Device

```bash
# Build and run on connected device
./gradlew connectedDebugAndroidTest

# Or with specific test:
./gradlew connectedDebugAndroidTest \
    -Pandroid.testInstrumentationRunnerArguments.class=\
com.yourcompany.decoder.test.DecoderTest
```

## Step 7: Performance Verification

### 7.1 Measure Decode Latency

Use `adb` to capture metrics:

```bash
# List connected devices
adb devices

# Pull metrics log
adb pull /data/local/tmp/decoder_metrics.log

# View in real-time
adb logcat | grep "DecoderMetrics"
```

### 7.2 Monitor Resource Usage

```bash
# CPU usage
adb shell top -n 1 | grep "video\|decoder"

# Memory usage
adb shell dumpsys meminfo | grep -A 5 "NATIVE HEAP"

# Frame drops
adb shell "dumpsys SurfaceFlinger | grep 'Frame drops'"
```

### 7.3 Thermal Monitoring

```bash
# Check device temperature
adb shell cat /sys/class/thermal/thermal_zone*/temp

# Set power profile to high-performance
adb shell "settings put global power_profile 2"
```

## Troubleshooting

### Build Issues

#### Error: "Cannot find NDK"

```powershell
# Windows: Check NDK path
$env:ANDROID_NDK_HOME

# Or specify explicitly
cargo ndk -t arm64-v8a `
  --ndk "C:\Android\ndk\25.1.8937393" `
  build --release
```

#### Error: "Target not found"

```bash
rustup target add aarch64-linux-android
rustup target list | grep android
```

#### Error: "cargo-ndk not found"

```bash
cargo install cargo-ndk
which cargo-ndk  # Verify installation
```

### Runtime Issues

#### Library Not Found

**Symptom:** `UnsatisfiedLinkError: Failed to load libbonsai_mobile_ffi.so`

**Solution:**
```bash
# Verify .so exists
adb shell ls /data/app/com.yourcompany.app-*/lib/arm64-v8a/

# Check library dependencies
adb shell ldd /data/app/com.yourcompany.app-*/lib/arm64-v8a/libbonsai_mobile_ffi.so

# Force reload
adb shell am force-stop com.yourcompany.app
adb shell rm -rf /data/app/com.yourcompany.app-*
adb install app/build/outputs/apk/debug/app-debug.apk
```

#### Decoder Crashes

**Symptom:** App crashes immediately after calling `decodeFrame()`

**Solution:**
1. Check logcat for detailed error:
   ```bash
   adb logcat "*:S" BrdfNativeBridge:V
   ```

2. Verify codec support:
   ```kotlin
   val codecs = MediaCodecList(MediaCodecList.REGULAR_CODECS)
   val format = MediaFormat.createVideoFormat("video/avc", 1920, 1080)
   val codecName = codecs.findDecoderForFormat(format)
   if (codecName == null) {
       Log.e("Decoder", "H.264 not supported on this device")
   }
   ```

3. Reduce resolution for testing:
   ```kotlin
   val decoder = VideoDecoder("video/avc", 640, 480)
   ```

### Performance Issues

#### High Latency (>20ms)

**Solutions:**
1. Enable low-latency mode:
   ```kotlin
   decoder.setLowLatencyMode(true)
   ```

2. Reduce NAL unit size:
   ```kotlin
   // Instead of whole frame, send per-slice NAL units
   val nalUnits = splitIntoSlices(frameData)
   for (nal in nalUnits) {
       decoder.decodeFrame(nal, timestamp)
   }
   ```

3. Check device thermal state:
   ```bash
   adb shell cat /sys/class/thermal/thermal_zone0/temp
   ```

#### Frame Drops

**Solutions:**
1. Process frames faster:
   ```kotlin
   // Dequeue immediately after decoding
   val frame = decoder.getDecodedFrame()
   if (frame != null) {
       processFrameNow(frame)  // Don't defer
       decoder.releaseBuffer()
   }
   ```

2. Increase output buffer queue:
   - Modified in `DecoderConfig.max_output_buffers`
   - Rebuild Rust library and reinstall APK

3. Monitor drop rate:
   ```kotlin
   val metrics = decoder.getMetrics()
   if (metrics.dropRatePercent() > 1.0) {
       Log.w("Decoder", "High drop rate!")
   }
   ```

## Performance Tuning Checklist

- [ ] Build with `--release` (not debug)
- [ ] Enable low-latency mode for real-time apps
- [ ] Monitor metrics via `getMetrics()`
- [ ] Verify codec support for device
- [ ] Test on target device (Redmi Note 12 Pro 5G)
- [ ] Check device temperature during testing
- [ ] Profile with Android Profiler
- [ ] Set device to high-performance power profile
- [ ] Use dedicated thread for decoding
- [ ] Process frames without delay

## Next Steps

1. **Integrate with streaming client** - Adapt from your media framework
2. **Add OpenGL output** - Render YUV420 to texture for GPU processing
3. **Implement bitrate adaptation** - Monitor metrics and adjust input
4. **Add HW-accelerated encoding** - Use MediaCodec for reverse path
5. **Port to desktop platforms** - Use FFMPEG or GStreamer instead of MediaCodec

## Support

For issues or questions:
1. Check README.md FAQ section
2. Review test cases in `crates/bonsai-mobile-ffi/tests/`
3. File issues on project repository
4. Check Android MediaCodec documentation
