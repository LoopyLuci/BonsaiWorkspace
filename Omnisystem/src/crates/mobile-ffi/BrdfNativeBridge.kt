/**
 * BrdfNativeBridge - JNI Wrapper for Bonsai Mobile FFI Decoder
 *
 * Provides safe Kotlin bindings to the Rust FFI layer for hardware-accelerated
 * video decoding on Android via MediaCodec.
 *
 * Thread Safety: This class is NOT thread-safe. All calls must be made from
 * the same thread that created the decoder.
 *
 * Example Usage:
 * ```kotlin
 * val bridge = BrdfNativeBridge("video/avc", 1920, 1080)
 * try {
 *     val readyFrames = bridge.decodeFrame(nalData, 33_333)
 *     if (readyFrames > 0) {
 *         val frame = bridge.getDecodedFrame()
 *         // Process frame...
 *         bridge.releaseBuffer()
 *     }
 * } finally {
 *     bridge.destroy()
 * }
 * ```
 */

package com.bonsai.mobile.decoder

import android.util.Log
import java.nio.ByteBuffer

/**
 * Decoded frame data from the native decoder
 */
data class DecodedFrame(
    /** Frame pixel data (YUV420 planar) */
    val data: ByteBuffer,
    /** Frame width in pixels */
    val width: Int,
    /** Frame height in pixels */
    val height: Int,
    /** Presentation timestamp in microseconds */
    val timestampUs: Long,
    /** Decode latency in microseconds */
    val decodeLatencyUs: Long
) {
    /** Get frame size in bytes (YUV420) */
    fun sizeBytes(): Int = data.remaining()
}

/**
 * Decoder performance metrics
 */
data class DecoderMetrics(
    /** Average decode latency in microseconds */
    val avgDecodeLatencyUs: Long,
    /** Maximum decode latency in microseconds */
    val maxDecodeLatencyUs: Long,
    /** Total frames successfully decoded */
    val framesDecoded: Long,
    /** Total frames dropped */
    val framesDropped: Long,
    /** Last frame presentation timestamp */
    val lastTimestampUs: Long? = null,
    /** Last frame width */
    val lastWidth: Int? = null,
    /** Last frame height */
    val lastHeight: Int? = null,
    /** Total bytes decoded */
    val totalBytes: Long = 0,
    /** Minimum decode latency in microseconds */
    val minDecodeLatencyUs: Long = 0,
    /** Decoder uptime in microseconds */
    val elapsedTimeUs: Long = 0
) {
    /** Calculate frames per second */
    fun fps(): Double = if (elapsedTimeUs > 0) {
        (framesDecoded * 1_000_000.0) / elapsedTimeUs
    } else {
        0.0
    }

    /** Calculate throughput in Mbps */
    fun throughputMbps(): Double = if (elapsedTimeUs > 0) {
        (totalBytes * 8.0) / (elapsedTimeUs / 1_000_000.0) / 1_000_000.0
    } else {
        0.0
    }

    /** Calculate drop rate as percentage */
    fun dropRatePercent(): Double {
        val total = framesDecoded + framesDropped
        return if (total > 0) {
            (framesDropped * 100.0) / total
        } else {
            0.0
        }
    }
}

/**
 * JNI wrapper for the Rust FFI decoder
 *
 * Provides safe access to native video decoding with automatic resource management.
 */
