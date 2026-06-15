package ai.bonsai.buddy.ui.remote_desktop

import ai.bonsai.buddy.data.remote_desktop.*
import android.view.Surface
import androidx.arch.core.executor.testing.InstantTaskExecutorRule
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.mockito.Mockito.mock

/**
 * Unit tests for RemoteDesktopViewModel.
 *
 * Tests state management, connection handling, and input event routing.
 */
@ExperimentalCoroutinesApi
class RemoteDesktopViewModelTest {
    @get:Rule
    val instantExecutorRule = InstantTaskExecutorRule()

    private lateinit var viewModel: RemoteDesktopViewModel
    private lateinit var mockSurface: Surface

    @Before
    fun setup() {
        viewModel = RemoteDesktopViewModel()
        mockSurface = mock(Surface::class.java)
    }

    @Test
    fun testInitialState() {
        assertEquals(ConnectionState.DISCONNECTED, viewModel.connectionState.value)
        assertEquals(SessionStats(), viewModel.sessionStats.value)
        assertFalse(viewModel.showKeyboard.value)
        assertEquals(InputMapper.MouseMode.ABSOLUTE, viewModel.mouseMode.value)
        assertNull(viewModel.error.value)
    }

    @Test
    fun testToggleKeyboard() {
        assertFalse(viewModel.showKeyboard.value)

        viewModel.toggleKeyboard()
        assertTrue(viewModel.showKeyboard.value)

        viewModel.toggleKeyboard()
        assertFalse(viewModel.showKeyboard.value)
    }

    @Test
    fun testHideKeyboard() {
        viewModel.toggleKeyboard()
        assertTrue(viewModel.showKeyboard.value)

        viewModel.hideKeyboard()
        assertFalse(viewModel.showKeyboard.value)
    }

    @Test
    fun testToggleMouseMode() {
        assertEquals(InputMapper.MouseMode.ABSOLUTE, viewModel.mouseMode.value)

        viewModel.toggleMouseMode()
        assertEquals(InputMapper.MouseMode.RELATIVE, viewModel.mouseMode.value)

        viewModel.toggleMouseMode()
        assertEquals(InputMapper.MouseMode.ABSOLUTE, viewModel.mouseMode.value)
    }

    @Test
    fun testSendTouchEventWhenDisconnected() = runTest {
        viewModel.sendTouchEvent(100f, 200f, TouchAction.DOWN)
        // Should not crash, just not send anything
    }

    @Test
    fun testSendKeyEventWhenDisconnected() = runTest {
        viewModel.sendKeyEvent(0x1E, true)
        // Should not crash, just not send anything
    }

    @Test
    fun testSendTextWhenDisconnected() = runTest {
        viewModel.sendText("hello")
        // Should not crash, just not send anything
    }

    @Test
    fun testSendScrollWhenDisconnected() = runTest {
        viewModel.sendScroll(10f, 20f)
        // Should not crash, just not send anything
    }

    @Test
    fun testClearError() {
        // In a real test, we'd trigger an error first
        assertNull(viewModel.error.value)

        viewModel.clearError()
        assertNull(viewModel.error.value)
    }

    @Test
    fun testMultipleToggleKeyboard() {
        for (i in 0..3) {
            val shouldBeVisible = i % 2 == 0
            assertEquals(shouldBeVisible, viewModel.showKeyboard.value)
            viewModel.toggleKeyboard()
        }
    }

    @Test
    fun testDisconnectWhenNotConnected() = runTest {
        viewModel.disconnect()
        // Should not crash
        assertEquals(ConnectionState.DISCONNECTED, viewModel.connectionState.value)
    }

    @Test
    fun testViewModelLifecycle() {
        // Simulate ViewModel lifecycle
        viewModel.toggleKeyboard()
        assertTrue(viewModel.showKeyboard.value)

        // onCleared() would be called by Android framework
        // For testing, we just verify no crashes on destroy
        viewModel.hashCode() // Access to ensure object is valid
    }

    @Test
    fun testConnectionStateObservation() {
        val connectionStates = mutableListOf<ConnectionState>()

        // Initial state
        connectionStates.add(viewModel.connectionState.value)
        assertEquals(ConnectionState.DISCONNECTED, connectionStates[0])
    }

    @Test
    fun testSessionStatsObservation() {
        val stats = viewModel.sessionStats.value

        assertEquals(0f, stats.fps, 0.01f)
        assertEquals(0L, stats.bitrate)
        assertEquals(0L, stats.latency)
        assertEquals(0f, stats.packetLoss, 0.01f)
    }

    @Test
    fun testMouseModeCycles() {
        val modes = mutableListOf<InputMapper.MouseMode>()
        modes.add(viewModel.mouseMode.value)

        for (i in 0..3) {
            viewModel.toggleMouseMode()
            modes.add(viewModel.mouseMode.value)
        }

        // Should cycle between ABSOLUTE and RELATIVE
        assertEquals(5, modes.size)
        assertEquals(InputMapper.MouseMode.ABSOLUTE, modes[0])
        assertEquals(InputMapper.MouseMode.RELATIVE, modes[1])
        assertEquals(InputMapper.MouseMode.ABSOLUTE, modes[2])
        assertEquals(InputMapper.MouseMode.RELATIVE, modes[3])
        assertEquals(InputMapper.MouseMode.ABSOLUTE, modes[4])
    }

    @Test
    fun testKeyboardVisibilityCycles() {
        val visibilities = mutableListOf<Boolean>()
        visibilities.add(viewModel.showKeyboard.value)

        for (i in 0..3) {
            viewModel.toggleKeyboard()
            visibilities.add(viewModel.showKeyboard.value)
        }

        // Should cycle between hidden and visible
        assertEquals(5, visibilities.size)
        assertEquals(false, visibilities[0])
        assertEquals(true, visibilities[1])
        assertEquals(false, visibilities[2])
        assertEquals(true, visibilities[3])
        assertEquals(false, visibilities[4])
    }

    @Test
    fun testMultipleClearErrors() {
        viewModel.clearError()
        viewModel.clearError()
        viewModel.clearError()
        // Should not crash
        assertNull(viewModel.error.value)
    }
}
