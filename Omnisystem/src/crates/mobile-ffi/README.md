# Bonsai Mobile FFI - Hardware-Accelerated Video Decoding for Android

A production-grade Rust FFI crate providing zero-copy hardware-accelerated H.264/H.265 video decoding on Android devices via MediaCodec, targeting <10ms decode latency on Redmi Note 12 Pro 5G and other modern Android hardware.

## Features

- **Hardware-Accelerated Decoding**: Leverages Android's MediaCodec for efficient H.264/H.265 decoding
- **Low Latency**: <10ms per-frame decode latency on modern Android devices (Snapdragon 778G+)
- **Zero-Copy Buffers**: Direct buffer management between Kotlin and Rust without unnecessary copying
- **Thread-Safe Metrics**: Real-time performance monitoring with atomic operations
- **Production Grade**: Complete error handling, safe FFI boundaries, comprehensive tests
- **Dual Codec Support**: H.264 (AVC) and H.265 (HEVC) via feature flags

## Performance Targets

| Metric | Target | Test Device |
|--------|--------|------------|
| Decode Latency | <10ms | Redmi Note 12 Pro 5G |
| Throughput (1080p) | 60 FPS sustained | @ 8Mbps bitrate |
| Throughput (720p) | 60 FPS sustained | @ 4Mbps bitrate |
| Throughput (4K) | 30 FPS sustained | @ 20Mbps bitrate |
| Memory Overhead | <20MB | Per decoder instance |
| Frame Drop Rate | <0.1% | Under normal operation |

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│           Kotlin Application Layer                       │
│  (Android UI, MediaPlayer, etc.)                        │
└────────────────┬────────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────────────────────┐
│    BrdfNativeBridge (Kotlin JNI Wrapper)                │
│  - Safe exception handling                              │
│  - Type conversions (Kotlin ↔ Rust)                     │
│  - Resource lifecycle management                        │
└────────────────┬────────────────────────────────────────┘
                 │ (JNI boundary)
                 ↓
┌─────────────────────────────────────────────────────────┐
│    Rust FFI Layer (bonsai-mobile-ffi)                   │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │ Decoder    │  │ Metrics    │  │ Codec      │        │
│  │ Management │  │ Collection │  │ Format     │        │
│  └────────────┘  └────────────┘  └────────────┘        │
└────────────────┬────────────────────────────────────────┘
                 │ (Android NDK)
                 ↓
┌─────────────────────────────────────────────────────────┐
│         MediaCodec (Android Hardware Decoder)           │
│  - H.264/H.265 decoding                                │
│  - Hardware acceleration                                │
│  - Output YUV420 planar format                         │
└─────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Setup Rust Toolchain

```bash
# Add Android targets
rustup target add aarch64-linux-android armv7-linux-android x86_64-linux-android

# Install cargo-ndk for cross-compilation
cargo install cargo-ndk
```

### 2. Build for Android

```bash
# Build for Android ARM64 (Redmi Note 12 Pro 5G)
cd crates/bonsai-mobile-ffi
cargo ndk -t arm64-v8a build --release

# Or cross-compile with cargo-ndk wrapper
cargo-ndk --ndk /path/to/android-ndk -t arm64-v8a build --release
```

This produces: `target/aarch64-linux-android/release/libbonsai_mobile_ffi.so`

### 3. Integrate Into Android Project

**Step 1: Copy Kotlin wrapper**

```bash
# Copy to your Android project
cp BrdfNativeBridge.kt app/src/main/java/com/yourapp/decoder/

# Update package name in the file
sed -i 's/com\.bonsai\.mobile\.decoder/com.yourapp.decoder/' \
    app/src/main/java/com/yourapp/decoder/BrdfNativeBridge.kt
```

**Step 2: Add native library to gradle build**

In `app/build.gradle`:

```gradle
android {
    ndkVersion '25.1.8937393'
    
    defaultConfig {
        externalNativeBuild {
            cmake {
                abiFilters 'arm64-v8a'
            }
        }
    }
}

// Add Rust build task (or apply android_build.gradle)
task buildRustFFI {
    doLast {
        exec {
            commandLine 'cargo-ndk', '-t', 'arm64-v8a', 'build', '--release'
            workingDir = "${project.rootDir}/crates/bonsai-mobile-ffi"
        }
        copy {
            from "${project.rootDir}/crates/bonsai-mobile-ffi/target/aarch64-linux-android/release"
            include 'libbonsai_mobile_ffi.so'
            into "${projectDir}/src/main/jniLibs/arm64-v8a"
        }
    }
}

preBuild.dependsOn buildRustFFI
```

**Step 3: Use in Kotlin code**

