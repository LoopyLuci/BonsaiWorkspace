package ai.bonsai.modelmanager.viewmodel

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import ai.bonsai.shared.model.ModelRegistry
import ai.bonsai.shared.model.ModelInfo
import ai.bonsai.shared.model.ModelConverter
import dagger.hilt.android.lifecycle.HiltViewModel
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class ModelManagerViewModel @Inject constructor(
    @ApplicationContext context: Context
) : ViewModel() {
    private val modelRegistry = ModelRegistry(context)
    private val modelConverter = ModelConverter(context)

    private val _availableModels = MutableStateFlow<List<ModelInfo>>(emptyList())
    val availableModels: StateFlow<List<ModelInfo>> = _availableModels.asStateFlow()

    private val _selectedModel = MutableStateFlow<ModelInfo?>(null)
    val selectedModel: StateFlow<ModelInfo?> = _selectedModel.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _downloadProgress = MutableStateFlow<Float>(0f)
    val downloadProgress: StateFlow<Float> = _downloadProgress.asStateFlow()

    private val _conversionProgress = MutableStateFlow<Float>(0f)
    val conversionProgress: StateFlow<Float> = _conversionProgress.asStateFlow()

    init {
        loadModels()
    }

    /**
     * Load all available models from storage.
     */
    fun loadModels() {
        viewModelScope.launch {
            _isLoading.value = true
            try {
                val models = modelRegistry.scanModels()
                _availableModels.value = models
                _error.value = null
            } catch (e: Exception) {
                _error.value = "Failed to load models: ${e.message}"
            } finally {
                _isLoading.value = false
            }
        }
    }

    /**
     * Select a model for detailed view.
     */
    fun selectModel(model: ModelInfo) {
        _selectedModel.value = model
    }

    /**
     * Download a model from Hugging Face Hub.
     */
    fun downloadModel(modelId: String, onComplete: () -> Unit) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            try {
                // Simulate download (would call actual HF Hub API)
                nativeDownloadModel(modelId) { progress ->
                    _downloadProgress.value = progress
                }
                _downloadProgress.value = 1f
                loadModels()
                onComplete()
            } catch (e: Exception) {
                _error.value = "Download failed: ${e.message}"
            } finally {
                _isLoading.value = false
            }
        }
    }

    /**
     * Convert model to different format.
     */
    fun convertModel(modelPath: String, targetFormat: String, onComplete: (String?) -> Unit) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            try {
                val result = modelConverter.convertModel(modelPath, targetFormat) { current, total ->
                    _conversionProgress.value = current.toFloat() / total.coerceAtLeast(1)
                }
                if (result != null) {
                    loadModels()
                    onComplete(result)
                } else {
                    _error.value = "Conversion failed"
                    onComplete(null)
                }
            } catch (e: Exception) {
                _error.value = "Conversion error: ${e.message}"
                onComplete(null)
            } finally {
                _isLoading.value = false
            }
        }
    }

    /**
     * Quantize a model.
     */
    fun quantizeModel(modelPath: String, level: String, onComplete: (String?) -> Unit) {
        viewModelScope.launch {
            _isLoading.value = true
            _error.value = null
            try {
                val result = modelConverter.quantizeModel(modelPath, level) { progress ->
                    _conversionProgress.value = progress
                }
                if (result != null) {
                    loadModels()
                    onComplete(result)
                } else {
                    _error.value = "Quantization failed"
                    onComplete(null)
                }
            } catch (e: Exception) {
                _error.value = "Quantization error: ${e.message}"
                onComplete(null)
            } finally {
                _isLoading.value = false
            }
        }
    }

    /**
     * Delete a model.
     */
    fun deleteModel(modelPath: String) {
        viewModelScope.launch {
            try {
                java.io.File(modelPath).delete()
                loadModels()
                _error.value = null
            } catch (e: Exception) {
                _error.value = "Delete failed: ${e.message}"
            }
        }
    }

    /**
     * Clear error message.
     */
    fun clearError() {
        _error.value = null
    }
}

// Native function for downloading models
private external fun nativeDownloadModel(
    modelId: String,
    onProgress: (Float) -> Unit
)
