package ai.bonsai.buddy.data.network

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class ConnectionConfig(
    val host: String,
    val buddyPort: Int = 11420,
    val workspacePort: Int = 11369,
    val useHttps: Boolean = false
) {
    fun buddyBaseUrl(): String = "${scheme()}://$host:$buddyPort"
    fun workspaceBaseUrl(): String = "${scheme()}://$host:$workspacePort"

    private fun scheme(): String = if (useHttps) "https" else "http"
}

@Serializable
data class ChatRequest(
    val message: String,
    @SerialName("stream") val stream: Boolean = false
)

@Serializable
data class ChatResponse(
    val response: String? = null,
    val content: String? = null,
    val message: String? = null
)

@Serializable
data class ToolDescriptor(
    val name: String,
    val description: String? = null
)
