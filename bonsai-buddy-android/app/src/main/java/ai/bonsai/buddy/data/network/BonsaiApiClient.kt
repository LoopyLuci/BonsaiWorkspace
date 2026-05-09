package ai.bonsai.buddy.data.network

import ai.bonsai.buddy.data.storage.SecureConfigStore
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.ResponseException
import io.ktor.client.plugins.DefaultRequest
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.accept
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.get
import io.ktor.client.request.post
import io.ktor.client.request.preparePost
import io.ktor.client.request.setBody
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.readUTF8Line
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
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

    fun chatStream(prompt: String): Flow<String> = flow {
        val config = requireNotNull(configStore.getConnectionConfig()) {
            "Connection is not configured"
        }

        client.preparePost("${config.buddyBaseUrl()}/api/buddy/chat") {
            accept(ContentType.Text.EventStream)
            setBody(ChatRequest(message = prompt, stream = true))
        }.execute { response ->
            if (!response.status.isSuccess()) {
                throw ResponseException(response, "SSE chat request failed: ${response.status}")
            }

            val channel = response.bodyAsChannel()
            while (!channel.isClosedForRead) {
                val line = channel.readUTF8Line() ?: break
                if (!line.startsWith("data:")) continue

                val raw = line.removePrefix("data:").trim()
                if (raw.isBlank() || raw == "[DONE]") continue

                val token = parseSseToken(raw)
                if (!token.isNullOrBlank()) {
                    emit(token)
                }
            }
        }
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

    private fun parseSseToken(raw: String): String? {
        val parsed = runCatching { json.parseToJsonElement(raw) }.getOrNull() ?: return raw
        if (parsed !is JsonElement) return raw
        val obj = parsed.jsonObject

        return obj["token"]?.jsonPrimitive?.contentOrNull
            ?: obj["delta"]?.jsonPrimitive?.contentOrNull
            ?: obj["content"]?.jsonPrimitive?.contentOrNull
            ?: obj["response"]?.jsonPrimitive?.contentOrNull
    }
}
