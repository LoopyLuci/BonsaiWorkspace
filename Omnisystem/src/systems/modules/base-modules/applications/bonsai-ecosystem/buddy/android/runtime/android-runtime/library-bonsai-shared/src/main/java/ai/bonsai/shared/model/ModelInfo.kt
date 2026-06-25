package ai.bonsai.shared.model

import java.util.UUID

/**
 * Model metadata and information.
 */
data class ModelInfo(
    val id: String = UUID.randomUUID().toString(),
    val path: String,
    val name: String,
    val version: String = "1.0",
    val format: String,
    val sizeBytes: Long,
    val modified: Long = System.currentTimeMillis(),
    val parameters: Long = 0,
    val architecture: String = "unknown",
    val isQuantized: Boolean = false,
    val quantizationLevel: String? = null,
    val author: String? = null,
    val license: String? = null,
    val description: String? = null,
    val tags: List<String> = emptyList()
) {
    // Backwards compatibility properties
    @Deprecated("Use sizeBytes instead")
    val size: Long get() = sizeBytes
}

/**
 * Represents a model that's currently loaded and available for inference.
 */
data class AvailableModel(
    val id: String,
    val name: String,
    val description: String,
    val modelFormat: String,
    val sizeBytes: Long,
    val parameters: String,
    val quantization: String? = null,
    val isLocal: Boolean = true,
    val path: String? = null,
    val remoteUrl: String? = null
)