```kotlin
import com.yourapp.decoder.BrdfNativeBridge
import com.yourapp.decoder.DecodedFrame

class VideoDecoder(private val mimeType: String = "video/avc") {
    private var bridge: BrdfNativeBridge? = null

    fun initialize(width: Int, height: Int) {
        bridge = BrdfNativeBridge(mimeType, width, height)
        bridge?.setLowLatencyMode(true)  // <10ms latency
    }

    fun decodeFrame(nalData: ByteArray, timestampUs: Long): DecodedFrame? {
        val bridge = bridge ?: return null
        
        val readyFrames = bridge.decodeFrame(nalData, timestampUs)
        return if (readyFrames > 0) {
            bridge.getDecodedFrame()
        } else {
            null
        }
    }

    fun getMetrics() = bridge?.getMetrics()

    fun destroy() {
        bridge?.close()
        bridge = null
    }
}
```

## API Reference

### FFI Entry Points

#### `initDecoder(mime_type, width, height) -> *mut Decoder`

Initialize a new decoder instance.

**Parameters:**
- `mime_type`: C string, either "video/avc" or "video/hevc"
- `width`: Frame width in pixels (must be even, >0)
- `height`: Frame height in pixels (must be even, >0)

**Returns:** Opaque decoder pointer, or NULL on error

**Example:**
```c
void* decoder = initDecoder("video/avc", 1920, 1080);
if (!decoder) {
    fprintf(stderr, "Failed to initialize decoder\n");
}
```

#### `decodeFrame(decoder, input_data, input_size, timestamp_us) -> i32`

Queue an input buffer for decoding.

**Parameters:**
- `decoder`: Decoder pointer from `initDecoder`
- `input_data`: H.264/H.265 NAL unit bytes
- `input_size`: Size in bytes
- `timestamp_us`: Presentation timestamp in microseconds

**Returns:**
- ≥ 0: Number of frames ready for output
- < 0: Error code

**Example:**
```c
int ready = decodeFrame(decoder, nal_data, 2048, 33333);
if (ready > 0) {
    // Frames available for dequeue
}
```

#### `getDecodedFrame(decoder, out_data, out_size, out_width, out_height, out_timestamp) -> i32`

Retrieve next decoded frame.

**Parameters:**
- `decoder`: Decoder pointer
- `out_*`: Output pointers to fill with frame data

**Returns:**
- 0: Success, output pointers filled
- -2: No output buffer available
- < 0: Error

#### `releaseBuffer(decoder, buffer_ptr) -> i32`

Release output buffer back to decoder for reuse.

**Returns:**
- 0: Success
- < 0: Error

#### `getMetrics(decoder, metrics_array[5]) -> i32`

Get performance metrics.

**Output array:**
```c
metrics[0] = avg_decode_latency_us
metrics[1] = max_decode_latency_us
metrics[2] = frames_decoded
metrics[3] = frames_dropped
metrics[4] = last_timestamp_us
```

**Returns:** 0 on success, -1 on error

**Example:**
```c
int64_t metrics[5];
if (getMetrics(decoder, metrics) == 0) {
    printf("Avg latency: %ldus, FPS: %.1f\n", 
           metrics[0],
           (double)metrics[2] * 1e6 / elapsed_us);
}
```

#### `destroyDecoder(decoder) -> void`

Clean up decoder and free resources.

**Safety:** Must not access decoder after calling.

### Kotlin API

```kotlin
class BrdfNativeBridge(mimeType: String, width: Int, height: Int) : AutoCloseable

// Decode a frame
fun decodeFrame(inputData: ByteArray, timestampUs: Long): Int

// Get decoded frame
fun getDecodedFrame(): DecodedFrame?

// Release buffer
fun releaseBuffer(): Unit

// Get metrics
fun getMetrics(): DecoderMetrics

// Low-latency mode
fun setLowLatencyMode(enabled: Boolean): Unit

// Reset decoder
fun reset(): Unit

// Resource cleanup
override fun close(): Unit
```

## Testing

### Unit Tests

```bash
cd crates/bonsai-mobile-ffi
cargo test
```

### Benchmark Tests

```bash
# Run latency benchmarks
cargo bench --bench decode_latency

# Run throughput benchmarks
cargo bench --bench throughput
```

### Integration Tests

```bash
# On Android device via gradle
./gradlew connectedAndroidTest
```

## Troubleshooting

### "Failed to load bonsai_mobile_ffi native library"

**Cause:** The .so library is not in the expected location

**Solution:**
```bash
# Verify library exists in jniLibs
ls app/src/main/jniLibs/arm64-v8a/libbonsai_mobile_ffi.so

# Rebuild library
cargo ndk -t arm64-v8a build --release
```

### "Codec not available: video/avc"

**Cause:** Device doesn't have H.264 decoder (unlikely on modern Android)

**Solution:**
- Verify device supports codec: `MediaCodecList.findDecoderForFormat()`
- Use HEVC fallback: `"video/hevc"`

