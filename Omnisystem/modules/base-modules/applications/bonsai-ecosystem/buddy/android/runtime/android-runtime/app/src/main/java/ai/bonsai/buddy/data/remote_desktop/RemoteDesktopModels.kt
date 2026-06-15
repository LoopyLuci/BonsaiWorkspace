package ai.bonsai.buddy.data.remote_desktop

import kotlinx.serialization.Serializable
import java.util.UUID

/**
 * Capability token for remote desktop access.
 * Grants permission to connect to a specific peer desktop.
 */
@Serializable
data class RemoteDesktopToken(
    val tokenId: String = UUID.randomUUID().toString(),
    val peerId: String,
    val permissions: Set<String> = setOf("video", "input"),
    val issuedAt: Long = System.currentTimeMillis(),
    val expiresAt: Long = System.currentTimeMillis() + (24 * 60 * 60 * 1000), // 24 hours
    val capabilityData: String = "" // Base64-encoded capability token from daemon
)

/**
 * Information about a remote desktop peer (available desktop).
 */
@Serializable
data class RemoteDesktopPeer(
    val peerId: String,
    val name: String,
    val hostname: String?,
    val osType: String?,
    val capabilities: Set<String> = setOf("video", "input", "clipboard"),
    val resolution: Resolution? = null,
    val isOnline: Boolean = false,
    val lastSeen: Long = 0
)

@Serializable
data class Resolution(
    val width: Int,
    val height: Int,
    val refreshRate: Int = 60
)

/**
 * Session statistics for remote desktop connection.
 */
data class SessionStats(
    val fps: Float = 0f,
    val bitrate: Long = 0L, // bits per second
    val latency: Long = 0L, // milliseconds
    val packetLoss: Float = 0f, // 0-100%
    val videoBytesReceived: Long = 0L,
    val inputEventsSent: Long = 0L,
    val decodedFrames: Long = 0L,
    val droppedFrames: Long = 0L,
    val uptime: Long = 0L, // milliseconds since session start
    val resolution: Resolution? = null,
    val codec: String = "H.264" // H.264 or H.265
)

/**
 * Input event types for remote desktop.
 */
sealed class InputEvent {
    data class TouchEvent(
        val x: Float,
        val y: Float,
        val action: TouchAction,
        val pressure: Float = 1.0f
    ) : InputEvent()

    data class KeyEvent(
        val keycode: Int,
        val isDown: Boolean,
        val modifiers: Int = 0 // Ctrl, Alt, Shift flags
    ) : InputEvent()

    data class TextInputEvent(
        val text: String
    ) : InputEvent()

    data class ScrollEvent(
        val dx: Float,
        val dy: Float
    ) : InputEvent()

    data class MouseMoveEvent(
        val x: Float,
        val y: Float
    ) : InputEvent()
}

enum class TouchAction {
    DOWN,
    MOVE,
    UP,
    CANCEL
}

/**
 * Gestures detected by the UI layer.
 */
sealed class GestureEvent {
    data class Tap(val x: Float, val y: Float) : GestureEvent()
    data class LongPress(val x: Float, val y: Float) : GestureEvent()
    data class DoubleTap(val x: Float, val y: Float) : GestureEvent()
    data class TwoFingerTap(val x1: Float, val y1: Float, val x2: Float, val y2: Float) : GestureEvent()
    data class Drag(val startX: Float, val startY: Float, val endX: Float, val endY: Float) : GestureEvent()
    data class TwoFingerDrag(val dx: Float, val dy: Float) : GestureEvent()
    data class Pinch(val scale: Float, val centerX: Float, val centerY: Float) : GestureEvent()
    data class ThreeFingerSwipeUp(val x: Float, val y: Float) : GestureEvent()
    data class ThreeFingerSwipeDown(val x: Float, val y: Float) : GestureEvent()
}

/**
 * Connection state for remote desktop session.
 */
enum class ConnectionState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    RECONNECTING,
    ERROR,
    SUSPENDED
}

data class ConnectionError(
    val code: String,
    val message: String,
    val cause: Throwable? = null
)
