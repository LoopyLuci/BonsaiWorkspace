package ai.bonsai.modelmanager.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import ai.bonsai.modelmanager.viewmodel.ModelManagerViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelDownloaderScreen(
    viewModel: ModelManagerViewModel,
    onBack: () -> Unit,
    onDownloadComplete: () -> Unit
) {
    val downloadProgress = viewModel.downloadProgress.collectAsState()
    val isLoading = viewModel.isLoading.collectAsState()
    val error = viewModel.error.collectAsState()

    var modelId by remember { mutableStateOf("") }
    var isDownloading by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Download Model") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { paddingValues ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    elevation = CardDefaults.cardElevation(defaultElevation = 4.dp)
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Text(
                            text = "Popular Models",
                            style = MaterialTheme.typography.titleMedium
                        )

                        // Popular model suggestions
                        listOf(
                            "meta-llama/Llama-2-7b-hf",
                            "mistralai/Mistral-7B-v0.1",
                            "TheBloke/Llama-2-13B-GGUF",
                            "TheBloke/Mistral-7B-GGUF"
                        ).forEach { model ->
                            ModelSuggestion(
                                model = model,
                                onClick = { modelId = model }
                            )
                        }
                    }
                }
            }

            item {
                Divider()
                Text(
                    text = "Or enter a model ID",
                    style = MaterialTheme.typography.titleSmall,
                    modifier = Modifier.padding(vertical = 8.dp)
                )
            }

            item {
                OutlinedTextField(
                    value = modelId,
                    onValueChange = { modelId = it },
                    label = { Text("Model ID (e.g., meta-llama/Llama-2-7b)") },
                    modifier = Modifier.fillMaxWidth(),
                    enabled = !isDownloading
                )
            }

            if (isDownloading && downloadProgress.value > 0) {
                item {
                    Column(
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        LinearProgressIndicator(
                            modifier = Modifier.fillMaxWidth(),
                            progress = { downloadProgress.value }
                        )
                        Text(
                            text = "${(downloadProgress.value * 100).toInt()}%",
                            style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.align(Alignment.CenterHorizontally)
                        )
                    }
                }
            }

            item {
                Button(
                    onClick = {
                        isDownloading = true
                        viewModel.downloadModel(modelId) {
                            isDownloading = false
                            onDownloadComplete()
                        }
                    },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(48.dp),
                    enabled = modelId.isNotEmpty() && !isDownloading && !isLoading.value
                ) {
                    if (isDownloading || isLoading.value) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(20.dp),
                            strokeWidth = 2.dp
                        )
                    } else {
                        Text("Download")
                    }
                }
            }

            if (error.value != null) {
                item {
                    Card(
                        modifier = Modifier.fillMaxWidth(),
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.errorContainer
                        )
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(16.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Text(
                                text = "Error",
                                style = MaterialTheme.typography.titleMedium,
                                color = MaterialTheme.colorScheme.error
                            )
                            Text(
                                text = error.value ?: "",
                                style = MaterialTheme.typography.bodyMedium
                            )
                            Button(
                                onClick = { viewModel.clearError() },
                                modifier = Modifier.align(Alignment.End)
                            ) {
                                Text("Dismiss")
                            }
                        }
                    }
                }
            }

            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.primaryContainer
                    )
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Text(
                            text = "Tips",
                            style = MaterialTheme.typography.titleSmall
                        )
                        BulletPoint("Models are downloaded from Hugging Face Hub")
                        BulletPoint("Larger models require more storage and RAM")
                        BulletPoint("GGUF format models are optimized for mobile")
                        BulletPoint("You can quantize models to reduce size")
                    }
                }
            }
        }
    }
}

@Composable
private fun ModelSuggestion(
    model: String,
    onClick: () -> Unit
) {
    Button(
        onClick = onClick,
        modifier = Modifier
            .fillMaxWidth()
            .height(40.dp),
        colors = ButtonDefaults.buttonColors(
            containerColor = MaterialTheme.colorScheme.secondaryContainer
        )
    ) {
        Text(
            text = model,
            style = MaterialTheme.typography.bodySmall
        )
    }
}

@Composable
private fun BulletPoint(text: String) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.fillMaxWidth()
    ) {
        Text("•", style = MaterialTheme.typography.bodySmall)
        Text(
            text = text,
            style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.weight(1f)
        )
    }
}
