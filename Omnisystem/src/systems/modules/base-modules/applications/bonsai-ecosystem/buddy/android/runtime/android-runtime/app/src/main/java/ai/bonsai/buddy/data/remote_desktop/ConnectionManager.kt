package ai.bonsai.buddy.data.remote_desktop

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

data class RemotePeerInfo(
    val peerId: String,
    val osType: String = "Android",
    val host: String,
    val port: Int
)

class ConnectionManager {
    private val _connections = MutableStateFlow<List<RemotePeerInfo>>(emptyList())
    val connections: StateFlow<List<RemotePeerInfo>> = _connections

    suspend fun discoverPeers(): List<RemotePeerInfo> {
        return emptyList()
    }

    suspend fun connectToPeer(peer: RemotePeerInfo): Boolean {
        return true
    }

    suspend fun disconnectPeer(peerId: String) {
        // Implementation
    }
}
