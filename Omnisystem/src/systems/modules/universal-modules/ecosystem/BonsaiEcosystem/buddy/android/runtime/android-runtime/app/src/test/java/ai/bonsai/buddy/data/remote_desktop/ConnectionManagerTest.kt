package ai.bonsai.buddy.data.remote_desktop

import android.content.Context
import android.content.SharedPreferences
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.Mockito.*

/**
 * Unit tests for ConnectionManager.
 *
 * Tests peer discovery, device pairing, token generation, and QR code handling.
 */
class ConnectionManagerTest {
    private lateinit var manager: ConnectionManager
    private lateinit var mockContext: Context
    private lateinit var mockPrefs: SharedPreferences
    private lateinit var mockEditor: SharedPreferences.Editor

    @Before
    fun setup() {
        mockContext = mock(Context::class.java)
        mockPrefs = mock(SharedPreferences::class.java)
        mockEditor = mock(SharedPreferences.Editor::class.java)

        `when`(mockContext.getSharedPreferences("remote_desktop_prefs", Context.MODE_PRIVATE))
            .thenReturn(mockPrefs)
        `when`(mockPrefs.edit()).thenReturn(mockEditor)
        `when`(mockEditor.putString(anyString(), anyString())).thenReturn(mockEditor)

        manager = ConnectionManager(mockContext)
    }

    @Test
    fun testInitialState() {
        assertTrue(manager.availablePeers.value.isEmpty())
        assertTrue(manager.pairedDevices.value.isEmpty())
        assertFalse(manager.isDiscovering.value)
    }

    @Test
    fun testGenerateToken() = runBlocking {
        val token = manager.generateToken("test-peer")

        assertEquals("test-peer", token.peerId)
        assertTrue(token.isValid())
        assertTrue(token.getTimeRemaining() > 0)
        assertTrue(token.permissions.contains("video"))
        assertTrue(token.permissions.contains("input"))
    }

    @Test
    fun testGenerateTokenCustomPermissions() = runBlocking {
        val permissions = setOf("video", "clipboard")
        val token = manager.generateToken("test-peer", permissions)

        assertEquals(permissions, token.permissions)
    }

    @Test
    fun testGenerateTokenCustomExpiry() = runBlocking {
        val expiresIn = 60 * 60 * 1000 // 1 hour
        val token = manager.generateToken("test-peer", expiresIn = expiresIn)

        assertTrue(token.isValid())
        val remaining = token.getTimeRemaining()
        assertTrue("Time remaining should be close to 1 hour", remaining in (3600000 - 5000)..3600000)
    }

    @Test
    fun testPairDevice() = runBlocking {
        val device = RemoteDesktopPeer(
            peerId = "peer-001",
            name = "Desktop",
            hostname = "desktop.local",
            isOnline = true
        )

        manager.pairDevice(device)

        assertTrue(manager.pairedDevices.value.any { it.peerId == "peer-001" })
    }

    @Test
    fun testUnpairDevice() = runBlocking {
        val device = RemoteDesktopPeer(
            peerId = "peer-001",
            name = "Desktop",
            isOnline = true
        )

        manager.pairDevice(device)
        assertTrue(manager.pairedDevices.value.any { it.peerId == "peer-001" })

        manager.unpairDevice("peer-001")
        assertFalse(manager.pairedDevices.value.any { it.peerId == "peer-001" })
    }

    @Test
    fun testPairMultipleDevices() = runBlocking {
        val device1 = RemoteDesktopPeer(peerId = "peer-001", name = "Desktop 1", isOnline = true)
        val device2 = RemoteDesktopPeer(peerId = "peer-002", name = "Desktop 2", isOnline = true)

        manager.pairDevice(device1)
        manager.pairDevice(device2)

        assertEquals(2, manager.pairedDevices.value.size)
    }

    @Test
    fun testNoDuplicatePairing() = runBlocking {
        val device = RemoteDesktopPeer(peerId = "peer-001", name = "Desktop", isOnline = true)

        manager.pairDevice(device)
        manager.pairDevice(device) // Try to pair again

        assertEquals(1, manager.pairedDevices.value.size)
    }

    @Test
    fun testGetPeerInfo() = runBlocking {
        manager.startDiscovery()

        val peerInfo = manager.getPeerInfo("peer-001")
        // Should find in discovery results
        assertNotNull(peerInfo)
    }

