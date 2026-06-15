package ai.bonsai.shared.model

import android.content.Context
import android.util.Log
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.io.File

/**
 * Registry for model hot-swapping and management.
 * Allows switching between models at runtime without reloading the entire application.
 */
class ModelRegistry(private val context: Context) {
    private val modelsDir = File(context.getExternalFilesDir(null), "models")

    private val _activeModel = MutableStateFlow<LoadedModel?>(null)
    val activeModel: StateFlow<LoadedModel?> = _activeModel.asStateFlow()

    private val _availableModels = MutableStateFlow<List<ModelInfo>>(emptyList())
    val availableModels: StateFlow<List<ModelInfo>> = _availableModels.asStateFlow()

    private val _loadingProgress = MutableStateFlow<Float>(0f)
    val loadingProgress: StateFlow<Float> = _loadingProgress.asStateFlow()

    init {
        modelsDir.mkdirs()
    }

    /**
     * Load a model for active use.
     *
     * @param modelPath Path to model file
     * @param config Optional configuration overrides
     * @param onProgress Progress callback (0.0-1.0)
     * @return Loaded model or null if loading failed
     */
    suspend fun loadModel(
        modelPath: String,
        config: Map<String, Any> = emptyMap(),
        onProgress: (Float) -> Unit = {}
    ): LoadedModel? {
        return try {
            val file = File(modelPath)
            if (!file.exists()) {
                Log.e(TAG, "Model file not found: $modelPath")
                return null
            }

            // Unload previous model
            _activeModel.value?.let { unloadModel(it) }

            _loadingProgress.value = 0.1f
            onProgress(0.1f)

            // Load new model
            val modelHandle = nativeLoadModel(
                modelPath,
                config,
                { progress ->
                    _loadingProgress.value = 0.1f + (progress * 0.9f)
                    onProgress(_loadingProgress.value)
                }
            )

            if (modelHandle == 0L) {
                Log.e(TAG, "Failed to load model")
                return null
            }

            val metadata = nativeGetModelMetadata(modelHandle)
            val model = LoadedModel(
                id = file.nameWithoutExtension,
                path = modelPath,
                handle = modelHandle,
                name = file.name,
                format = detectFormat(modelPath),
                size = file.length(),
                memoryUsage = metadata?.get("memory_usage") as? Long ?: 0,
                parameters = metadata?.get("parameters") as? Long ?: 0,
                architecture = metadata?.get("architecture") as? String ?: "unknown",
                loadedAt = System.currentTimeMillis()
            )

            _activeModel.value = model
            _loadingProgress.value = 1.0f
            onProgress(1.0f)

            Log.d(TAG, "Model loaded: ${model.name}")
            model
        } catch (e: Exception) {
            Log.e(TAG, "Error loading model", e)
            null
        }
    }

    /**
     * Unload the active model.
     */
    suspend fun unloadModel(model: LoadedModel? = null) {
        try {
            val modelToUnload = model ?: _activeModel.value ?: return
            nativeUnloadModel(modelToUnload.handle)
            if (_activeModel.value?.id == modelToUnload.id) {
                _activeModel.value = null
            }
            Log.d(TAG, "Model unloaded: ${modelToUnload.name}")
        } catch (e: Exception) {
            Log.e(TAG, "Error unloading model", e)
        }
    }

    /**
     * Scan for available models in the models directory.
     */
    suspend fun scanModels(): List<ModelInfo> {
        return try {
            val models = mutableListOf<ModelInfo>()
            modelsDir.listFiles()?.forEach { file ->
                if (isModelFile(file)) {
                    val info = ModelInfo(
                        path = file.absolutePath,
                        name = file.name,
                        format = detectFormat(file.absolutePath),
                        size = file.length(),
                        modified = file.lastModified(),
                        parameters = 0,
                        architecture = "unknown"
                    )
                    models.add(info)
                }
            }
            _availableModels.value = models
            Log.d(TAG, "Found ${models.size} models")
            models
        } catch (e: Exception) {
            Log.e(TAG, "Error scanning models", e)
            emptyList()
        }
    }

    /**
     * Get model by ID.
     */
    suspend fun getModel(modelId: String): ModelInfo? {
        return _availableModels.value.find { it.name.substringBeforeLast(".") == modelId }
    }

    /**
     * Switch to a different model.
     *
     * @param modelPath Path to new model
     * @param onProgress Progress callback
     * @return New loaded model or null
     */
    suspend fun switchModel(
        modelPath: String,
        onProgress: (Float) -> Unit = {}
    ): LoadedModel? {
        return loadModel(modelPath, emptyMap(), onProgress)
    }

    /**
     * Get current active model handle (for native layer).
     */
    fun getActiveModelHandle(): Long {
        return _activeModel.value?.handle ?: 0L
    }

    /**
     * Pre-warm model (allocate resources, optimize).
     */
    suspend fun warmModel(modelHandle: Long): Boolean {
        return try {
            nativeWarmModel(modelHandle)
        } catch (e: Exception) {
            Log.e(TAG, "Error warming model", e)
            false
        }
    }

    /**
     * Check if a model file is valid.
     */
    suspend fun validateModel(modelPath: String): Boolean {
        return try {
            nativeValidateModel(modelPath)
        } catch (e: Exception) {
            Log.e(TAG, "Error validating model", e)
            false
        }
    }

    private fun isModelFile(file: File): Boolean {
        val extensions = setOf(".gguf", ".onnx", ".safetensors", ".bin", ".pt", ".pb")
        return extensions.any { file.name.endsWith(it) }
    }

    private fun detectFormat(path: String): String {
        return when {
            path.endsWith(".gguf") || path.endsWith(".ggml") -> "ggml"
            path.endsWith(".onnx") -> "onnx"
            path.endsWith(".safetensors") -> "safetensors"
            path.endsWith(".bin") || path.endsWith(".pt") -> "pytorch"
            path.endsWith(".pb") -> "tensorflow"
            else -> "unknown"
        }
    }

    companion object {
        private const val TAG = "ModelRegistry"
    }
}

data class LoadedModel(
    val id: String,
    val path: String,
    val handle: Long,
    val name: String,
    val format: String,
    val size: Long,
    val memoryUsage: Long,
    val parameters: Long,
    val architecture: String,
    val loadedAt: Long
)

// Native functions
private external fun nativeLoadModel(
    modelPath: String,
    config: Map<String, Any>,
    onProgress: (Float) -> Unit
): Long

private external fun nativeUnloadModel(handle: Long)

private external fun nativeGetModelMetadata(handle: Long): Map<String, Any>?

private external fun nativeWarmModel(handle: Long): Boolean

private external fun nativeValidateModel(modelPath: String): Boolean
