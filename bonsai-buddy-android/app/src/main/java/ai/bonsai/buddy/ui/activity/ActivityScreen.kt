package ai.bonsai.buddy.ui.activity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AssistChip
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.pulltorefresh.PullToRefreshBox
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
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
import ai.bonsai.buddy.data.db.ActivityEntity
import ai.bonsai.buddy.data.repository.mobile.ActivityRepository
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@Composable
fun ActivityRoute(
    modifier: Modifier = Modifier,
    viewModel: ActivityViewModel = hiltViewModel()
) {
    val state by viewModel.uiState.collectAsState()
    ActivityScreen(
        state = state,
        onFilter = viewModel::setFilter,
        onRefresh = viewModel::refresh,
        modifier = modifier
    )
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ActivityScreen(
    state: ActivityUiState,
    onFilter: (String) -> Unit,
    onRefresh: () -> Unit,
    modifier: Modifier = Modifier
) {
    val filters = listOf("all", "tools", "models", "swarm", "errors")
    PullToRefreshBox(
        isRefreshing = state.refreshing,
        onRefresh = onRefresh,
        modifier = modifier.fillMaxSize()
    ) {
        Column(modifier = Modifier.fillMaxSize().padding(12.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text("Activity", style = MaterialTheme.typography.headlineSmall)
            androidx.compose.foundation.layout.Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                filters.forEach { filter ->
                    AssistChip(onClick = { onFilter(filter) }, label = { Text(filter) })
                }
            }
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.events, key = { it.id }) { event ->
                    Column(modifier = Modifier.fillMaxWidth()) {
                        Text(event.type.uppercase(), fontWeight = FontWeight.SemiBold)
                        Text(event.message)
                        Text("${event.level ?: "info"} • ${event.timestamp}", style = MaterialTheme.typography.labelSmall)
                    }
                }
            }
        }
    }
}

@HiltViewModel
class ActivityViewModel @Inject constructor(
    private val repository: ActivityRepository
) : ViewModel() {
    private val _uiState = MutableStateFlow(ActivityUiState())
    val uiState: StateFlow<ActivityUiState> = _uiState.asStateFlow()

    private var filter: String = "all"

    init {
        refresh()
    }

    fun setFilter(newFilter: String) {
        filter = newFilter
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(events = repository.list(filter))
        }
    }

    fun refresh() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(refreshing = true)
            repository.refresh()
            _uiState.value = _uiState.value.copy(
                events = repository.list(filter),
                refreshing = false
            )
        }
    }
}

data class ActivityUiState(
    val events: List<ActivityEntity> = emptyList(),
    val refreshing: Boolean = false
)
