package ai.bonsai.nodecontroller.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import android.util.Log
import java.util.*
import javax.inject.Inject
import ai.bonsai.nodecontroller.ui.DeviceInfo

private const val TAG = "NodeControllerViewModel"

@HiltViewModel
class NodeControllerViewModel @Inject constructor() : ViewModel() {

    private val _devices = MutableStateFlow<List<DeviceInfo>>(emptyList())
    val devices: StateFlow<List<DeviceInfo>> = _devices.asStateFlow()

    private val _isLoading = MutableStateFlow(false)
    val isLoading: StateFlow<Boolean> = _isLoading.asStateFlow()

    private val _isScanning = MutableStateFlow(false)
    val isScanning: StateFlow<Boolean> = _isScanning.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _selectedDevice = MutableStateFlow<String?>(null)
    val selectedDevice: StateFlow<String?> = _selectedDevice.asStateFlow()

    init {
        scanDevices()
    }

    fun scanDevices() {
        viewModelScope.launch {
            _isScanning.value = true
            _error.value = null
            try {
                // Simulate network scan
                kotlinx.coroutines.delay(2000)

                val mockDevices = listOf(
                    DeviceInfo(
                        id = UUID.randomUUID().toString(),
                        name = "Kitchen Tablet",
                        address = "192.168.1.101",
                        status = "Active",
                        isOnline = true,
                        modelName = "iPad Air",
                        osVersion = "iPadOS 17",
                        battery = 85,
                        signal = 95
                    ),
                    DeviceInfo(
                        id = UUID.randomUUID().toString(),
                        name = "Bedroom Phone",
                        address = "192.168.1.102",
                        status = "Active",
                        isOnline = true,
                        modelName = "Pixel 8 Pro",
                        osVersion = "Android 14",
                        battery = 72,
                        signal = 78
                    ),
                    DeviceInfo(
                        id = UUID.randomUUID().toString(),
                        name = "Living Room TV",
                        address = "192.168.1.103",
                        status = "Idle",
                        isOnline = true,
                        modelName = "Android TV",
                        osVersion = "Android 12",
                        battery = 100,
                        signal = 92
                    )
                )
                _devices.value = mockDevices
                Log.i(TAG, "Scanned ${mockDevices.size} devices")
            } catch (e: Exception) {
                Log.e(TAG, "Error scanning devices", e)
                _error.value = "Failed to scan devices: ${e.message}"
            } finally {
                _isScanning.value = false
            }
        }
    }

    fun selectDevice(deviceId: String) {
        _selectedDevice.value = deviceId
        Log.i(TAG, "Selected device: $deviceId")
    }

    fun clearError() {
        _error.value = null
    }
}
