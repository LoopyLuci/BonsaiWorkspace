package ai.bonsai.buddy.data.network

import ai.bonsai.buddy.data.logging.BonsaiLogger
import ai.bonsai.buddy.data.storage.SecureConfigStore
import io.ktor.client.HttpClient
import io.ktor.client.call.body
import io.ktor.client.plugins.DefaultRequest
import io.ktor.client.plugins.ResponseException
import io.ktor.client.plugins.contentnegotiation.ContentNegotiation
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.accept
import io.ktor.client.request.bearerAuth
import io.ktor.client.request.get
import io.ktor.client.request.post
import io.ktor.client.request.prepareGet
import io.ktor.client.request.preparePost
import io.ktor.client.request.setBody
import io.ktor.client.statement.bodyAsChannel
import io.ktor.http.ContentType
import io.ktor.http.contentType
import io.ktor.http.isSuccess
import io.ktor.serialization.kotlinx.json.json
import io.ktor.utils.io.readUTF8Line
import kotlinx.serialization.json.contentOrNull
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

@Singleton
class BonsaiApiClient @Inject constructor(
    private val configStore: SecureConfigStore,
    private val logger: BonsaiLogger
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
        logger.i(TAG, "health check ${config.buddyBaseUrl()}/health")
        client.get("${config.buddyBaseUrl()}/health")
        Unit
    }.onFailure { logger.e(TAG, "health check failed", it) }

    suspend fun sendChatMessage(prompt: String): Result<String> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "sendChatMessage to ${config.buddyBaseUrl()}")
        val response = client.post("${config.buddyBaseUrl()}/api/buddy/chat") {
            setBody(ChatRequest(message = prompt, stream = false))
        }

        val asTyped = runCatching { response.body<ChatResponse>() }.getOrNull()
        if (asTyped != null) {
            return@runCatching asTyped.response ?: asTyped.content ?: asTyped.message ?: ""
        }

        val asJson = response.body<JsonElement>()
        val obj = asJson.jsonObject
        obj["response"]?.jsonPrimitive?.content
            ?: obj["content"]?.jsonPrimitive?.content
            ?: obj["message"]?.jsonPrimitive?.content
            ?: ""
    }.onFailure { logger.e(TAG, "sendChatMessage failed", it) }

    fun chatStream(prompt: String): Flow<String> = flow {
        val config = requireConfigured()
        logger.i(TAG, "chatStream start")

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
        val config = requireConfigured()
        logger.i(TAG, "fetchTools from workspace API")
        val root = client.get("${config.workspaceBaseUrl()}/api/tools").body<JsonElement>()
        val tools = root.jsonObject["tools"]?.jsonArray.orEmpty()
        tools.mapNotNull { entry ->
            val obj = entry.jsonObject
            val name = obj["name"]?.jsonPrimitive?.content ?: return@mapNotNull null
            ToolDescriptor(
                name = name,
                description = obj["description"]?.jsonPrimitive?.contentOrNull
            )
        }
    }.onFailure { logger.e(TAG, "fetchTools failed", it) }

    suspend fun invokeTool(tool: String, params: Map<String, String>): Result<String> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "invokeTool $tool")
        val response = client.post("${config.buddyBaseUrl()}/api/buddy/tool") {
            setBody(ToolInvocationRequest(tool = tool, params = params))
        }.body<ToolInvocationResponse>()

        response.result ?: response.output ?: response.error ?: "No result"
    }.recoverCatching {
        val config = requireConfigured()
        val fallback = client.post("${config.buddyBaseUrl()}/api/tools/invoke") {
            setBody(ToolInvocationRequest(tool = tool, params = params))
        }.body<ToolInvocationResponse>()
        fallback.result ?: fallback.output ?: fallback.error ?: "No result"
    }.onFailure { logger.e(TAG, "invokeTool failed", it) }

    suspend fun fetchModels(): Result<List<ModelDescriptor>> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "fetchModels from /v1/models")
        val root = client.get("${config.buddyBaseUrl()}/v1/models").body<JsonElement>()
        val data = root.jsonObject["data"]?.jsonArray.orEmpty()
        data.mapNotNull { entry ->
            val obj = entry.jsonObject
            val id = obj["id"]?.jsonPrimitive?.content ?: return@mapNotNull null
            ModelDescriptor(
                id = id,
                name = obj["name"]?.jsonPrimitive?.contentOrNull ?: id,
                tier = obj["tier"]?.jsonPrimitive?.contentOrNull,
                quant = obj["quant"]?.jsonPrimitive?.contentOrNull,
                ram = obj["ram"]?.jsonPrimitive?.contentOrNull,
                progress = obj["progress"]?.jsonPrimitive?.contentOrNull?.toIntOrNull(),
                loaded = obj["loaded"]?.jsonPrimitive?.contentOrNull?.toBooleanStrictOrNull() ?: false
            )
        }
    }.onFailure { logger.e(TAG, "fetchModels failed", it) }

    suspend fun loadModel(modelId: String): Result<Unit> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "loadModel $modelId")
        client.post("${config.buddyBaseUrl()}/api/models/load") {
            setBody(ModelActionRequest(model = modelId))
        }
        Unit
    }.onFailure { logger.e(TAG, "loadModel failed", it) }

    suspend fun unloadModel(modelId: String): Result<Unit> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "unloadModel $modelId")
        client.post("${config.buddyBaseUrl()}/api/models/unload") {
            setBody(ModelActionRequest(model = modelId))
        }
        Unit
    }.onFailure { logger.e(TAG, "unloadModel failed", it) }

    suspend fun setInferenceMode(mode: InferenceMode): Result<Unit> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "setInferenceMode $mode")
        client.post("${config.buddyBaseUrl()}/api/inference/mode") {
            setBody(InferenceModeRequest(mode))
        }
        Unit
    }.onFailure { logger.e(TAG, "setInferenceMode failed", it) }

    suspend fun fetchActivityEvents(): Result<List<ActivityEventDto>> = runCatching {
        val config = requireConfigured()
        logger.i(TAG, "fetchActivityEvents")
        val root = client.get("${config.workspaceBaseUrl()}/api/activity").body<JsonElement>()
        val data = root.jsonObject["events"]?.jsonArray.orEmpty()
        data.mapNotNull { entry ->
            val obj = entry.jsonObject
            val id = obj["id"]?.jsonPrimitive?.contentOrNull ?: return@mapNotNull null
            ActivityEventDto(
                id = id,
                type = obj["type"]?.jsonPrimitive?.contentOrNull ?: "unknown",
                message = obj["message"]?.jsonPrimitive?.contentOrNull ?: "",
                level = obj["level"]?.jsonPrimitive?.contentOrNull,
                timestamp = obj["timestamp"]?.jsonPrimitive?.contentOrNull?.toLongOrNull()
                    ?: System.currentTimeMillis()
            )
        }
    }.recoverCatching {
        emptyList()
    }.onFailure { logger.e(TAG, "fetchActivityEvents failed", it) }

    fun eventStream(): Flow<String> = flow {
        val config = requireConfigured()
        logger.i(TAG, "eventStream connected")
        client.prepareGet("${config.workspaceBaseUrl()}/api/events") {
            accept(ContentType.Text.EventStream)
        }.execute { response ->
            if (!response.status.isSuccess()) {
                throw ResponseException(response, "Event stream failed: ${response.status}")
            }

            val channel = response.bodyAsChannel()
            while (!channel.isClosedForRead) {
                val line = channel.readUTF8Line() ?: break
                if (!line.startsWith("data:")) continue
                emit(line.removePrefix("data:").trim())
            }
        }
    }

    private fun requireConfigured(): ConnectionConfig = requireNotNull(configStore.getConnectionConfig()) {
        "Connection is not configured"
    }

    private fun parseSseToken(raw: String): String? {
        val parsed = runCatching { json.parseToJsonElement(raw) }.getOrNull() ?: return raw
        val obj = parsed.jsonObject

        return obj["token"]?.jsonPrimitive?.contentOrNull
            ?: obj["delta"]?.jsonPrimitive?.contentOrNull
            ?: obj["content"]?.jsonPrimitive?.contentOrNull
            ?: obj["response"]?.jsonPrimitive?.contentOrNull
    }

    private companion object {
        private const val TAG = "BonsaiApiClient"
    }
}