### High Decode Latency (>20ms)

**Causes:**
1. Low-latency mode not enabled
2. Input buffers too large (>4KB)
3. Device thermal throttling
4. Insufficient device memory

**Solutions:**
```kotlin
bridge.setLowLatencyMode(true)
// Use smaller NAL chunks
// Monitor device temperature
// Check available RAM
```

### Frame Drops

**Cause:** Output buffer queue full (16 max)

**Solution:**
```kotlin
// Dequeue frames faster
val frame = bridge.getDecodedFrame()
if (frame != null) {
    processFrameImmediately(frame)
    bridge.releaseBuffer()
}
```

## Performance Optimization Tips

### 1. Enable Low-Latency Mode

```kotlin
bridge.setLowLatencyMode(true)  // ~2-3ms improvement
```

### 2. Batch Frames

```kotlin
val frames = mutableListOf<DecodedFrame>()
for (i in 0..10) {
    bridge.decodeFrame(nalData[i], timestamps[i])
}
// Dequeue all at once
while (true) {
    val frame = bridge.getDecodedFrame() ?: break
    frames.add(frame)
}
```

### 3. Monitor Metrics

```kotlin
val metrics = bridge.getMetrics()
if (metrics.dropRatePercent() > 1.0) {
    Log.w("Decoder", "High drop rate: ${metrics.dropRatePercent()}%")
    // Reduce input rate or enable low-latency
}
```

### 4. Pre-allocate Buffers

```kotlin
// Reuse ByteArray instances to reduce GC pressure
val inputBuffer = ByteArray(4096)
val nalData = inputBuffer.sliceArray(0 until actualSize)
```

## Common Resolutions and Latency

| Resolution | Bitrate | Latency Target | Device |
|-----------|---------|----------------|--------|
| 480p | 1Mbps | <5ms | Entry-level |
| 720p | 2-4Mbps | <8ms | Mid-range |
| 1080p | 4-8Mbps | <10ms | Snapdragon 778G+ |
| 4K | 15-25Mbps | <15ms | Snapdragon 888+ |

## Supported Formats

### Video Codecs

- **H.264/AVC** (video/avc)
  - All Android devices ≥ API 16
  - Ubiquitous support
  - Recommended for compatibility

- **H.265/HEVC** (video/hevc)
  - Android devices ≥ API 21
  - Better compression
  - Higher performance potential

### Frame Formats

Output frames are always **YUV420 planar**:
```
Y plane:  width × height bytes
U plane:  (width/2) × (height/2) bytes (offset: width*height)
V plane:  (width/2) × (height/2) bytes (offset: width*height + w/2*h/2)
```

Total size: `width * height * 1.5` bytes

## Building for Multiple Architectures

```bash
# ARM64 (most common)
cargo ndk -t arm64-v8a build --release

# ARMv7 (older devices)
cargo ndk -t armeabi-v7a build --release

# x86_64 (emulator/some tablets)
cargo ndk -t x86_64 build --release

# Copy all
for arch in arm64-v8a armeabi-v7a x86_64; do
  cp target/${arch}-mapping[$arch]/release/libbonsai_mobile_ffi.so \
     app/src/main/jniLibs/${arch}/
done
```

Where arch mapping:
- `arm64-v8a` → `aarch64-linux-android`
- `armeabi-v7a` → `armv7-linux-android`
- `x86_64` → `x86_64-linux-android`

## Thread Safety

- **Decoder**: NOT thread-safe. Create separate instances per decoding thread
- **Metrics**: Thread-safe via atomic operations
- **JNI calls**: Must be made from the same thread that initialized the decoder

## Contributing

See main CONTRIBUTING.md for guidelines. Key points for this crate:

1. **Unsafe code**: Document all unsafe blocks with safety invariants
2. **Error handling**: Use meaningful error types, avoid panics
3. **Tests**: Cover happy path and error cases
4. **Benchmarks**: Add performance tests for changes
5. **Documentation**: All public APIs must have examples

## License

See LICENSE file in project root.

## FAQ

**Q: Why not use ExoPlayer or MediaCodec directly?**
A: This crate provides:
- Lower-level control for custom streaming scenarios
- Direct Rust access for performance-critical code
- Integration with other Rust components
- Predictable latency without framework overhead

**Q: Can I use this with OpenGL ES?**
A: Currently outputs YUV420 planar format. Integration with GL texture output can be added via `SurfaceTexture` if needed.

**Q: What about B-frames and adaptive bitrate?**
A: The decoder handles these transparently via MediaCodec. Bitrate adaptation should be at the source level.

**Q: Does this work on iOS?**
A: No, this is Android-specific. iOS decoding would require separate FFI bindings to `VideoToolbox`.
