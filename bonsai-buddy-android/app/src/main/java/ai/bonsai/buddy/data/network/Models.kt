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

@Serializable
data class ToolInvocationRequest(
    val tool: String,
    val params: Map<String, String> = emptyMap()
)

@Serializable
data class ToolInvocationResponse(
    val result: String? = null,
    val output: String? = null,
    val error: String? = null
)

@Serializable
data class ModelDescriptor(
    val id: String,
    val name: String = id,
    val tier: String? = null,
    val quant: String? = null,
    val ram: String? = null,
    val progress: Int? = null,
    val loaded: Boolean = false
)

@Serializable
data class ModelActionRequest(
    val model: String
)

@Serializable
data class InferenceModeRequest(
    val mode: InferenceMode
)

@Serializable
enum class InferenceMode {
    AUTO,
    CPU_ONLY,
    GPU_ONLY,
    HYBRID
}

@Serializable
data class ActivityEventDto(
    val id: String,
    val type: String,
    val message: String,
    val level: String? = null,
    val timestamp: Long
)
