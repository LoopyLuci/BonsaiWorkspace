package ai.bonsai.buddy.ui.onboarding

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.Preview
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.hilt.navigation.compose.hiltViewModel
import ai.bonsai.buddy.data.discovery.DiscoveredWorkspace
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.common.InputImage
import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

@Composable
fun OnboardingRoute(
    onOnboardingComplete: () -> Unit,
    modifier: Modifier = Modifier,
    viewModel: OnboardingViewModel = hiltViewModel()
) {
    val state by viewModel.uiState.collectAsState()
    OnboardingScreen(
        state = state,
        onSelectWorkspace = viewModel::selectWorkspace,
        onManualEndpoint = viewModel::setManualEndpoint,
        onTokenChange = viewModel::setToken,
        onQrToken = viewModel::setTokenFromQr,
        onNext = viewModel::nextStep,
        onBack = viewModel::prevStep,
        onVerify = { viewModel.verifyAndPersist(onOnboardingComplete) },
        modifier = modifier
    )
}

@Composable
fun OnboardingScreen(
    state: OnboardingUiState,
    onSelectWorkspace: (DiscoveredWorkspace) -> Unit,
    onManualEndpoint: (String) -> Unit,
    onTokenChange: (String) -> Unit,
    onQrToken: (String) -> Unit,
    onNext: () -> Unit,
    onBack: () -> Unit,
    onVerify: () -> Unit,
    modifier: Modifier = Modifier
) {
    var manualEndpoint by remember { mutableStateOf("") }
    var showScanner by remember { mutableStateOf(false) }

    val context = LocalContext.current
    val requestCameraPermission = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { granted ->
        showScanner = granted
    }

    Column(
        modifier = modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Text("Bonsai Buddy Setup", style = MaterialTheme.typography.headlineSmall)
        Text(
            "Step ${state.step.ordinal + 1} of 3",
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.primary
        )

        when (state.step) {
            OnboardingStep.Discover -> {
                Text("Discover Bonsai Workspace instances on your LAN.")
                LazyColumn(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(220.dp),
                    verticalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    items(state.discovered, key = { "${it.host}:${it.port}" }) { item ->
                        WorkspaceCard(item = item, onClick = { onSelectWorkspace(item) })
                    }
                }
                OutlinedTextField(
                    value = manualEndpoint,
                    onValueChange = { manualEndpoint = it },
                    label = { Text("Manual host:port") },
                    placeholder = { Text("192.168.1.20:11420") },
                    modifier = Modifier.fillMaxWidth()
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = { onManualEndpoint(manualEndpoint) }) {
                        Text("Use Manual Endpoint")
                    }
                    Button(onClick = onNext, enabled = state.selectedHost.isNotBlank()) {
                        Text("Continue")
                    }
                }
            }

            OnboardingStep.Authenticate -> {
                Text("Enter desktop connection token.")
                OutlinedTextField(
                    value = state.token,
                    onValueChange = onTokenChange,
                    label = { Text("Desktop token") },
                    modifier = Modifier.fillMaxWidth()
                )
                Button(onClick = {
                    val granted = ContextCompat.checkSelfPermission(
                        context,
                        Manifest.permission.CAMERA
                    ) == PackageManager.PERMISSION_GRANTED

                    if (granted) {
                        showScanner = true
                    } else {
                        requestCameraPermission.launch(Manifest.permission.CAMERA)
                    }
                }) {
                    Icon(Icons.Default.QrCodeScanner, contentDescription = null)
                    Text("  Scan QR Code")
                }
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = onBack) { Text("Back") }
                    Button(onClick = onNext, enabled = state.token.isNotBlank()) {
                        Text("Continue")
                    }
                }
            }

            OnboardingStep.Verify -> {
                Text("Verify connection against /health.")
                Card(
                    colors = CardDefaults.cardColors(
                        containerColor = MaterialTheme.colorScheme.surfaceVariant
                    )
                ) {
                    Column(modifier = Modifier.padding(12.dp)) {
                        Text("Host: ${state.selectedHost}:${state.selectedPort}")
                        Text("Token: ${if (state.token.isBlank()) "Not set" else "Configured"}")
                    }
                }

                if (state.busy) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        CircularProgressIndicator(modifier = Modifier.padding(end = 8.dp))
                        Text("Checking connectivity...")
                    }
                }

                if (state.status?.contains("Connected", ignoreCase = true) == true) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Default.CheckCircle, contentDescription = null)
                        Text("  ${state.status}")
                    }
                }

                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    TextButton(onClick = onBack) { Text("Back") }
                    Button(onClick = onVerify, enabled = !state.busy) {
                        Text("Start Chatting")
                    }
                }
            }
        }

        state.status?.let {
            Text(
                text = it,
                style = MaterialTheme.typography.bodyMedium,
                modifier = Modifier
                    .fillMaxWidth()
                    .background(
                        MaterialTheme.colorScheme.surfaceVariant,
                        shape = RoundedCornerShape(8.dp)
                    )
                    .padding(10.dp)
                    .alpha(0.95f)
            )
        }

        if (showScanner) {
            QrScannerDialog(
                onDismiss = { showScanner = false },
                onTokenScanned = { token ->
                    onQrToken(token)
                    showScanner = false
                }
            )
        }
    }
}