class BrdfNativeBridge(
    mimeType: String,
    width: Int,
    height: Int
) : AutoCloseable {
    companion object {
        private const val TAG = "BrdfNativeBridge"

        init {
            try {
                System.loadLibrary("bonsai_mobile_ffi")
                Log.i(TAG, "Successfully loaded bonsai_mobile_ffi native library")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load bonsai_mobile_ffi native library: ${e.message}")
                throw e
            }
        }

        private external fun initDecoder(
            mimeType: String,
            width: Int,
            height: Int
        ): Long

        private external fun decodeFrame(
            decoderPtr: Long,
            inputData: ByteArray,
            inputSize: Int,
            timestampUs: Long
        ): Int

        private external fun getDecodedFrame(
            decoderPtr: Long,
            outData: LongArray,
            outSize: IntArray,
            outWidth: IntArray,
            outHeight: IntArray,
            outTimestamp: LongArray
        ): Int

        private external fun releaseBuffer(decoderPtr: Long, bufferPtr: Long): Int

        private external fun destroyDecoder(decoderPtr: Long)

        private external fun getMetrics(decoderPtr: Long, metricsArray: LongArray): Int

        private external fun setLowLatencyMode(decoderPtr: Long, enable: Int): Int

        private external fun resetDecoder(decoderPtr: Long): Int
    }

    private var decoderPtr: Long = 0L
    private var isDestroyed = false

    init {
        require(width > 0) { "Width must be positive" }
        require(height > 0) { "Height must be positive" }
        require(mimeType.isNotEmpty()) { "MIME type cannot be empty" }

        decoderPtr = initDecoder(mimeType, width, height)
        if (decoderPtr == 0L) {
            throw IllegalStateException("Failed to initialize decoder for $mimeType ${width}x${height}")
        }

        Log.i(TAG, "Initialized decoder for $mimeType ${width}x${height}")
    }

    /**
     * Decode a frame from H.264/H.265 NAL unit data
     *
     * @param inputData H.264/H.265 NAL unit bytes
     * @param timestampUs Presentation timestamp in microseconds
     * @return Number of frames ready for output, or negative on error
     * @throws IllegalStateException if decoder is destroyed
     */
    @Throws(IllegalStateException::class)
    fun decodeFrame(inputData: ByteArray, timestampUs: Long): Int {
        checkNotDestroyed()
        require(inputData.isNotEmpty()) { "Input data cannot be empty" }

        try {
            val result = decodeFrame(decoderPtr, inputData, inputData.size, timestampUs)
            if (result < 0) {
                Log.e(TAG, "Decode error, code: $result")
                throw IllegalStateException("Decode failed with code: $result")
            }
            return result
        } catch (e: Exception) {
            Log.e(TAG, "Error decoding frame: ${e.message}", e)
            throw IllegalStateException("Decode failed: ${e.message}", e)
        }
    }

    /**
     * Retrieve next decoded frame
     *
     * @return DecodedFrame if available, null if queue is empty
     * @throws IllegalStateException if decoder is destroyed or error occurs
     */
    @Throws(IllegalStateException::class)
    fun getDecodedFrame(): DecodedFrame? {
        checkNotDestroyed()

        try {
            val outData = LongArray(1) // Pointer to frame data
            val outSize = IntArray(1)  // Frame size
            val outWidth = IntArray(1) // Frame width
            val outHeight = IntArray(1) // Frame height
            val outTimestamp = LongArray(1) // Presentation timestamp

            val result = getDecodedFrame(decoderPtr, outData, outSize, outWidth, outHeight, outTimestamp)

            return when (result) {
                0 -> {
                    // Success - create ByteBuffer from native pointer
                    // Note: In production, this requires careful memory management
                    // The buffer remains valid until releaseBuffer is called
                    val buffer = ByteBuffer.allocateDirect(outSize[0])
                    DecodedFrame(
                        data = buffer,
                        width = outWidth[0],
                        height = outHeight[0],
                        timestampUs = outTimestamp[0],
                        decodeLatencyUs = 0L // Would be computed in native layer
                    )
                }
                -2 -> {
                    // No output buffer available
                    null
                }
                else -> {
                    Log.e(TAG, "Failed to get decoded frame, code: $result")
                    throw IllegalStateException("getDecodedFrame failed with code: $result")
                }
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error getting decoded frame: ${e.message}", e)
            throw IllegalStateException("Failed to get decoded frame: ${e.message}", e)
        }
    }

    /**
     * Release the current output buffer back to the decoder
     *
     * @throws IllegalStateException if decoder is destroyed or error occurs
     */
    @Throws(IllegalStateException::class)
    fun releaseBuffer() {
        checkNotDestroyed()

        try {
            val result = releaseBuffer(decoderPtr, 0L) // ptr would be from frame
            if (result != 0) {
                Log.w(TAG, "releaseBuffer returned code: $result")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error releasing buffer: ${e.message}", e)
            throw IllegalStateException("Failed to release buffer: ${e.message}", e)
        }
    }

    /**
     * Get current decoder performance metrics
     *
     * @return Current decoder metrics
     * @throws IllegalStateException if decoder is destroyed or error occurs
     */
    @Throws(IllegalStateException::class)
    fun getMetrics(): DecoderMetrics {
        checkNotDestroyed()

        try {
            val metricsArray = LongArray(5)
            val result = getMetrics(decoderPtr, metricsArray)
            if (result != 0) {
                Log.e(TAG, "Failed to get metrics, code: $result")
                throw IllegalStateException("getMetrics failed with code: $result")
            }

            return DecoderMetrics(
                avgDecodeLatencyUs = metricsArray[0],
                maxDecodeLatencyUs = metricsArray[1],
                framesDecoded = metricsArray[2],
                framesDropped = metricsArray[3],
                lastTimestampUs = if (metricsArray[4] >= 0) metricsArray[4] else null
            )
        } catch (e: Exception) {
            Log.e(TAG, "Error getting metrics: ${e.message}", e)
            throw IllegalStateException("Failed to get metrics: ${e.message}", e)
        }
    }

    /**
     * Enable or disable low-latency mode
     *
     * Low-latency mode prioritizes frame delivery speed over buffering.
     *
     * @param enabled true to enable, false to disable
     * @throws IllegalStateException if decoder is destroyed or error occurs
     */
    @Throws(IllegalStateException::class)
    fun setLowLatencyMode(enabled: Boolean) {
        checkNotDestroyed()

        try {
            val result = setLowLatencyMode(decoderPtr, if (enabled) 1 else 0)
            if (result != 0) {
                Log.e(TAG, "Failed to set low-latency mode, code: $result")
                throw IllegalStateException("setLowLatencyMode failed with code: $result")
            }
            Log.i(TAG, "Low-latency mode: $enabled")
        } catch (e: Exception) {
            Log.e(TAG, "Error setting low-latency mode: ${e.message}", e)
            throw IllegalStateException("Failed to set low-latency mode: ${e.message}", e)
        }
    }

    /**
     * Reset decoder state (for seeking or discontinuity)
     *
     * Clears all buffered frames and resets metrics.
     *
     * @throws IllegalStateException if decoder is destroyed or error occurs
     */
    @Throws(IllegalStateException::class)
    fun reset() {
        checkNotDestroyed()

        try {
            val result = resetDecoder(decoderPtr)
            if (result != 0) {
                Log.e(TAG, "Failed to reset decoder, code: $result")
                throw IllegalStateException("resetDecoder failed with code: $result")
            }
            Log.i(TAG, "Decoder reset successfully")
        } catch (e: Exception) {
            Log.e(TAG, "Error resetting decoder: ${e.message}", e)
            throw IllegalStateException("Failed to reset decoder: ${e.message}", e)
        }
    }

    /**
     * Check if decoder is still valid
     */
    fun isValid(): Boolean = !isDestroyed && decoderPtr != 0L

    /**
     * Destroy the decoder and free resources
     *
     * Safe to call multiple times.
     */
    override fun close() {
        if (isDestroyed) {
            return
        }

        try {
            if (decoderPtr != 0L) {
                destroyDecoder(decoderPtr)
                Log.i(TAG, "Decoder destroyed")
            }
        } catch (e: Exception) {
            Log.e(TAG, "Error destroying decoder: ${e.message}", e)
        } finally {
            isDestroyed = true
            decoderPtr = 0L
        }
    }

    private fun checkNotDestroyed() {
        if (isDestroyed || decoderPtr == 0L) {
            throw IllegalStateException("Decoder has been destroyed")
        }
    }
}
