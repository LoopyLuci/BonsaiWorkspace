package ai.bonsai.buddy.ui.remote_desktop

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

data class RemoteDesktopState(
    val isConnected: Boolean = false,
    val fps: Int = 0,
    val latencyMs: Long = 0
)

class RemoteDesktopViewModel(
    peerId: String = "",
    tokenBase64: String = ""
) : ViewModel() {
    private val _state = MutableStateFlow(RemoteDesktopState())
    val state: StateFlow<RemoteDesktopState> = _state

    init {
        viewModelScope.launch {
            // Initialize
        }
    }

    fun connectToPeer() {
        viewModelScope.launch {
            // Connect
        }
    }

    fun disconnectPeer() {
        viewModelScope.launch {
            // Disconnect
        }
    }

    fun injectTouch(x: Float, y: Float, action: String) {
        viewModelScope.launch {
            // Inject touch
        }
    }

    fun injectKey(keyCode: Int, down: Boolean) {
        viewModelScope.launch {
            // Inject key
        }
    }

    fun injectText(text: String) {
        viewModelScope.launch {
            // Inject text
        }
    }

    fun injectScroll(x: Float, y: Float, direction: String) {
        viewModelScope.launch {
            // Inject scroll
        }
    }

    override fun onCleared() {
        super.onCleared()
        // Cleanup
    }
}
