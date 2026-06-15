package ai.bonsai.buddy.ui.tools

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import ai.bonsai.buddy.data.db.ToolEntity
import ai.bonsai.buddy.data.repository.mobile.ToolsRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@Composable
fun ToolsRoute(
    modifier: Modifier = Modifier,
    viewModel: ToolsViewModel = hiltViewModel()
) {
    val state by viewModel.uiState.collectAsState()
    ToolsScreen(
        state = state,
        onRefresh = viewModel::refresh,
        onRunTool = viewModel::runTool,
        modifier = modifier
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ToolsScreen(
    state: ToolsUiState,
    onRefresh: () -> Unit,
    onRunTool: (String, Map<String, String>) -> Unit,
    modifier: Modifier = Modifier
) {
    var selected by remember { mutableStateOf<ToolEntity?>(null) }
    val params = remember { mutableStateMapOf<String, String>() }

    Column(modifier = modifier.fillMaxSize().padding(12.dp)) {
        Row(horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
            Text("Tools", style = MaterialTheme.typography.headlineSmall)
            TextButton(onClick = onRefresh) { Text("Refresh") }
        }

        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.tools, key = { it.name }) { tool ->
                Card(
                    onClick = {
                        selected = tool
                        params.clear()
                    },
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer)
                ) {
                    Column(modifier = Modifier.fillMaxWidth().padding(12.dp)) {
                        Text(tool.name, fontWeight = FontWeight.SemiBold)
                        Text(tool.category, style = MaterialTheme.typography.labelMedium)
                        tool.description?.let { Text(it, style = MaterialTheme.typography.bodyMedium) }
                    }
                }
            }
        }
    }

    selected?.let { tool ->
        ModalBottomSheet(onDismissRequest = { selected = null }) {
            Column(modifier = Modifier.fillMaxWidth().padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
                Text(tool.name, style = MaterialTheme.typography.titleLarge)
                OutlinedTextField(
                    value = params["arg"] ?: "",
                    onValueChange = { params["arg"] = it },
                    label = { Text("arg") },
                    modifier = Modifier.fillMaxWidth()
                )
                Button(onClick = { onRunTool(tool.name, params.toMap()) }) {
                    Text("Run")
                }
                if (state.lastResult.isNotBlank()) {
                    Text("Result", style = MaterialTheme.typography.titleMedium)
                    Text(state.lastResult, style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
    }
}

@HiltViewModel
class ToolsViewModel @Inject constructor(
    private val repository: ToolsRepository
) : ViewModel() {
    private val _uiState = MutableStateFlow(ToolsUiState())
    val uiState: StateFlow<ToolsUiState> = _uiState.asStateFlow()

    init {
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(tools = repository.listTools())
        }
    }

    fun runTool(name: String, params: Map<String, String>) {
        viewModelScope.launch {
            val result = repository.runTool(name, params)
            _uiState.value = _uiState.value.copy(lastResult = result)
        }
    }
}

data class ToolsUiState(
    val tools: List<ToolEntity> = emptyList(),
    val lastResult: String = ""
)
