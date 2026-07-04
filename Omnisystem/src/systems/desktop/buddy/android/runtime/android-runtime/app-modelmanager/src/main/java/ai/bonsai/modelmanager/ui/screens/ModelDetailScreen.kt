package ai.bonsai.modelmanager.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import ai.bonsai.modelmanager.viewmodel.ModelManagerViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelDetailScreen(
    modelId: String,
    viewModel: ModelManagerViewModel,
    onBack: () -> Unit,
    onTest: () -> Unit
) {
    val selectedModel = viewModel.selectedModel.collectAsState()
    val conversionProgress = viewModel.conversionProgress.collectAsState()
    val isLoading = viewModel.isLoading.collectAsState()
    val error = viewModel.error.collectAsState()

    var showQuantizationDialog by remember { mutableStateOf(false) }
    var selectedQuantLevel by remember { mutableStateOf("q4_k_m") }

    LaunchedEffect(Unit) {
        val model = viewModel.availableModels.value.find { it.name == modelId }
        if (model != null) {
            viewModel.selectModel(model)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Model Details") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
        ) {
            selectedModel.value?.let { model ->
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .verticalScroll(rememberScrollState())
                        .padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    // Model Info Card
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text(
                                text = model.name,
                                style = MaterialTheme.typography.headlineSmall
                            )
                            Divider()
                            InfoRow("Format", model.format)
                            InfoRow("Size", formatBytes(model.size))
                            InfoRow("Architecture", model.architecture)
                            InfoRow("Parameters", "${model.parameters}")
                        }
                    }

                    // Action Buttons
                    Button(
                        onClick = onTest,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text("Test in Bonsai Buddy")
                    }

                    Button(
                        onClick = { showQuantizationDialog = true },
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.buttonColors(
                            containerColor = MaterialTheme.colorScheme.secondary
                        )
                    ) {
                        Text("Quantize Model")
                    }

                    OutlinedButton(
                        onClick = { viewModel.deleteModel(model.path) },
                        modifier = Modifier.fillMaxWidth(),
                        colors = ButtonDefaults.outlinedButtonColors(
                            contentColor = MaterialTheme.colorScheme.error
                        )
                    ) {
                        Text("Delete Model")
                    }

                    // Progress indicators
                    if (isLoading.value) {
                        LinearProgressIndicator(
                            modifier = Modifier.fillMaxWidth(),
                            progress = { conversionProgress.value }
                        )
                    }

                    // Error display
                    if (error.value != null) {
                        Card(
                            modifier = Modifier.fillMaxWidth(),
                            colors = CardDefaults.cardColors(
                                containerColor = MaterialTheme.colorScheme.errorContainer
                            )
                        ) {
                            Text(
                                text = error.value ?: "",
                                modifier = Modifier.padding(12.dp),
                                color = MaterialTheme.colorScheme.error
                            )
                        }
                    }
                }
            }

            // Quantization Dialog
            if (showQuantizationDialog) {
                AlertDialog(
                    onDismissRequest = { showQuantizationDialog = false },
                    title = { Text("Select Quantization Level") },
                    text = {
                        Column(
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            QuantizationOption(
                                label = "Q4_K_M (Recommended)",
                                value = "q4_k_m",
                                selected = selectedQuantLevel == "q4_k_m",
                                onSelect = { selectedQuantLevel = "q4_k_m" }
                            )
                            QuantizationOption(
                                label = "Q4_0",
                                value = "q4_0",
                                selected = selectedQuantLevel == "q4_0",
                                onSelect = { selectedQuantLevel = "q4_0" }
                            )
                            QuantizationOption(
                                label = "Q8_0",
                                value = "q8_0",
                                selected = selectedQuantLevel == "q8_0",
                                onSelect = { selectedQuantLevel = "q8_0" }
                            )
                        }
                    },
                    confirmButton = {
                        Button(
                            onClick = {
                                viewModel.quantizeModel(selectedModel.value?.path ?: "", selectedQuantLevel) {
                                    showQuantizationDialog = false
                                }
                            }
                        ) {
                            Text("Quantize")
                        }
                    },
                    dismissButton = {
                        TextButton(
                            onClick = { showQuantizationDialog = false }
                        ) {
                            Text("Cancel")
                        }
                    }
                )
            }
        }
    }
}

@Composable
private fun InfoRow(label: String, value: String) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurface
        )
    }
}

@Composable
private fun QuantizationOption(
    label: String,
    value: String,
    selected: Boolean,
    onSelect: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        RadioButton(
            selected = selected,
            onClick = onSelect
        )
        Text(label)
    }
}

private fun formatBytes(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "${bytes / 1024} KB"
        bytes < 1024 * 1024 * 1024 -> "${bytes / (1024 * 1024)} MB"
        else -> "${bytes / (1024 * 1024 * 1024)} GB"
    }
}
