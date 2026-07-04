package ai.bonsai.shared.model

import android.content.Context
import android.util.Log
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Wrapper for on-device model format conversion.
 * Supports converting between ONNX, SafeTensors, and GGML formats.
 */
class ModelConverter(private val context: Context) {
    private val modelDir = File(context.getExternalFilesDir(null), "models")

    init {
        modelDir.mkdirs()
    }

    /**
     * Convert model from source format to target format.
     *
     * @param sourcePath Path to source model file
     * @param targetFormat Target format (ggml_q4_k_m, onnx, safetensors)
     * @param onProgress Callback with (current, total) bytes for progress tracking
     * @return Path to converted model or null if conversion failed
     */
    suspend fun convertModel(
        sourcePath: String,
        targetFormat: String,
        onProgress: (Long, Long) -> Unit = { _, _ -> }
    ): String? = withContext(Dispatchers.Default) {
        return@withContext try {
            val sourceFile = File(sourcePath)
            if (!sourceFile.exists()) {
                Log.e(TAG, "Source model not found: $sourcePath")
                return@withContext null
            }

            val extension = when (targetFormat.lowercase()) {
                "ggml_q4_k_m", "ggml_q4_0", "ggml_q8_0" -> ".gguf"
                "onnx" -> ".onnx"
                "safetensors" -> ".safetensors"
                else -> {
                    Log.e(TAG, "Unknown target format: $targetFormat")
                    return@withContext null
                }
            }

            val targetFile = File(modelDir, sourceFile.nameWithoutExtension + extension)

            // Call native conversion function
            val success = nativeConvertModel(
                sourcePath,
                targetFile.absolutePath,
                targetFormat,
                { current, total -> onProgress(current, total) }
            )

            if (success) {
                Log.d(TAG, "Model converted successfully: ${targetFile.absolutePath}")
                targetFile.absolutePath
            } else {
                Log.e(TAG, "Model conversion failed")
                null
            }
        } catch (e: Exception) {
            Log.e(TAG, "Conversion error", e)
            null
        }
    }

    /**
     * Quantize a model to reduce file size.
     *
     * @param modelPath Path to model to quantize
     * @param quantizationLevel Level of quantization (q4_k_m, q4_0, q8_0)
     * @param onProgress Callback with progress
     * @return Path to quantized model or null
     */
    suspend fun quantizeModel(
        modelPath: String,
        quantizationLevel: String = "q4_k_m",
        onProgress: (Float) -> Unit = {}
    ): String? = withContext(Dispatchers.Default) {
        return@withContext try {
            val sourceFile = File(modelPath)
            if (!sourceFile.exists()) {
                Log.e(TAG, "Model not found: $modelPath")
                return@withContext null
            }

            val targetFile = File(modelDir, "${sourceFile.nameWithoutExtension}_${quantizationLevel}.gguf")

            // Call native quantization function
            val success = nativeQuantizeModel(
                modelPath,
                targetFile.absolutePath,
                quantizationLevel,
                { progress -> onProgress(progress) }
            )

            if (success) {
                Log.d(TAG, "Model quantized: ${targetFile.absolutePath}")
                targetFile.absolutePath
            } else {
                Log.e(TAG, "Quantization failed")
                null
            }
        } catch (e: Exception) {
            Log.e(TAG, "Quantization error", e)
            null
        }
    }

    /**
     * Get model information without loading it fully.
     */
    suspend fun getModelInfo(modelPath: String): ModelInfo? = withContext(Dispatchers.Default) {
        return@withContext try {
            val file = File(modelPath)
            if (!file.exists()) return@withContext null

            val format = detectFormat(modelPath)
            val size = file.length()
            val modified = file.lastModified()

            // Get metadata from native layer
            val metadata = nativeGetModelMetadata(modelPath)

            ModelInfo(
                path = modelPath,
                name = file.name,
                format = format,
                size = size,
                modified = modified,
                parameters = metadata?.get("parameters") as? Long ?: 0,
                architecture = metadata?.get("architecture") as? String ?: "unknown"
            )
        } catch (e: Exception) {
            Log.e(TAG, "Error getting model info", e)
            null
        }
    }

    /**
     * Validate model integrity.
     */
    suspend fun validateModel(modelPath: String): Boolean = withContext(Dispatchers.Default) {
        return@withContext try {
            nativeValidateModel(modelPath)
        } catch (e: Exception) {
            Log.e(TAG, "Validation error", e)
            false
        }
    }

    /**
     * Detect model format from file path/header.
     */
    private fun detectFormat(path: String): String {
        return when {
            path.endsWith(".gguf") || path.endsWith(".ggml") -> "ggml"
            path.endsWith(".onnx") -> "onnx"
            path.endsWith(".safetensors") -> "safetensors"
            path.endsWith(".bin") || path.endsWith(".pt") -> "pytorch"
            else -> "unknown"
        }
    }

    companion object {
        private const val TAG = "ModelConverter"
    }
}

data class ModelInfo(
    val path: String,
    val name: String,
    val format: String,
    val size: Long,
    val modified: Long,
    val parameters: Long,
    val architecture: String
)

// Native conversion functions
private external fun nativeConvertModel(
    sourcePath: String,
    targetPath: String,
    targetFormat: String,
    onProgress: (Long, Long) -> Unit
): Boolean

private external fun nativeQuantizeModel(
    sourcePath: String,
    targetPath: String,
    quantizationLevel: String,
    onProgress: (Float) -> Unit
): Boolean

private external fun nativeGetModelMetadata(
    modelPath: String
): Map<String, Any>?

private external fun nativeValidateModel(
    modelPath: String
): Boolean
