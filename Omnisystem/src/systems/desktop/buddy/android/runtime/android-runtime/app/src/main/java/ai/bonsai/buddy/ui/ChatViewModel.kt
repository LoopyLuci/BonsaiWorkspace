package ai.bonsai.buddy.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import ai.bonsai.buddy.data.db.ChatMessageEntity
import ai.bonsai.buddy.data.network.BonsaiApiClient
import ai.bonsai.buddy.data.repository.ChatRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import android.util.Log

private const val TAG = "ChatViewModel"

@HiltViewModel
class ChatViewModel @Inject constructor(
    private val chatRepository: ChatRepository,
    private val apiClient: BonsaiApiClient
) : ViewModel() {
    private val _uiState = MutableStateFlow(ChatUiState())
    val uiState: StateFlow<ChatUiState> = _uiState.asStateFlow()

    private val pageSize = 30
    private var loadedCount = 0
    private var loadingOlder = false
    private var streamJob: Job? = null

    init {
        viewModelScope.launch {
            loadInitialMessages()
        }
    }

    fun loadOlderMessages() {
        if (loadingOlder || !_uiState.value.hasMoreHistory) return
        viewModelScope.launch {
            loadingOlder = true
            try {
                val page = chatRepository.getMessagesPage(limit = pageSize, offset = loadedCount)
                val oldestFirst = page.reversed()
                val merged = oldestFirst + _uiState.value.messages
                loadedCount += page.size
                _uiState.value = _uiState.value.copy(
                    messages = merged.distinctBy { it.id },
                    hasMoreHistory = page.size == pageSize
                )
                Log.i(TAG, "Loaded ${page.size} older messages")
            } catch (e: Exception) {
                Log.e(TAG, "Error loading older messages", e)
            } finally {
                loadingOlder = false
            }
        }
    }

    fun sendMessage(text: String) {
        val prompt = text.trim()
        if (prompt.isBlank()) return

        streamJob?.cancel()
        streamJob = viewModelScope.launch {
            _uiState.value = _uiState.value.copy(
                isSending = true,
                isStreaming = true,
                connectionStatus = "Streaming from desktop..."
            )

            chatRepository.appendMessage(role = "user", content = prompt)
            val assistantId = chatRepository.appendMessage(role = "assistant", content = "")

            var fullResponse = ""
            val streamResult = runCatching {
                apiClient.chatStream(prompt).collect { token ->
                    fullResponse += token
                    chatRepository.updateMessage(id = assistantId, content = fullResponse)
                    replaceMessageContent(id = assistantId, content = fullResponse)
                }
            }

            if (streamResult.isFailure || fullResponse.isBlank()) {
                val fallback = apiClient.sendChatMessage(prompt).getOrElse {
                    "Buddy is unreachable. Check LAN connection and desktop token."
                }
                fullResponse = fallback
                chatRepository.updateMessage(id = assistantId, content = fallback)
                replaceMessageContent(id = assistantId, content = fallback)
            }

            _uiState.value = _uiState.value.copy(
                isSending = false,
                isStreaming = false,
                connectionStatus = if (fullResponse.isNotBlank()) "Connected" else "Disconnected"
            )
            refreshLatestWindow()
        }
    }

    private suspend fun loadInitialMessages() {
        try {
            val page = chatRepository.getMessagesPage(limit = pageSize, offset = 0)
            loadedCount = page.size
            _uiState.value = _uiState.value.copy(
                messages = page.reversed(),
                hasMoreHistory = page.size == pageSize,
                connectionStatus = "Connected"
            )
            Log.i(TAG, "Loaded ${page.size} initial messages")
        } catch (e: Exception) {
            Log.e(TAG, "Error loading initial messages", e)
            _uiState.value = _uiState.value.copy(
                connectionStatus = "Disconnected"
            )
        }
    }

    private suspend fun refreshLatestWindow() {
        try {
            val page = chatRepository.getMessagesPage(limit = loadedCount.coerceAtLeast(pageSize), offset = 0)
            _uiState.value = _uiState.value.copy(
                messages = page.reversed(),
                hasMoreHistory = page.size >= loadedCount
            )
        } catch (e: Exception) {
            Log.e(TAG, "Error refreshing messages", e)
        }
    }

    private fun replaceMessageContent(id: Long, content: String) {
        val updated = _uiState.value.messages.map { msg ->
            if (msg.id == id) msg.copy(content = content, timestamp = System.currentTimeMillis()) else msg
        }
        _uiState.value = _uiState.value.copy(messages = updated)
    }
}

data class ChatUiState(
    val messages: List<ChatMessageEntity> = emptyList(),
    val isSending: Boolean = false,
    val isStreaming: Boolean = false,
    val hasMoreHistory: Boolean = false,
    val connectionStatus: String = "Disconnected"
)
