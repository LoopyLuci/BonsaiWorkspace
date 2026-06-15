package ai.bonsai.buddy.ui.models

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SwipeToDismissBox
import androidx.compose.material3.SwipeToDismissBoxValue
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import ai.bonsai.buddy.data.db.ModelEntity
import ai.bonsai.buddy.data.network.InferenceMode
import ai.bonsai.buddy.data.repository.mobile.ModelsRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@Composable
fun ModelsRoute(
    modifier: Modifier = Modifier,
    viewModel: ModelsViewModel = hiltViewModel()
) {
    val state by viewModel.uiState.collectAsState()
    ModelsScreen(
        state = state,
        onSetMode = viewModel::setMode,
        onToggleModel = viewModel::toggle,
        modifier = modifier
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelsScreen(
    state: ModelsUiState,
    onSetMode: (InferenceMode) -> Unit,
    onToggleModel: (ModelEntity) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("Models", style = MaterialTheme.typography.headlineSmall)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
            InferenceMode.entries.forEach { mode ->
                AssistChip(
                    onClick = { onSetMode(mode) },
                    label = { Text(mode.name.replace('_', ' ')) }
                )
            }
        }

        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.models, key = { it.id }) { model ->
                var dismissed by remember(model.id) { mutableStateOf(false) }
                SwipeToDismissBox(
                    state = androidx.compose.material3.rememberSwipeToDismissBoxState(
                        confirmValueChange = {
                            if (it != SwipeToDismissBoxValue.Settled) {
                                dismissed = true
                                onToggleModel(model)
                            }
                            true
                        }
                    ),
                    backgroundContent = {},
                    content = {
                        Card(
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
                        ) {
                            Column(modifier = Modifier.fillMaxWidth().padding(12.dp)) {
                                Text(model.name, fontWeight = FontWeight.SemiBold)
                                Text("tier=${model.tier ?: "n/a"} quant=${model.quant ?: "n/a"} ram=${model.ram ?: "n/a"}")
                                Text("load=${model.progress}% state=${if (model.loaded) "loaded" else "idle"}")
                            }
                        }
                    }
                )
                if (dismissed) {
                    dismissed = false
                }
            }
        }
    }
}

@HiltViewModel
class ModelsViewModel @Inject constructor(
    private val repository: ModelsRepository
) : ViewModel() {
    private val _uiState = MutableStateFlow(ModelsUiState())
    val uiState: StateFlow<ModelsUiState> = _uiState.asStateFlow()

    init {
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(models = repository.listModels())
        }
    }

    fun setMode(mode: InferenceMode) {
        viewModelScope.launch {
            repository.setMode(mode)
            _uiState.value = _uiState.value.copy(mode = mode)
        }
    }

    fun toggle(model: ModelEntity) {
        viewModelScope.launch {
            if (model.loaded) repository.unload(model.id) else repository.load(model.id)
            refresh()
        }
    }
}

data class ModelsUiState(
    val models: List<ModelEntity> = emptyList(),
    val mode: InferenceMode = InferenceMode.AUTO
)
