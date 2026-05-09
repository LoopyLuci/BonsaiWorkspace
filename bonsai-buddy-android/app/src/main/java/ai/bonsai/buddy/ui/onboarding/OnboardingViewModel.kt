package ai.bonsai.buddy.ui.onboarding

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import ai.bonsai.buddy.data.discovery.DiscoveredWorkspace
import ai.bonsai.buddy.data.discovery.DiscoverySource
import ai.bonsai.buddy.data.discovery.NsdDiscoveryManager
import ai.bonsai.buddy.data.network.BonsaiApiClient
import ai.bonsai.buddy.data.network.ConnectionConfig
import ai.bonsai.buddy.data.storage.SecureConfigStore
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

@HiltViewModel
class OnboardingViewModel @Inject constructor(
    private val discoveryManager: NsdDiscoveryManager,
    private val apiClient: BonsaiApiClient,
    private val secureConfigStore: SecureConfigStore
) : ViewModel() {
    private val selectedHost = MutableStateFlow("")
    private val selectedPort = MutableStateFlow("11420")
    private val tokenInput = MutableStateFlow(secureConfigStore.getToken().orEmpty())
    private val step = MutableStateFlow(OnboardingStep.Discover)
    private val status = MutableStateFlow<String?>(null)
    private val busy = MutableStateFlow(false)

    val discovered: StateFlow<List<DiscoveredWorkspace>> = discoveryManager
        .discoverWorkspaces()
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), emptyList())

    val uiState: StateFlow<OnboardingUiState> = combine(
        step,
        selectedHost,
        selectedPort,
        tokenInput,
        discovered,
        status,
        busy
    ) { values ->
        OnboardingUiState(
            step = values[0] as OnboardingStep,
            selectedHost = values[1] as String,
            selectedPort = values[2] as String,
            token = values[3] as String,
            discovered = values[4] as List<DiscoveredWorkspace>,
            status = values[5] as String?,
            busy = values[6] as Boolean
        )
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), OnboardingUiState())

    fun selectWorkspace(item: DiscoveredWorkspace) {
        selectedHost.value = item.host
        selectedPort.value = item.port.toString()
        status.value = "Selected ${item.name} (${item.source.name.lowercase()})"
    }

    fun setManualEndpoint(endpoint: String) {
        val trimmed = endpoint.trim()
        if (trimmed.isBlank()) return
        val host = trimmed.substringBefore(':').trim()
        val port = trimmed.substringAfter(':', "11420").trim()
        selectedHost.value = host
        selectedPort.value = port
        status.value = "Manual endpoint selected"
    }

    fun setToken(token: String) {
        tokenInput.value = token
    }

    fun setTokenFromQr(value: String) {
        tokenInput.value = value.trim()
        status.value = "Token captured from QR"
    }

    fun nextStep() {
        step.value = when (step.value) {
            OnboardingStep.Discover -> OnboardingStep.Authenticate
            OnboardingStep.Authenticate -> OnboardingStep.Verify
            OnboardingStep.Verify -> OnboardingStep.Verify
        }
    }

    fun prevStep() {
        step.value = when (step.value) {
            OnboardingStep.Discover -> OnboardingStep.Discover
            OnboardingStep.Authenticate -> OnboardingStep.Discover
            OnboardingStep.Verify -> OnboardingStep.Authenticate
        }
    }

    fun verifyAndPersist(onComplete: () -> Unit) {
        viewModelScope.launch {
            val host = selectedHost.value.trim()
            val port = selectedPort.value.toIntOrNull() ?: 11420
            val token = tokenInput.value.trim()
            if (host.isBlank() || token.isBlank()) {
                status.value = "Host and token are required"
                return@launch
            }

            busy.value = true
            status.value = "Verifying desktop connectivity..."
            val config = ConnectionConfig(host = host, buddyPort = port)

            secureConfigStore.saveToken(token)
            val verify = apiClient.checkBuddyHealth(config)

            verify
                .onSuccess {
                    secureConfigStore.saveConnectionConfig(config)
                    status.value = "Connected. Ready to start chatting."
                    onComplete()
                }
                .onFailure { err ->
                    status.value = "Verification failed: ${err.message ?: "Unknown error"}"
                }

            busy.value = false
        }
    }
}

enum class OnboardingStep {
    Discover,
    Authenticate,
    Verify
}

data class OnboardingUiState(
    val step: OnboardingStep = OnboardingStep.Discover,
    val selectedHost: String = "",
    val selectedPort: String = "11420",
    val token: String = "",
    val discovered: List<DiscoveredWorkspace> = emptyList(),
    val status: String? = null,
    val busy: Boolean = false
)
