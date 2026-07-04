package ai.bonsai.buddy.network

import android.util.Log
import okhttp3.*
import org.json.JSONObject
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.receiveAsFlow
import java.util.concurrent.TimeUnit

/**
 * Manages a persistent WebSocket connection to the Bonsai backend.
 * Emits real-time events as a Kotlin Flow for Jetpack Compose.
 */
class WebSocketManager(
    private val serverUrl: String = "ws://192.168.1.100:4200/ws",
    private val autoReconnect: Boolean = true,
    private val reconnectDelayMs: Long = 5000
) {

    private val client = OkHttpClient.Builder()
        .pingInterval(30, TimeUnit.SECONDS)
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    private var webSocket: WebSocket? = null
    private val _events = Channel<WebSocketEvent>(Channel.BUFFERED)
    val events: Flow<WebSocketEvent> = _events.receiveAsFlow()

    private var isConnecting = false

    sealed class WebSocketEvent {
        data class Connected(val timestamp: Long = System.currentTimeMillis()) : WebSocketEvent()
        data class Disconnected(val reason: String = "") : WebSocketEvent()
        data class Message(val type: String, val payload: JSONObject) : WebSocketEvent()
        data class Error(val message: String) : WebSocketEvent()
    }

    fun connect() {
        if (isConnecting || webSocket != null) return
        isConnecting = true

        Log.d("WebSocket", "Connecting to $serverUrl")

        val request = Request.Builder()
            .url(serverUrl)
            .build()

        webSocket = client.newWebSocket(request, object : WebSocketListener() {
            override fun onOpen(webSocket: WebSocket, response: Response) {
                Log.d("WebSocket", "Connected to $serverUrl")
                isConnecting = false
                _events.trySend(WebSocketEvent.Connected())
            }

            override fun onMessage(webSocket: WebSocket, text: String) {
                try {
                    val json = JSONObject(text)
                    val type = json.optString("type", "unknown")
                    val payload = json.optJSONObject("payload") ?: JSONObject()
                    _events.trySend(WebSocketEvent.Message(type, payload))
                } catch (e: Exception) {
                    Log.e("WebSocket", "Error parsing message: ${e.message}")
                    _events.trySend(WebSocketEvent.Error("Parse error: ${e.message}"))
                }
            }

            override fun onClosing(webSocket: WebSocket, code: Int, reason: String) {
                Log.d("WebSocket", "WebSocket closing: $reason")
                webSocket.close(1000, null)
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                Log.d("WebSocket", "WebSocket closed: $reason (code: $code)")
                isConnecting = false
                _events.trySend(WebSocketEvent.Disconnected(reason))

                if (autoReconnect && code != 1000) {
                    Log.d("WebSocket", "Attempting to reconnect in ${reconnectDelayMs}ms...")
                    Thread.sleep(reconnectDelayMs)
                    this@WebSocketManager.webSocket = null
                    connect()
                }
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                Log.e("WebSocket", "WebSocket failure: ${t.message}")
                isConnecting = false
                _events.trySend(WebSocketEvent.Error(t.message ?: "Unknown error"))

                if (autoReconnect) {
                    Log.d("WebSocket", "Attempting to reconnect in ${reconnectDelayMs}ms...")
                    Thread.sleep(reconnectDelayMs)
                    this@WebSocketManager.webSocket = null
                    connect()
                }
            }
        })
    }

    fun disconnect() {
        Log.d("WebSocket", "Disconnecting WebSocket")
        isConnecting = false
        webSocket?.close(1000, "Client closing")
        webSocket = null
    }

    fun send(message: String) {
        webSocket?.send(message)
    }

    fun isConnected(): Boolean = webSocket != null && !isConnecting
}
