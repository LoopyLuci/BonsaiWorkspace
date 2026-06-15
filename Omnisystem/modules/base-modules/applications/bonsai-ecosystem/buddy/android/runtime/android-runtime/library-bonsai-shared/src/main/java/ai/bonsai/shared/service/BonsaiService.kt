package ai.bonsai.shared.service

import android.app.Service
import android.content.Intent
import android.content.Context
import android.os.IBinder
import android.os.Environment
import android.util.Log
import androidx.room.Room
import ai.bonsai.shared.IBonsaiCallback
import ai.bonsai.shared.IBonsaiService
import ai.bonsai.shared.db.BonsaiDatabase
import java.io.File
import java.util.*
import kotlinx.coroutines.*
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/**
 * BonsaiService
 *
 * Production-grade Android Service providing:
 * - LLM inference with streaming support
 * - Model management (load/unload)
 * - Session tracking and persistence
 * - Knowledge base integration
 * - Peer transfer and communication
 */
class BonsaiService : Service() {
    private val binder = BonsaiServiceImpl(this)

    companion object {
        private const val TAG = "BonsaiService"

        init {
            try {
                System.loadLibrary("bonsai_android_llm")
                Log.i(TAG, "Native library loaded: bonsai_android_llm")
            } catch (e: UnsatisfiedLinkError) {
                Log.e(TAG, "Failed to load native library", e)
                throw RuntimeException("Failed to load bonsai_android_llm", e)
            }
        }
    }

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "BonsaiService created")
        binder.initialize(this)
    }

    override fun onBind(intent: Intent): IBinder {
        Log.i(TAG, "BonsaiService bound")
        return binder.asBinder()
    }

    override fun onDestroy() {
        super.onDestroy()
        Log.i(TAG, "BonsaiService destroyed")
        binder.shutdown()
    }
}

/**
 * BonsaiServiceImpl
 *
 * Core service implementation with full JNI binding support.
 * Thread-safe with proper error handling and resource lifecycle management.
 */
class BonsaiServiceImpl(private val service: BonsaiService) : IBonsaiService.Stub() {
    companion object {
        private const val TAG = "BonsaiServiceImpl"
        private const val MODELS_DIR = "/sdcard/Bonsai/models"
        private const val CACHE_DIR = "/data/local/tmp/bonsai"
    }

    private var currentModelId: String? = null
    private var modelHandle: Long = 0
    private var transferDaemonHandle: Long = 0
    private lateinit var database: BonsaiDatabase
    private val scope = CoroutineScope(Dispatchers.Default + SupervisorJob())

    /**
     * Initialize service resources (called from onCreate)
     */
    fun initialize(context: Context) {
        try {
            database = Room.databaseBuilder(
                context,
                BonsaiDatabase::class.java,
                "bonsai_database"
            ).build()

            // Ensure models directory exists
            val modelsDir = File(MODELS_DIR)
            modelsDir.mkdirs()

            Log.i(TAG, "BonsaiServiceImpl initialized")
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize BonsaiServiceImpl", e)
        }
    }

    // ========================================================================
    // Phase 2-3: New LLM JNI Interface
    // ========================================================================

    /**
     * Initialize an LLM model from file path
     * @param modelPath Full path to GGUF or SafeTensors model file
     * @return JSON response with model_id or error
     */
    override fun nativeInitModel(modelPath: String): String {
        return try {
            Log.i(TAG, "nativeInitModel: $modelPath")
            val response = nativeInitModel_jni(modelPath)
            currentModelId = extractModelId(response)
            response
        } catch (e: Exception) {
            Log.e(TAG, "nativeInitModel failed", e)
            errorJson("Model initialization failed: ${e.message}")
        }
    }

    /**
     * Single-turn chat inference
     * @param modelId Model ID from nativeInitModel
     * @param messagesJson JSON array of chat messages
     * @param temperature Sampling temperature (0.0-2.0)
     * @param maxTokens Maximum tokens to generate
     * @return JSON response with generated text
     */
    override fun nativeChat(
        modelId: String,
        messagesJson: String,
        temperature: Float,
        maxTokens: Int
    ): String {
        return try {
            Log.i(TAG, "nativeChat: model=$modelId, tokens=$maxTokens, temp=$temperature")
            nativeChat_jni(modelId, messagesJson, temperature, maxTokens)
        } catch (e: Exception) {
            Log.e(TAG, "nativeChat failed", e)
            errorJson("Chat failed: ${e.message}")
        }
    }

