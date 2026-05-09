package ai.bonsai.buddy.data.network

import ai.bonsai.buddy.data.storage.SecureConfigStore
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.DefaultRequest
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.get
import io.ktor.client.request.post
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.serialization.kotlinx.json.json
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class BonsaiApiClient @Inject constructor(
    private val configStore: SecureConfigStore
) {
    private val json = Json {
        ignoreUnknownKeys = true
        isLenient = true
    }

    private val client = HttpClient {
        install(ContentNegotiation) {
            json(json)
        }
        install(Logging) {
            level = LogLevel.INFO
        }
        install(DefaultRequest) {
            contentType(ContentType.Application.Json)
            configStore.getToken()?.takeIf { it.isNotBlank() }?.let { bearerAuth(it) }
        }
    }

    suspend fun checkBuddyHealth(config: ConnectionConfig): Result<Unit> = runCatching {
        client.get("${config.buddyBaseUrl()}/health")
        Unit
    }

    suspend fun sendChatMessage(prompt: String): Result<String> = runCatching {
        val config = requireNotNull(configStore.getConnectionConfig()) {
            "Connection is not configured"
        }
        val response = client.post("${config.buddyBaseUrl()}/api/buddy/chat") {
            setBody(ChatRequest(message = prompt, stream = false))
        }

        val asTyped = runCatching { response.body<ChatResponse>() }.getOrNull()
        if (asTyped != null) {
            return@runCatching asTyped.response ?: asTyped.content ?: asTyped.message ?: ""
        }

        val asJson = response.body<kotlinx.serialization.json.JsonElement>()
        val obj = asJson.jsonObject
        obj["response"]?.jsonPrimitive?.content
            ?: obj["content"]?.jsonPrimitive?.content
            ?: obj["message"]?.jsonPrimitive?.content
            ?: ""
    }

    suspend fun fetchTools(): Result<List<ToolDescriptor>> = runCatching {
        val config = requireNotNull(configStore.getConnectionConfig()) {
            "Connection is not configured"
        }
        val root = client.get("${config.workspaceBaseUrl()}/api/tools").body<kotlinx.serialization.json.JsonElement>()
        val tools = root.jsonObject["tools"]?.jsonArray.orEmpty()
        tools.mapNotNull { entry ->
            val obj = entry.jsonObject
            val name = obj["name"]?.jsonPrimitive?.content ?: return@mapNotNull null
            ToolDescriptor(name = name, description = obj["description"]?.jsonPrimitive?.contentOrNull)
        }
    }
}
