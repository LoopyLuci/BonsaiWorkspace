package ai.bonsai.buddy.ui.chat

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import ai.bonsai.buddy.data.db.ChatMessageEntity
import ai.bonsai.buddy.data.repository.ChatRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

@HiltViewModel
class ChatViewModel @Inject constructor(
    private val chatRepository: ChatRepository
) : ViewModel() {
    private val sending = MutableStateFlow(false)
    private val status = MutableStateFlow("Connected")

    val uiState: StateFlow<ChatUiState> = combine(
        chatRepository.streamMessages(),
        sending,
        status
    ) { messages, isSending, connectionStatus ->
        ChatUiState(
            messages = messages,
            isSending = isSending,
            connectionStatus = connectionStatus
        )
    }.stateIn(
        scope = viewModelScope,
        started = SharingStarted.WhileSubscribed(5_000),
        initialValue = ChatUiState()
    )

    fun sendMessage(text: String) {
        if (text.isBlank()) return
        viewModelScope.launch {
            sending.value = true
            status.value = "Syncing with desktop..."
            chatRepository.sendUserMessage(text)
                .onFailure { status.value = "Disconnected" }
                .onSuccess { status.value = "Connected" }
            sending.value = false
        }
    }
}

data class ChatUiState(
    val messages: List<ChatMessageEntity> = emptyList(),
    val isSending: Boolean = false,
    val connectionStatus: String = "Disconnected"
)
