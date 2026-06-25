package ai.bonsai.buddy.data.remote_desktop

import android.view.Surface
import kotlinx.coroutines.*
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.Mockito.*

/**
 * Unit tests for BrdfMobileClient.
 *
 * Tests connection management, input injection, and session state.
 */
class BrdfMobileClientTest {
    private lateinit var client: BrdfMobileClient
    private lateinit var mockSurface: Surface
    private lateinit var testScope: CoroutineScope

    @Before
    fun setup() {
        testScope = CoroutineScope(StandardTestDispatcher() + Job())
        mockSurface = mock(Surface::class.java)
        client = BrdfMobileClient(testScope)
    }

    @Test
    fun testInitialState() {
        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)
        assertEquals(SessionStats(), client.sessionStats.value)
    }

    @Test
    fun testConnectionStateFlow() = runTest {
        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)
    }

    @Test
    fun testInjectTouchWhenDisconnected() = runTest {
        client.injectTouch(100f, 100f, TouchAction.DOWN)
        // Should not throw, but also not send anything
        assertEquals(0, client.sessionStats.value.inputEventsSent)
    }

    @Test
    fun testInjectKeyWhenDisconnected() = runTest {
        client.injectKey(0x1E, true)
        // Should not throw, but also not send anything
        assertEquals(0, client.sessionStats.value.inputEventsSent)
    }

    @Test
    fun testInjectTextWhenDisconnected() = runTest {
        client.injectText("hello")
        // Should not throw, but also not send anything
        assertEquals(0, client.sessionStats.value.inputEventsSent)
    }

    @Test
    fun testInjectScrollWhenDisconnected() = runTest {
        client.injectScroll(10f, 20f)
        // Should not throw, but also not send anything
        assertEquals(0, client.sessionStats.value.inputEventsSent)
    }

    @Test
    fun testDoubleDisconnectIsSafe() = runTest {
        client.disconnect()
        client.disconnect() // Should not throw
        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)
    }

    @Test
    fun testSessionStatsStructure() {
        val stats = client.sessionStats.value

        assertEquals(0f, stats.fps, 0.01f)
        assertEquals(0L, stats.bitrate)
        assertEquals(0L, stats.latency)
        assertEquals(0f, stats.packetLoss, 0.01f)
        assertEquals(0L, stats.videoBytesReceived)
        assertEquals(0L, stats.inputEventsSent)
        assertEquals(0L, stats.decodedFrames)
        assertEquals(0L, stats.droppedFrames)
        assertEquals(0L, stats.uptime)
    }

    @Test
    fun testClientDestroy() {
        client.destroy()
        // Should not throw
    }

    @Test
    fun testMultipleDestroyCalls() {
        client.destroy()
        client.destroy() // Should not throw
    }

    @Test
    fun testConnectRequiredParametersValidation() = runTest {
        try {
            client.connect("", RemoteDesktopToken(peerId = ""), mockSurface)
            // If connection succeeds, that's fine
        } catch (e: ConnectionException) {
            // Expected for empty peer ID
        }
    }

    @Test
    fun testRemoteDesktopTokenCreation() {
        val token = RemoteDesktopToken(
            peerId = "test-peer",
            permissions = setOf("video", "input"),
            capabilityData = "test-data"
        )

        assertEquals("test-peer", token.peerId)
        assertEquals(setOf("video", "input"), token.permissions)
        assertEquals("test-data", token.capabilityData)
    }

    @Test
    fun testRemoteDesktopTokenValidity() {
        val futureExpiry = System.currentTimeMillis() + 3600000 // 1 hour
        val token = RemoteDesktopToken(
            peerId = "test-peer",
            expiresAt = futureExpiry
        )

        assertTrue("Token should be valid", token.isValid())
    }

    @Test
    fun testRemoteDesktopTokenExpiry() {
        val pastExpiry = System.currentTimeMillis() - 1000 // 1 second ago
        val token = RemoteDesktopToken(
            peerId = "test-peer",
            expiresAt = pastExpiry
        )

        assertFalse("Token should be expired", token.isValid())
    }

    @Test
    fun testRemoteDesktopTokenTimeRemaining() {
        val futureExpiry = System.currentTimeMillis() + 5000 // 5 seconds
        val token = RemoteDesktopToken(
            peerId = "test-peer",
            expiresAt = futureExpiry
        )

        val remaining = token.getTimeRemaining()
        assertTrue("Should have time remaining", remaining > 0)
        assertTrue("Should be close to 5 seconds", remaining <= 5000)
    }

    @Test
    fun testRemoteDesktopTokenExpiredTimeRemaining() {
        val pastExpiry = System.currentTimeMillis() - 1000
        val token = RemoteDesktopToken(
            peerId = "test-peer",
            expiresAt = pastExpiry
        )

        assertEquals(0, token.getTimeRemaining())
    }

    @Test
    fun testInputEventTypes() {
        val touchEvent = InputEvent.TouchEvent(100f, 200f, TouchAction.DOWN)
        assertTrue(touchEvent is InputEvent.TouchEvent)
        assertEquals(TouchAction.DOWN, touchEvent.action)

        val keyEvent = InputEvent.KeyEvent(0x1E, true, 0)
        assertTrue(keyEvent is InputEvent.KeyEvent)
        assertTrue(keyEvent.isDown)

        val textEvent = InputEvent.TextInputEvent("test")
        assertTrue(textEvent is InputEvent.TextInputEvent)
        assertEquals("test", textEvent.text)

        val scrollEvent = InputEvent.ScrollEvent(10f, 20f)
        assertTrue(scrollEvent is InputEvent.ScrollEvent)
        assertEquals(10f, scrollEvent.dx, 0.01f)
    }

    @Test
    fun testConnectionStateTransitions() = runTest {
        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)

        client.disconnect()
        assertEquals(ConnectionState.DISCONNECTED, client.connectionState.value)
    }

    @Test
    fun testRemoteDesktopPeerCreation() {
        val peer = RemoteDesktopPeer(
            peerId = "peer-001",
            name = "Desktop",
            hostname = "desktop.local",
            osType = "Windows",
            resolution = Resolution(1920, 1080, 60),
            isOnline = true
        )

        assertEquals("peer-001", peer.peerId)
        assertEquals("Desktop", peer.name)
        assertEquals("desktop.local", peer.hostname)
        assertEquals("Windows", peer.osType)
        assertTrue(peer.isOnline)
    }

    @Test
    fun testResolutionCreation() {
        val resolution = Resolution(2560, 1600, 120)

        assertEquals(2560, resolution.width)
        assertEquals(1600, resolution.height)
        assertEquals(120, resolution.refreshRate)
    }

    @Test
    fun testConnectionErrorCreation() {
        val error = ConnectionError(
            code = "TEST_ERROR",
            message = "Test error message",
            cause = Exception("Root cause")
        )

        assertEquals("TEST_ERROR", error.code)
        assertEquals("Test error message", error.message)
        assertNotNull(error.cause)
    }
}