@Composable
private fun QrScannerDialog(
    onDismiss: () -> Unit,
    onTokenScanned: (String) -> Unit
) {
    AlertDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton(onClick = onDismiss) {
                Icon(Icons.Default.Close, contentDescription = null)
                Text(" Close")
            }
        },
        title = { Text("Scan Desktop Token QR") },
        text = {
            Box(modifier = Modifier.height(320.dp)) {
                QrScannerPreview(onTokenScanned = onTokenScanned)
            }
        }
    )
}

@androidx.annotation.OptIn(ExperimentalGetImage::class)
@Composable
private fun QrScannerPreview(onTokenScanned: (String) -> Unit) {
    val lifecycleOwner = LocalLifecycleOwner.current
    val scanner = remember {
        val options = BarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build()
        BarcodeScanning.getClient(options)
    }
    val executor = remember { Executors.newSingleThreadExecutor() }

    DisposableEffect(Unit) {
        onDispose {
            scanner.close()
            executor.shutdown()
        }
    }

    AndroidView(
        modifier = Modifier.fillMaxSize(),
        factory = { ctx ->
            val previewView = PreviewView(ctx)
            val cameraProviderFuture = ProcessCameraProvider.getInstance(ctx)

            cameraProviderFuture.addListener({
                val cameraProvider = cameraProviderFuture.get()
                val preview = Preview.Builder().build().also {
                    it.setSurfaceProvider(previewView.surfaceProvider)
                }

                val analysis = ImageAnalysis.Builder().build().also { imageAnalysis ->
                    imageAnalysis.setAnalyzer(executor) { imageProxy ->
                        val mediaImage = imageProxy.image
                        if (mediaImage == null) {
                            imageProxy.close()
                            return@setAnalyzer
                        }

                        val inputImage = InputImage.fromMediaImage(
                            mediaImage,
                            imageProxy.imageInfo.rotationDegrees
                        )

                        scanner.process(inputImage)
                            .addOnSuccessListener { barcodes ->
                                val token = barcodes.firstOrNull()?.rawValue?.trim()
                                if (!token.isNullOrBlank()) {
                                    onTokenScanned(token)
                                }
                            }
                            .addOnCompleteListener {
                                imageProxy.close()
                            }
                    }
                }

                cameraProvider.unbindAll()
                cameraProvider.bindToLifecycle(
                    lifecycleOwner,
                    CameraSelector.DEFAULT_BACK_CAMERA,
                    preview,
                    analysis
                )
            }, ContextCompat.getMainExecutor(ctx))

            previewView
        }
    )
}

@Composable
private fun WorkspaceCard(
    item: DiscoveredWorkspace,
    onClick: () -> Unit
) {
    Card(
        onClick = onClick,
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.secondaryContainer)
    ) {
        Column(modifier = Modifier.padding(12.dp)) {
            Text(item.name, fontWeight = FontWeight.SemiBold)
            Text("${item.host}:${item.port}")
            Text(item.source.name)
        }
    }
}
