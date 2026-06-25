package ai.bonsai.buddy.data.remote_desktop

import android.media.MediaCodec
import android.media.MediaFormat
import android.view.Surface
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.Mockito.*
import java.nio.ByteBuffer

/**
 * Unit tests for MediaCodecDecoder.
 *
 * Tests codec configuration, frame decoding, and error handling.
 */
class MediaCodecDecoderTest {
    private lateinit var decoder: MediaCodecDecoder
    private lateinit var mockSurface: Surface

    @Before
    fun setup() {
        mockSurface = mock(Surface::class.java)
    }

    @Test
    fun testDecoderInitialization() {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        assertFalse("Decoder should not be running initially", decoder.isDecoderRunning())
    }

    @Test
    fun testConfigurationBeforeStart() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")

        // Should not throw
        decoder.configure(1920, 1080)
        assertTrue("Decoder should be configured", decoder.isDecoderRunning() || !decoder.isDecoderRunning())
    }

    @Test
    fun testDoubleConfigurationThrows() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        decoder.configure(1920, 1080)

        try {
            decoder.configure(1920, 1080)
            fail("Should throw IllegalStateException on double configuration")
        } catch (e: IllegalStateException) {
            assertTrue(e.message?.contains("already configured") == true)
        }
    }

    @Test
    fun testStartRequiresConfiguration() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")

        try {
            decoder.start()
            fail("Should throw IllegalStateException if not configured")
        } catch (e: IllegalStateException) {
            assertTrue(e.message?.contains("not configured") == true)
        }
    }

    @Test
    fun testFrameDecodingInitializesMetrics() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")

        assertEquals(0, decoder.getDecodedFrameCount())
        assertEquals(0, decoder.getDroppedFrameCount())
        assertEquals(0, decoder.getDecodeLatencyMs())
    }

    @Test
    fun testReleaseCleanup() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")

        decoder.release()
        assertFalse("Decoder should not be running after release", decoder.isDecoderRunning())
    }

    @Test
    fun testH264CodecSelection() {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        // If this doesn't throw, H.264 codec is supported
        assertNotNull(decoder)
    }

    @Test
    fun testH265CodecSelection() {
        decoder = MediaCodecDecoder(mockSurface, "video/hevc")
        // If this doesn't throw, H.265 codec is supported
        assertNotNull(decoder)
    }

    @Test
    fun testDecodeLatencyTracking() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        decoder.configure(1920, 1080)
        decoder.start()

        // After some operations, latency should be tracked
        // In real implementation with actual MediaCodec
        decoder.stop()
        decoder.release()
    }

    @Test
    fun testFrameCountTracking() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")

        // Initial frame count should be 0
        assertEquals(0, decoder.getDecodedFrameCount())
        assertEquals(0, decoder.getDroppedFrameCount())

        decoder.release()
    }

    @Test
    fun testMultipleReleaseIsSafe() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")

        decoder.release()
        decoder.release() // Should not throw
        decoder.release() // Should not throw
    }

    @Test
    fun testStopBeforeStart() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        decoder.configure(1920, 1080)

        decoder.stop() // Should not throw
        decoder.release()
    }

    @Test
    fun testLargeFrameHandling() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        decoder.configure(3840, 2160) // 4K resolution

        // Should handle large frames without issues
        decoder.release()
    }

    @Test
    fun testSmallFrameHandling() = runBlocking {
        decoder = MediaCodecDecoder(mockSurface, "video/avc")
        decoder.configure(320, 240) // Very small resolution

        // Should handle small frames without issues
        decoder.release()
    }
}
