package ai.bonsai.buddy.data.remote_desktop

import android.view.Surface
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

data class StreamStats(
    val fps: Int = 60,
    val latencyMs: Long = 50,
    val bitrateMbps: Double = 5.0
)

class BrdfMobileClient(
    private val peerId: String,
    private val tokenBase64: String,
    private val surface: Surface? = null
) {
    private val _streamStats = MutableStateFlow(StreamStats())
    val streamStats: StateFlow<StreamStats> = _streamStats

    suspend fun connectToPeer() {
        // Implementation
    }

    suspend fun startStreamingScreen() {
        // Implementation
    }

    suspend fun injectInput(input: String) {
        // Implementation
    }

    suspend fun stopStreaming() {
        // Implementation
    }

    suspend fun getStats(): StreamStats {
        return StreamStats()
    }

    fun release() {
        // Implementation
    }
}
