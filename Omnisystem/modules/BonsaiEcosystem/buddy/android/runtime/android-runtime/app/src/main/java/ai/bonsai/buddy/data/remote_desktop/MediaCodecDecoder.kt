package ai.bonsai.buddy.data.remote_desktop

import android.media.MediaCodec
import android.media.MediaFormat
import android.view.Surface

class MediaCodecDecoder(
    private val codecName: String = "avc",
    private val width: Int = 1080,
    private val height: Int = 720
) {
    private var codec: MediaCodec? = null
    private var surface: Surface? = null

    fun initialize(outputSurface: Surface) {
        surface = outputSurface
        val format = MediaFormat.createVideoFormat("video/avc", width, height)
        codec = MediaCodec.createDecoderByType("video/avc")
        codec?.configure(format, outputSurface, null, 0)
        codec?.start()
    }

    fun decodeFrame(data: ByteArray) {
        codec?.let { codec ->
            val inputIndex = codec.dequeueInputBuffer(10000)
            if (inputIndex >= 0) {
                val inputBuffer = codec.getInputBuffer(inputIndex)
                inputBuffer?.put(data)
                codec.queueInputBuffer(inputIndex, 0, data.size, System.currentTimeMillis(), 0)
            }
        }
    }

    fun release() {
        codec?.stop()
        codec?.release()
        codec = null
    }
}
