package ai.bonsai.computedonor.viewmodel

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import android.util.Log
import javax.inject.Inject
import ai.bonsai.computedonor.ui.DonorState
import kotlin.math.sin

private const val TAG = "ComputeDonorViewModel"

@HiltViewModel
class ComputeDonorViewModel @Inject constructor() : ViewModel() {

    private val _donorState = MutableStateFlow(DonorState())
    val donorState: StateFlow<DonorState> = _donorState.asStateFlow()

    private val _cpuUsage = MutableStateFlow(0f)
    val cpuUsage: StateFlow<Float> = _cpuUsage.asStateFlow()

    private val _memoryUsage = MutableStateFlow(0f)
    val memoryUsage: StateFlow<Float> = _memoryUsage.asStateFlow()

    private val _isRunning = MutableStateFlow(false)
    val isRunning: StateFlow<Boolean> = _isRunning.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private var simulationCounter = 0

    init {
        startResourceMonitoring()
    }

    private fun startResourceMonitoring() {
        viewModelScope.launch {
            while (true) {
                try {
                    simulateResourceMetrics()
                    kotlinx.coroutines.delay(1000)
                } catch (e: Exception) {
                    Log.e(TAG, "Error in resource monitoring", e)
                }
            }
        }
    }

    private fun simulateResourceMetrics() {
        simulationCounter++
        val cpuBase = _donorState.value.cpuAllocation * 0.7f
        val memoryBase = _donorState.value.memoryAllocation * 0.6f

        val cpuVariance = 0.2f * sin(simulationCounter * 0.05f).toFloat()
        val memVariance = 0.15f * sin(simulationCounter * 0.03f).toFloat()

        _cpuUsage.value = (cpuBase + cpuVariance).coerceIn(0f, 1f)
        _memoryUsage.value = (memoryBase + memVariance).coerceIn(0f, 1f)

        if (_donorState.value.isEnabled) {
            _isRunning.value = true
        }
    }

    fun toggleDonor() {
        val newState = _donorState.value.copy(
            isEnabled = !_donorState.value.isEnabled
        )
        _donorState.value = newState
        Log.i(TAG, "Donor toggled: ${newState.isEnabled}")

        if (!newState.isEnabled) {
            _cpuUsage.value = 0f
            _memoryUsage.value = 0f
            _isRunning.value = false
        }
    }

    fun updateCpuAllocation(allocation: Float) {
        val newState = _donorState.value.copy(
            cpuAllocation = allocation
        )
        _donorState.value = newState
        Log.i(TAG, "CPU allocation updated: $allocation")
    }

    fun updateMemoryAllocation(allocation: Float) {
        val newState = _donorState.value.copy(
            memoryAllocation = allocation
        )
        _donorState.value = newState
        Log.i(TAG, "Memory allocation updated: $allocation")
    }

    fun clearError() {
        _error.value = null
    }
}
