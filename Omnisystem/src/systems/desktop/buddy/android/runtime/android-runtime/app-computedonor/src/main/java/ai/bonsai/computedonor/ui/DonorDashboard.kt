package ai.bonsai.computedonor.ui

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import ai.bonsai.computedonor.viewmodel.ComputeDonorViewModel
import android.util.Log

private const val TAG = "DonorDashboard"

@Composable
fun DonorDashboardRoute(
    modifier: Modifier = Modifier,
    viewModel: ComputeDonorViewModel = hiltViewModel()
) {
    val donorState by viewModel.donorState.collectAsStateWithLifecycle()
    val cpuUsage by viewModel.cpuUsage.collectAsStateWithLifecycle()
    val memoryUsage by viewModel.memoryUsage.collectAsStateWithLifecycle()
    val isRunning by viewModel.isRunning.collectAsStateWithLifecycle()
    val error by viewModel.error.collectAsStateWithLifecycle()

    DonorDashboard(
        donorState = donorState,
        cpuUsage = cpuUsage,
        memoryUsage = memoryUsage,
        isRunning = isRunning,
        error = error,
        onToggleDonor = { viewModel.toggleDonor() },
        onUpdateCpuAllocation = { viewModel.updateCpuAllocation(it) },
        onUpdateMemoryAllocation = { viewModel.updateMemoryAllocation(it) },
        onDismissError = { viewModel.clearError() },
        modifier = modifier
    )
}

data class DonorState(
    val isEnabled: Boolean = false,
    val cpuAllocation: Float = 0.3f,
    val memoryAllocation: Float = 0.2f,
    val connectedDevices: Int = 0,
    val totalComputeShared: String = "0.00 GFLOPs"
)

@Composable
fun DonorDashboard(
    donorState: DonorState,
    cpuUsage: Float,
    memoryUsage: Float,
    isRunning: Boolean,
    error: String?,
    onToggleDonor: () -> Unit,
    onUpdateCpuAllocation: (Float) -> Unit,
    onUpdateMemoryAllocation: (Float) -> Unit,
    onDismissError: () -> Unit,
    modifier: Modifier = Modifier
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("💾 Compute Donor") },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.primary,
                    titleContentColor = MaterialTheme.colorScheme.onPrimary
                )
            )
        },
        modifier = modifier
    ) { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            if (error != null) {
                item {
                    Surface(
                        modifier = Modifier.fillMaxWidth(),
                        color = MaterialTheme.colorScheme.errorContainer,
                        shape = RoundedCornerShape(8.dp)
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(8.dp)
                        ) {
                            Icon(
                                Icons.Default.Error,
                                "Error",
                                tint = MaterialTheme.colorScheme.error,
                                modifier = Modifier.size(24.dp)
                            )
                            Text(
                                error,
                                color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.bodySmall,
                                modifier = Modifier.weight(1f)
                            )
                            IconButton(onClick = onDismissError, modifier = Modifier.size(24.dp)) {
                                Icon(Icons.Default.Close, "Dismiss")
                            }
                        }
                    }
                }
            }

            // Status Card
            item {
                Card(
                    modifier = Modifier.fillMaxWidth(),
                    colors = CardDefaults.cardColors(
                        containerColor = if (donorState.isEnabled)
                            MaterialTheme.colorScheme.primaryContainer
                        else
                            MaterialTheme.colorScheme.surfaceVariant
                    )
                ) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(12.dp)
                    ) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    "Donor Status",
                                    style = MaterialTheme.typography.titleMedium
                                )
                                Text(
                                    if (donorState.isEnabled) "Active" else "Inactive",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = if (donorState.isEnabled)
                                        Color.Green else MaterialTheme.colorScheme.outline
                                )
                            }
                            Switch(
                                checked = donorState.isEnabled,
                                onCheckedChange = { onToggleDonor() }
                            )
                        }

                        Divider()

                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(12.dp)
                        ) {
                            StatItem(
                                label = "Connected",
                                value = donorState.connectedDevices.toString(),
                                icon = Icons.Default.Devices
                            )
                            StatItem(
                                label = "Computing",
                                value = donorState.totalComputeShared,
                                icon = Icons.Default.Speed
                            )
                        }
                    }
                }
            }

            // CPU Allocation
            item {
                AllocationCard(
                    title = "CPU Allocation",
                    icon = Icons.Default.Memory,
                    value = cpuUsage,
                    allocation = donorState.cpuAllocation,
                    onAllocationChange = onUpdateCpuAllocation
                )
            }

            // Memory Allocation
            item {
                AllocationCard(
                    title = "Memory Allocation",
                    icon = Icons.Default.StorageOutlined,
                    value = memoryUsage,
                    allocation = donorState.memoryAllocation,
                    onAllocationChange = onUpdateMemoryAllocation
                )
            }

            // Schedule Card
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        Text(
                            "Schedule",
                            style = MaterialTheme.typography.titleSmall
                        )
                        ScheduleOption("Always Active")
                        ScheduleOption("Only When Charging")
                        ScheduleOption("Custom Hours", selected = true)
                    }
                }
            }

            // Battery & Temperature Warnings
            item {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(80.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    StatusWarningCard(
                        icon = Icons.Default.BatteryAlert,
                        label = "Battery",
                        status = "Normal"
                    )
                    StatusWarningCard(
                        icon = Icons.Default.Thermostat,
                        label = "Temperature",
                        status = "Cool"
                    )
                }
            }
        }
    }
}

@Composable
fun AllocationCard(
    title: String,
    icon: androidx.compose.material.icons.materialIcon.MaterialIcon,
    value: Float,
    allocation: Float,
    onAllocationChange: (Float) -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(icon, title, modifier = Modifier.size(24.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(title, style = MaterialTheme.typography.titleSmall)
                    Text(
                        "Current: ${(value * 100).toInt()}%",
                        style = MaterialTheme.typography.labelSmall
                    )
                }
            }

            LinearProgressIndicator(
                modifier = Modifier.fillMaxWidth(),
                progress = { value }
            )

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Allocate:", style = MaterialTheme.typography.labelSmall)
                Slider(
                    value = allocation,
                    onValueChange = onAllocationChange,
                    valueRange = 0f..1f,
                    modifier = Modifier.weight(1f)
                )
                Text(
                    "${(allocation * 100).toInt()}%",
                    style = MaterialTheme.typography.labelSmall
                )
            }
        }
    }
}

@Composable
fun StatItem(
    label: String,
    value: String,
    icon: androidx.compose.material.icons.materialIcon.MaterialIcon
) {
    Column(
        modifier = Modifier
            .weight(1f)
            .padding(8.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        Icon(icon, label, modifier = Modifier.size(24.dp))
        Text(label, style = MaterialTheme.typography.labelSmall)
        Text(value, style = MaterialTheme.typography.titleSmall)
    }
}

@Composable
fun ScheduleOption(
    label: String,
    selected: Boolean = false
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        RadioButton(selected = selected, onClick = null)
        Text(label, style = MaterialTheme.typography.bodySmall)
    }
}

@Composable
fun StatusWarningCard(
    icon: androidx.compose.material.icons.materialIcon.MaterialIcon,
    label: String,
    status: String
) {
    Card(
        modifier = Modifier
            .weight(1f)
            .fillMaxHeight()
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(8.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Icon(icon, label, modifier = Modifier.size(24.dp))
            Text(label, style = MaterialTheme.typography.labelSmall)
            Text(status, style = MaterialTheme.typography.labelSmall)
        }
    }
}