    /**
     * Streaming chat inference
     * Generates tokens and invokes callback for each token
     * @param modelId Model ID
     * @param messagesJson Chat messages
     * @param temperature Sampling temperature
     * @param callback Receives onToken/onComplete/onError
     */
    override fun nativeChatStream(
        modelId: String,
        messagesJson: String,
        temperature: Float,
        callback: IBonsaiCallback
    ) {
        try {
            Log.i(TAG, "nativeChatStream: model=$modelId, temp=$temperature")
            nativeChatStream_jni(modelId, messagesJson, temperature, object : StreamCallback {
                override fun onToken(token: String) {
                    try {
                        callback.onToken(token)
                    } catch (e: Exception) {
                        Log.e(TAG, "Callback error in onToken", e)
                    }
                }

                override fun onComplete() {
                    try {
                        callback.onComplete()
                    } catch (e: Exception) {
                        Log.e(TAG, "Callback error in onComplete", e)
                    }
                }

                override fun onError(error: String) {
                    try {
                        callback.onError(error)
                    } catch (e: Exception) {
                        Log.e(TAG, "Callback error in onError", e)
                    }
                }
            })
        } catch (e: Exception) {
            Log.e(TAG, "nativeChatStream failed", e)
            try {
                callback.onError("Stream failed: ${e.message}")
            } catch (e2: Exception) {
                Log.e(TAG, "Failed to call error callback", e2)
            }
        }
    }

    /**
     * Unload model and free resources
     * @param modelId Model ID
     * @return true if successful
     */
    override fun nativeUnloadModel(modelId: String): Boolean {
        return try {
            Log.i(TAG, "nativeUnloadModel: $modelId")
            val result = nativeUnloadModel_jni(modelId)
            if (result) {
                currentModelId = null
            }
            result
        } catch (e: Exception) {
            Log.e(TAG, "nativeUnloadModel failed", e)
            false
        }
    }

    /**
     * List available models in standard directory
     * @return List of model filenames (*.gguf, *.safetensors)
     */
    override fun nativeGetAvailableModels(): MutableList<String> {
        return try {
            Log.i(TAG, "nativeGetAvailableModels")
            nativeGetAvailableModels_jni().toMutableList()
        } catch (e: Exception) {
            Log.e(TAG, "nativeGetAvailableModels failed", e)
            mutableListOf()
        }
    }

    /**
     * Get session information
     * @param sessionId Session identifier
     * @return JSON with session details
     */
    override fun nativeGetSessionInfo(sessionId: String): String {
        return try {
            Log.i(TAG, "nativeGetSessionInfo: $sessionId")
            nativeGetSessionInfo_jni(sessionId)
        } catch (e: Exception) {
            Log.e(TAG, "nativeGetSessionInfo failed", e)
            errorJson("Session not found")
        }
    }

    // ========================================================================
    // Legacy Interface (Deprecated)
    // ========================================================================

    override fun initModel(modelPath: String, tokenizerPath: String): Long {
        return try {
            Log.i(TAG, "initModel (legacy): $modelPath")
            modelHandle = nativeInitModel_legacy(modelPath, tokenizerPath)
            modelHandle
        } catch (e: Exception) {
            Log.e(TAG, "initModel (legacy) failed", e)
            0L
        }
    }

    override fun chat(handle: Long, prompt: String, temperature: Float): String {
        return try {
            nativeChat_legacy(handle, prompt, temperature)
        } catch (e: Exception) {
            Log.e(TAG, "chat (legacy) failed", e)
            "Error: ${e.message}"
        }
    }

    override fun generateStream(handle: Long, prompt: String, callback: IBonsaiCallback) {
        try {
            nativeGenerateStream_legacy(handle, prompt, object : StreamCallback {
                override fun onToken(token: String) {
                    try { callback.onToken(token) } catch (e: Exception) { }
                }
                override fun onComplete() {
                    try { callback.onComplete() } catch (e: Exception) { }
                }
                override fun onError(error: String) {
                    try { callback.onError(error) } catch (e: Exception) { }
                }
            })
        } catch (e: Exception) {
            Log.e(TAG, "generateStream (legacy) failed", e)
        }
    }

