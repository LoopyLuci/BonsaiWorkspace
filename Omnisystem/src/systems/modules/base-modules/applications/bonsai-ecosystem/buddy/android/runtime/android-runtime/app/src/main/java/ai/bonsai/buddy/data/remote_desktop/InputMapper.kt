package ai.bonsai.buddy.data.remote_desktop

import android.view.KeyEvent
import android.view.MotionEvent
import kotlin.math.atan2
import kotlin.math.hypot

/**
 * Converts Android pointer/touch events and gestures into BRDF InputEvents.
 *
 * Handles:
 * - Touch coordinate mapping (device -> desktop resolution)
 * - Modifier key tracking (Ctrl, Alt, Shift)
 * - Mouse mode switching (absolute vs relative)
 * - Gesture recognition (tap, drag, pinch, scroll)
 */
class InputMapper(
    private val desktopWidth: Int,
    private val desktopHeight: Int,
    private val deviceWidth: Int,
    private val deviceHeight: Int
) {
    private var mouseMode = MouseMode.ABSOLUTE
    private var lastX = 0f
    private var lastY = 0f
    private var isMouseDown = false

    private var ctrlPressed = false
    private var altPressed = false
    private var shiftPressed = false
    private var metaPressed = false

    // Gesture state
    private var lastTapTime = 0L
    private var lastTapX = 0f
    private var lastTapY = 0f
    private var pointerCount = 0
    private var lastPointerDistance = 0f

    enum class MouseMode {
        ABSOLUTE, // Report absolute coordinates
        RELATIVE  // Report relative movement deltas
    }

    /**
     * Map Android MotionEvent to InputEvent.
     * Handles both single-touch and multi-touch gestures.
     */
    fun mapTouchEvent(event: MotionEvent): InputEvent? {
        val action = event.actionMasked
        val x = event.x
        val y = event.y
        pointerCount = event.pointerCount

        return when {
            pointerCount >= 3 -> handleMultiTouchGesture(event)
            pointerCount == 2 -> handleTwoFingerGesture(event)
            else -> handleSingleTouchGesture(action, x, y)
        }
    }

    /**
     * Map Android KeyEvent to InputEvent.
     * Tracks modifier keys and maps hardware keycodes to logical events.
     */
    fun mapKeyEvent(event: KeyEvent): InputEvent? {
        val action = event.action
        val isDown = action == KeyEvent.ACTION_DOWN

        // Update modifier state
        when (event.keyCode) {
            KeyEvent.KEYCODE_CTRL_LEFT, KeyEvent.KEYCODE_CTRL_RIGHT -> {
                ctrlPressed = isDown
                return null // Don't send modifier-only events
            }
            KeyEvent.KEYCODE_ALT_LEFT, KeyEvent.KEYCODE_ALT_RIGHT -> {
                altPressed = isDown
                return null
            }
            KeyEvent.KEYCODE_SHIFT_LEFT, KeyEvent.KEYCODE_SHIFT_RIGHT -> {
                shiftPressed = isDown
                return null
            }
            KeyEvent.KEYCODE_META_LEFT, KeyEvent.KEYCODE_META_RIGHT -> {
                metaPressed = isDown
                return null
            }
        }

        // Map hardware keycode
        val mappedKeycode = mapAndroidKeycode(event.keyCode)
        if (mappedKeycode < 0) {
            return null // Unsupported key
        }

        val modifiers = buildModifierFlags()
        return InputEvent.KeyEvent(mappedKeycode, isDown, modifiers)
    }

    /**
     * Inject text directly (for on-screen keyboard input).
     */
    fun mapTextInput(text: String): InputEvent {
        return InputEvent.TextInputEvent(text)
    }

    /**
     * Switch between absolute and relative mouse modes.
     */
    fun setMouseMode(mode: MouseMode) {
        mouseMode = mode
    }

    /**
     * Get current mouse mode.
     */
    fun getMouseMode(): MouseMode = mouseMode

    /**
     * Adjust for desktop resolution. Call when desktop resolution changes.
     */
    fun updateDesktopResolution(width: Int, height: Int) {
        // Can be used for dynamic resolution adaptation
    }

    private fun handleSingleTouchGesture(action: Int, x: Float, y: Float): InputEvent? {
        val (desktopX, desktopY) = mapCoordinates(x, y)

        return when (action) {
            MotionEvent.ACTION_DOWN -> {
                isMouseDown = true
                lastX = x
                lastY = y

                // Check for double-tap
                val now = System.currentTimeMillis()
                if (now - lastTapTime < DOUBLE_TAP_TIMEOUT && distance(x, y, lastTapX, lastTapY) < DOUBLE_TAP_DISTANCE) {
                    lastTapTime = 0 // Clear double-tap state
                    // Double-tap = double-click
                    InputEvent.TouchEvent(desktopX, desktopY, TouchAction.DOWN, 1.0f)
                } else {
                    lastTapTime = now
                    lastTapX = x
                    lastTapY = y
                    InputEvent.TouchEvent(desktopX, desktopY, TouchAction.DOWN, 1.0f)
                }
            }
            MotionEvent.ACTION_MOVE -> {
                if (!isMouseDown) {
                    return null
                }

                if (mouseMode == MouseMode.RELATIVE) {
                    val dx = x - lastX
                    val dy = y - lastY
                    lastX = x
                    lastY = y
                    if (dx != 0f || dy != 0f) {
                        InputEvent.MouseMoveEvent(dx, dy)
                    } else {
                        null
                    }
                } else {
                    lastX = x
                    lastY = y
                    InputEvent.TouchEvent(desktopX, desktopY, TouchAction.MOVE, 1.0f)
                }
            }
            MotionEvent.ACTION_UP -> {
                isMouseDown = false
                InputEvent.TouchEvent(desktopX, desktopY, TouchAction.UP, 1.0f)
            }
            MotionEvent.ACTION_CANCEL -> {
                isMouseDown = false
                InputEvent.TouchEvent(desktopX, desktopY, TouchAction.CANCEL, 0f)
            }
            else -> null
        }
    }

    private fun handleTwoFingerGesture(event: MotionEvent): InputEvent? {
        val pointer1 = MotionEvent.PointerCoords()
        val pointer2 = MotionEvent.PointerCoords()
        event.getPointerCoords(0, pointer1)
        event.getPointerCoords(1, pointer2)

        val currentDistance = distance(pointer1.x, pointer1.y, pointer2.x, pointer2.y)

        return when (event.actionMasked) {
            MotionEvent.ACTION_POINTER_DOWN -> {
                lastPointerDistance = currentDistance
                null
            }
            MotionEvent.ACTION_MOVE -> {
                if (lastPointerDistance > 0) {
                    val scale = currentDistance / lastPointerDistance
                    val centerX = (pointer1.x + pointer2.x) / 2
                    val centerY = (pointer1.y + pointer2.y) / 2
                    lastPointerDistance = currentDistance

                    // Pinch gesture
                    if (scale > PINCH_THRESHOLD || scale < (1f / PINCH_THRESHOLD)) {
                        val (desktopCenterX, desktopCenterY) = mapCoordinates(centerX, centerY)
                        InputEvent.TouchEvent(desktopCenterX, desktopCenterY, TouchAction.UP, 1.0f) // End move
                    } else {
                        // Two-finger scroll
                        val (p1x, p1y) = mapCoordinates(pointer1.x, pointer1.y)
                        val (p2x, p2y) = mapCoordinates(pointer2.x, pointer2.y)
                        val dx = (p2x - p1x) * 0.1f
                        val dy = (p2y - p1y) * 0.1f
                        InputEvent.ScrollEvent(dx, dy)
                    }
                } else {
                    null
                }
            }
            else -> null
        }
    }

    private fun handleMultiTouchGesture(event: MotionEvent): InputEvent? {
        // 3+ finger swipes for keyboard show/hide
        if (event.actionMasked == MotionEvent.ACTION_MOVE && event.pointerCount >= 3) {
            // Detect vertical swipe direction by looking at center of all pointers
            var sumY = 0f
            for (i in 0 until event.pointerCount) {
                sumY += event.getY(i)
            }
            val centerY = sumY / event.pointerCount

            // Simple heuristic: if most pointers are moving up, it's a swipe up
            val allPointersUp = (0 until event.pointerCount).all { i ->
                event.getY(i) < centerY
            }

            return if (allPointersUp) {
                InputEvent.KeyEvent(KEYCODE_SHOW_KEYBOARD, true)
            } else {
                InputEvent.KeyEvent(KEYCODE_HIDE_KEYBOARD, true)
            }
        }
        return null
    }

    private fun mapCoordinates(x: Float, y: Float): Pair<Float, Float> {
        // Scale from device coordinates to desktop coordinates
        val scaledX = (x / deviceWidth) * desktopWidth
        val scaledY = (y / deviceHeight) * desktopHeight
        return Pair(scaledX, scaledY)
    }

    private fun mapAndroidKeycode(androidKeycode: Int): Int {
        // Map Android keycodes to Linux input event codes (for consistency with BRDF)
        return when (androidKeycode) {
            KeyEvent.KEYCODE_0 -> 0x0B
            KeyEvent.KEYCODE_1 -> 0x02
            KeyEvent.KEYCODE_2 -> 0x03
            KeyEvent.KEYCODE_3 -> 0x04
            KeyEvent.KEYCODE_4 -> 0x05
            KeyEvent.KEYCODE_5 -> 0x06
            KeyEvent.KEYCODE_6 -> 0x07
            KeyEvent.KEYCODE_7 -> 0x08
            KeyEvent.KEYCODE_8 -> 0x09
            KeyEvent.KEYCODE_9 -> 0x0A
            KeyEvent.KEYCODE_A -> 0x1E
            KeyEvent.KEYCODE_B -> 0x30
            KeyEvent.KEYCODE_C -> 0x2E
            KeyEvent.KEYCODE_D -> 0x20
            KeyEvent.KEYCODE_E -> 0x12
            KeyEvent.KEYCODE_F -> 0x21
            KeyEvent.KEYCODE_G -> 0x22
            KeyEvent.KEYCODE_H -> 0x23
            KeyEvent.KEYCODE_I -> 0x17
            KeyEvent.KEYCODE_J -> 0x24
            KeyEvent.KEYCODE_K -> 0x25
            KeyEvent.KEYCODE_L -> 0x26
            KeyEvent.KEYCODE_M -> 0x32
            KeyEvent.KEYCODE_N -> 0x31
            KeyEvent.KEYCODE_O -> 0x18
            KeyEvent.KEYCODE_P -> 0x19
            KeyEvent.KEYCODE_Q -> 0x10
            KeyEvent.KEYCODE_R -> 0x13
            KeyEvent.KEYCODE_S -> 0x1F
            KeyEvent.KEYCODE_T -> 0x14
            KeyEvent.KEYCODE_U -> 0x16
            KeyEvent.KEYCODE_V -> 0x2F
            KeyEvent.KEYCODE_W -> 0x11
            KeyEvent.KEYCODE_X -> 0x2D
            KeyEvent.KEYCODE_Y -> 0x15
            KeyEvent.KEYCODE_Z -> 0x2C
            KeyEvent.KEYCODE_SPACE -> 0x39
            KeyEvent.KEYCODE_ENTER -> 0x1C
            KeyEvent.KEYCODE_DEL -> 0x0E
            KeyEvent.KEYCODE_TAB -> 0x0F
            KeyEvent.KEYCODE_ESCAPE -> 0x01
            KeyEvent.KEYCODE_F1 -> 0x3B
            KeyEvent.KEYCODE_F2 -> 0x3C
            KeyEvent.KEYCODE_F3 -> 0x3D
            KeyEvent.KEYCODE_F4 -> 0x3E
            KeyEvent.KEYCODE_F5 -> 0x3F
            KeyEvent.KEYCODE_F6 -> 0x40
            KeyEvent.KEYCODE_F7 -> 0x41
            KeyEvent.KEYCODE_F8 -> 0x42
            KeyEvent.KEYCODE_F9 -> 0x43
            KeyEvent.KEYCODE_F10 -> 0x44
            KeyEvent.KEYCODE_F11 -> 0x57
            KeyEvent.KEYCODE_F12 -> 0x58
            KeyEvent.KEYCODE_HOME -> 0x47
            KeyEvent.KEYCODE_MOVE_END -> 0x4F
            KeyEvent.KEYCODE_PAGE_UP -> 0x49
            KeyEvent.KEYCODE_PAGE_DOWN -> 0x51
            KeyEvent.KEYCODE_DPAD_LEFT -> 0x4B
            KeyEvent.KEYCODE_DPAD_RIGHT -> 0x4D
            KeyEvent.KEYCODE_DPAD_UP -> 0x48
            KeyEvent.KEYCODE_DPAD_DOWN -> 0x50
            else -> -1 // Unsupported
        }
    }

    private fun buildModifierFlags(): Int {
        var flags = 0
        if (ctrlPressed) flags = flags or MODIFIER_CTRL
        if (altPressed) flags = flags or MODIFIER_ALT
        if (shiftPressed) flags = flags or MODIFIER_SHIFT
        if (metaPressed) flags = flags or MODIFIER_META
        return flags
    }

    private fun distance(x1: Float, y1: Float, x2: Float, y2: Float): Float {
        return hypot(x1 - x2, y1 - y2)
    }

    companion object {
        private const val DOUBLE_TAP_TIMEOUT = 300L
        private const val DOUBLE_TAP_DISTANCE = 50f
        private const val PINCH_THRESHOLD = 1.1f

        // Virtual keycodes for keyboard show/hide
        const val KEYCODE_SHOW_KEYBOARD = 0xF001
        const val KEYCODE_HIDE_KEYBOARD = 0xF002

        // Modifier flags
        const val MODIFIER_CTRL = 1 shl 0
        const val MODIFIER_ALT = 1 shl 1
        const val MODIFIER_SHIFT = 1 shl 2
        const val MODIFIER_META = 1 shl 3
    }
}