    @Test
    fun testGetPeerInfoFromPaired() = runBlocking {
        val device = RemoteDesktopPeer(peerId = "peer-001", name = "Desktop", isOnline = true)
        manager.pairDevice(device)

        val peerInfo = manager.getPeerInfo("peer-001")
        assertNotNull(peerInfo)
        assertEquals("Desktop", peerInfo?.name)
    }

    @Test
    fun testGetNonexistentPeerInfo() = runBlocking {
        val peerInfo = manager.getPeerInfo("nonexistent")
        assertNull(peerInfo)
    }

    @Test
    fun testGeneratePairingQrCode() {
        val device = RemoteDesktopPeer(
            peerId = "peer-001",
            name = "Desktop",
            hostname = "desktop.local",
            isOnline = true
        )

        val qrCode = manager.generatePairingQrCode(device)

        assertTrue(qrCode.startsWith("brdf://peer/"))
        assertTrue(qrCode.contains("peer-001"))
    }

    @Test
    fun testParsePairingQrCode() = runBlocking {
        val qrCode = "brdf://peer/peer-001/desktop.local"

        val parsed = manager.parsePairingQrCode(qrCode)

        assertNotNull(parsed)
        assertEquals("peer-001", parsed?.peerId)
        assertEquals("desktop.local", parsed?.hostname)
    }

    @Test
    fun testParsePairingQrCodeInvalid() = runBlocking {
        val invalidCode = "invalid://code"

        val parsed = manager.parsePairingQrCode(invalidCode)

        assertNull(parsed)
    }

    @Test
    fun testRecordPeerUsage() = runBlocking {
        manager.recordPeerUsage("peer-001")

        verify(mockEditor).putString("last_peer_id", "peer-001")
    }

    @Test
    fun testGetLastUsedDevice() = runBlocking {
        val device = RemoteDesktopPeer(peerId = "peer-001", name = "Desktop", isOnline = true)
        manager.pairDevice(device)
        manager.recordPeerUsage("peer-001")

        val lastUsed = manager.getLastUsedDevice()

        assertNotNull(lastUsed)
        assertEquals("peer-001", lastUsed?.peerId)
    }

    @Test
    fun testGetLastUsedDeviceNoHistory() {
        `when`(mockPrefs.getString("last_peer_id", null)).thenReturn(null)

        val lastUsed = manager.getLastUsedDevice()

        assertNull(lastUsed)
    }

    @Test
    fun testStartDiscovery() = runBlocking {
        assertFalse(manager.isDiscovering.value)

        manager.startDiscovery()

        assertTrue(manager.isDiscovering.value)
    }

    @Test
    fun testStopDiscovery() = runBlocking {
        manager.startDiscovery()
        assertTrue(manager.isDiscovering.value)

        manager.stopDiscovery()
        assertFalse(manager.isDiscovering.value)
    }

    @Test
    fun testDoubleStartDiscovery() = runBlocking {
        manager.startDiscovery()
        assertTrue(manager.isDiscovering.value)

        manager.startDiscovery() // Should not fail
        assertTrue(manager.isDiscovering.value)
    }

    @Test
    fun testTokenValidityCheck() {
        val futureToken = RemoteDesktopToken(
            peerId = "test",
            expiresAt = System.currentTimeMillis() + 3600000
        )
        assertTrue(futureToken.isValid())

        val pastToken = RemoteDesktopToken(
            peerId = "test",
            expiresAt = System.currentTimeMillis() - 1000
        )
        assertFalse(pastToken.isValid())
    }

    @Test
    fun testTokenTimeRemaining() {
        val futureToken = RemoteDesktopToken(
            peerId = "test",
            expiresAt = System.currentTimeMillis() + 5000
        )
        val remaining = futureToken.getTimeRemaining()
        assertTrue("Time remaining should be positive", remaining > 0)
        assertTrue("Time remaining should be <= 5000", remaining <= 5000)

        val pastToken = RemoteDesktopToken(
            peerId = "test",
            expiresAt = System.currentTimeMillis() - 1000
        )
        assertEquals(0, pastToken.getTimeRemaining())
    }

    @Test
    fun testQrCodeRoundTrip() = runBlocking {
        val device = RemoteDesktopPeer(
            peerId = "peer-001",
            name = "Desktop",
            hostname = "desktop.local",
            isOnline = true
        )

        val qrCode = manager.generatePairingQrCode(device)
        val parsed = manager.parsePairingQrCode(qrCode)

        assertNotNull(parsed)
        assertEquals(device.peerId, parsed?.peerId)
        assertEquals(device.hostname, parsed?.hostname)
    }
}