    override fun loadToken(token: ByteArray): Boolean {
        return try {
            nativeLoadToken(token)
        } catch (e: Exception) {
            Log.e(TAG, "loadToken failed", e)
            false
        }
    }

    override fun verifyToken(peerId: String): Boolean {
        return try {
            nativeVerifyToken(peerId)
        } catch (e: Exception) {
            Log.e(TAG, "verifyToken failed", e)
            false
        }
    }

    override fun startTransferDaemon(configPath: String): Boolean {
        return try {
            transferDaemonHandle = nativeStartTransferDaemon(configPath)
            transferDaemonHandle != 0L
        } catch (e: Exception) {
            Log.e(TAG, "startTransferDaemon failed", e)
            false
        }
    }

    override fun connectToPeer(peerId: String, token: ByteArray): Long {
        return try {
            nativeConnectToPeer(peerId, token)
        } catch (e: Exception) {
            Log.e(TAG, "connectToPeer failed", e)
            0L
        }
    }

    override fun injectInput(sessionHandle: Long, eventType: Int, data: ByteArray) {
        try {
            nativeInjectInput(sessionHandle, eventType, data)
        } catch (e: Exception) {
            Log.e(TAG, "injectInput failed", e)
        }
    }

    override fun releaseHandle(handle: Long) {
        try {
            nativeReleaseHandle(handle)
        } catch (e: Exception) {
            Log.e(TAG, "releaseHandle failed", e)
        }
    }

    override fun shutdown() {
        try {
            scope.cancel()
            if (modelHandle != 0L) nativeReleaseHandle(modelHandle)
            if (transferDaemonHandle != 0L) nativeReleaseHandle(transferDaemonHandle)
            if (currentModelId != null) nativeUnloadModel(currentModelId!!)
            Log.i(TAG, "BonsaiServiceImpl shutdown complete")
        } catch (e: Exception) {
            Log.e(TAG, "shutdown failed", e)
        }
    }

    // ========================================================================
    // Native Method Declarations
    // ========================================================================

    // Phase 2-3 LLM JNI methods
    private external fun nativeInitModel_jni(modelPath: String): String
    private external fun nativeChat_jni(modelId: String, messagesJson: String, temperature: Float, maxTokens: Int): String
    private external fun nativeChatStream_jni(modelId: String, messagesJson: String, temperature: Float, callback: StreamCallback)
    private external fun nativeUnloadModel_jni(modelId: String): Boolean
    private external fun nativeGetAvailableModels_jni(): Array<String>
    private external fun nativeGetSessionInfo_jni(sessionId: String): String

    // Legacy methods
    private external fun nativeInitModel_legacy(modelPath: String, tokenizerPath: String): Long
    private external fun nativeChat_legacy(handle: Long, prompt: String, temperature: Float): String
    private external fun nativeGenerateStream_legacy(handle: Long, prompt: String, callback: StreamCallback)
    private external fun nativeLoadToken(token: ByteArray): Boolean
    private external fun nativeVerifyToken(peerId: String): Boolean
    private external fun nativeStartTransferDaemon(configPath: String): Long
    private external fun nativeConnectToPeer(peerId: String, token: ByteArray): Long
    private external fun nativeInjectInput(sessionHandle: Long, eventType: Int, data: ByteArray)
    private external fun nativeReleaseHandle(handle: Long)

    interface StreamCallback {
        fun onToken(token: String)
        fun onComplete()
        fun onError(error: String)
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    private fun extractModelId(jsonResponse: String): String? {
        return try {
            val json = Json.parseToJsonElement(jsonResponse)
            json.jsonObject["model_id"]?.jsonPrimitive?.content
        } catch (e: Exception) {
            Log.w(TAG, "Failed to extract model_id from response", e)
            null
        }
    }

    private fun errorJson(message: String): String {
        return """{"status":"error","message":"$message"}"""
    }
}
