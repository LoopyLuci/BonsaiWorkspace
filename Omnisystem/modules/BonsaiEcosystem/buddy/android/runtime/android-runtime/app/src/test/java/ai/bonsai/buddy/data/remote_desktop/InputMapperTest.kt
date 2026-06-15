package ai.bonsai.buddy.data.remote_desktop

import android.view.KeyEvent
import android.view.MotionEvent
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.mockito.Mockito.*

/**
 * Unit tests for InputMapper.
 *
 * Tests coordinate transformation, gesture recognition, and keycode mapping.
 */
class InputMapperTest {
    private lateinit var mapper: InputMapper

    @Before
    fun setup() {
        // Desktop 1920x1080, Device 1080x2400 (typical phone)
        mapper = InputMapper(1920, 1080, 1080, 2400)
    }

    @Test
    fun testCoordinateMapping() {
        // Test that device coordinates are scaled to desktop coordinates
        val event = mock(MotionEvent::class.java)
        `when`(event.x).thenReturn(540f) // Middle of device width
        `when`(event.y).thenReturn(1200f) // Middle of device height
        `when`(event.actionMasked).thenReturn(MotionEvent.ACTION_DOWN)
        `when`(event.pointerCount).thenReturn(1)

        val inputEvent = mapper.mapTouchEvent(event)
        assertNotNull(inputEvent)

        val touchEvent = inputEvent as InputEvent.TouchEvent
        assertEquals(960f, touchEvent.x, 1f) // Middle of desktop width
        assertEquals(540f, touchEvent.y, 1f) // Middle of desktop height
    }

    @Test
    fun testTouchDownEvent() {
        val event = mock(MotionEvent::class.java)
        `when`(event.x).thenReturn(100f)
        `when`(event.y).thenReturn(200f)
        `when`(event.actionMasked).thenReturn(MotionEvent.ACTION_DOWN)
        `when`(event.pointerCount).thenReturn(1)

        val inputEvent = mapper.mapTouchEvent(event)
        assertTrue(inputEvent is InputEvent.TouchEvent)

        val touchEvent = inputEvent as InputEvent.TouchEvent
        assertEquals(TouchAction.DOWN, touchEvent.action)
        assertEquals(1.0f, touchEvent.pressure, 0.01f)
    }

    @Test
    fun testTouchMoveEvent() {
        val downEvent = mock(MotionEvent::class.java)
        `when`(downEvent.x).thenReturn(100f)
        `when`(downEvent.y).thenReturn(200f)
        `when`(downEvent.actionMasked).thenReturn(MotionEvent.ACTION_DOWN)
        `when`(downEvent.pointerCount).thenReturn(1)

        mapper.mapTouchEvent(downEvent)

        val moveEvent = mock(MotionEvent::class.java)
        `when`(moveEvent.x).thenReturn(150f)
        `when`(moveEvent.y).thenReturn(250f)
        `when`(moveEvent.actionMasked).thenReturn(MotionEvent.ACTION_MOVE)
        `when`(moveEvent.pointerCount).thenReturn(1)

        val inputEvent = mapper.mapTouchEvent(moveEvent)
        assertTrue(inputEvent is InputEvent.TouchEvent)

        val touchEvent = inputEvent as InputEvent.TouchEvent
        assertEquals(TouchAction.MOVE, touchEvent.action)
    }

    @Test
    fun testTouchUpEvent() {
        val downEvent = mock(MotionEvent::class.java)
        `when`(downEvent.x).thenReturn(100f)
        `when`(downEvent.y).thenReturn(200f)
        `when`(downEvent.actionMasked).thenReturn(MotionEvent.ACTION_DOWN)
        `when`(downEvent.pointerCount).thenReturn(1)

        mapper.mapTouchEvent(downEvent)

        val upEvent = mock(MotionEvent::class.java)
        `when`(upEvent.x).thenReturn(100f)
        `when`(upEvent.y).thenReturn(200f)
        `when`(upEvent.actionMasked).thenReturn(MotionEvent.ACTION_UP)
        `when`(upEvent.pointerCount).thenReturn(1)

        val inputEvent = mapper.mapTouchEvent(upEvent)
        assertTrue(inputEvent is InputEvent.TouchEvent)

        val touchEvent = inputEvent as InputEvent.TouchEvent
        assertEquals(TouchAction.UP, touchEvent.action)
    }

    @Test
    fun testKeyEventMapping() {
        val keyEvent = mock(KeyEvent::class.java)
        `when`(keyEvent.keyCode).thenReturn(KeyEvent.KEYCODE_A)
        `when`(keyEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

        val inputEvent = mapper.mapKeyEvent(keyEvent)
        assertTrue(inputEvent is InputEvent.KeyEvent)

        val keyInputEvent = inputEvent as InputEvent.KeyEvent
        assertEquals(true, keyInputEvent.isDown)
        assertEquals(0x1E, keyInputEvent.keycode) // Linux keycode for 'A'
    }

    @Test
    fun testModifierKeyTracking() {
        // Press Ctrl
        val ctrlEvent = mock(KeyEvent::class.java)
        `when`(ctrlEvent.keyCode).thenReturn(KeyEvent.KEYCODE_CTRL_LEFT)
        `when`(ctrlEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

        mapper.mapKeyEvent(ctrlEvent)

        // Press 'A' with Ctrl held
        val aEvent = mock(KeyEvent::class.java)
        `when`(aEvent.keyCode).thenReturn(KeyEvent.KEYCODE_A)
        `when`(aEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

        val inputEvent = mapper.mapKeyEvent(aEvent)
        val keyInputEvent = inputEvent as InputEvent.KeyEvent

        // Should have Ctrl modifier set
        val hasCtrlModifier = (keyInputEvent.modifiers and InputMapper.MODIFIER_CTRL) != 0
        assertTrue("Ctrl modifier not set", hasCtrlModifier)
    }

    @Test
    fun testMouseModeSwitch() {
        assertEquals(InputMapper.MouseMode.ABSOLUTE, mapper.getMouseMode())

        mapper.setMouseMode(InputMapper.MouseMode.RELATIVE)
        assertEquals(InputMapper.MouseMode.RELATIVE, mapper.getMouseMode())

        mapper.setMouseMode(InputMapper.MouseMode.ABSOLUTE)
        assertEquals(InputMapper.MouseMode.ABSOLUTE, mapper.getMouseMode())
    }

    @Test
    fun testTextInput() {
        val textEvent = mapper.mapTextInput("Hello")
        assertTrue(textEvent is InputEvent.TextInputEvent)

        val textInputEvent = textEvent as InputEvent.TextInputEvent
        assertEquals("Hello", textInputEvent.text)
    }

    @Test
    fun testFunctionKeyMapping() {
        for (i in 0..11) {
            val keyEvent = mock(KeyEvent::class.java)
            `when`(keyEvent.keyCode).thenReturn(KeyEvent.KEYCODE_F1 + i)
            `when`(keyEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

            val inputEvent = mapper.mapKeyEvent(keyEvent)
            assertTrue(inputEvent is InputEvent.KeyEvent)

            val keyInputEvent = inputEvent as InputEvent.KeyEvent
            val expectedKeycode = 0x3B + i // F1-F12 keycodes
            assertEquals(expectedKeycode, keyInputEvent.keycode)
        }
    }

    @Test
    fun testArrowKeyMapping() {
        val testCases = listOf(
            KeyEvent.KEYCODE_DPAD_LEFT to 0x4B,
            KeyEvent.KEYCODE_DPAD_RIGHT to 0x4D,
            KeyEvent.KEYCODE_DPAD_UP to 0x48,
            KeyEvent.KEYCODE_DPAD_DOWN to 0x50
        )

        testCases.forEach { (androidKeycode, expectedKeycode) ->
            val keyEvent = mock(KeyEvent::class.java)
            `when`(keyEvent.keyCode).thenReturn(androidKeycode)
            `when`(keyEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

            val inputEvent = mapper.mapKeyEvent(keyEvent)
            val keyInputEvent = inputEvent as InputEvent.KeyEvent
            assertEquals(expectedKeycode, keyInputEvent.keycode)
        }
    }

    @Test
    fun testSpecialKeyMapping() {
        val testCases = listOf(
            KeyEvent.KEYCODE_SPACE to 0x39,
            KeyEvent.KEYCODE_ENTER to 0x1C,
            KeyEvent.KEYCODE_TAB to 0x0F,
            KeyEvent.KEYCODE_ESCAPE to 0x01,
            KeyEvent.KEYCODE_HOME to 0x47,
            KeyEvent.KEYCODE_MOVE_END to 0x4F,
            KeyEvent.KEYCODE_PAGE_UP to 0x49,
            KeyEvent.KEYCODE_PAGE_DOWN to 0x51
        )

        testCases.forEach { (androidKeycode, expectedKeycode) ->
            val keyEvent = mock(KeyEvent::class.java)
            `when`(keyEvent.keyCode).thenReturn(androidKeycode)
            `when`(keyEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

            val inputEvent = mapper.mapKeyEvent(keyEvent)
            val keyInputEvent = inputEvent as InputEvent.KeyEvent
            assertEquals(expectedKeycode, keyInputEvent.keycode)
        }
    }

    @Test
    fun testUnsupportedKeyMapping() {
        val keyEvent = mock(KeyEvent::class.java)
        `when`(keyEvent.keyCode).thenReturn(999) // Invalid keycode
        `when`(keyEvent.action).thenReturn(KeyEvent.ACTION_DOWN)

        val inputEvent = mapper.mapKeyEvent(keyEvent)
        assertNull("Unsupported key should return null", inputEvent)
    }

    @Test
    fun testKeyUpEvent() {
        val keyEvent = mock(KeyEvent::class.java)
        `when`(keyEvent.keyCode).thenReturn(KeyEvent.KEYCODE_A)
        `when`(keyEvent.action).thenReturn(KeyEvent.ACTION_UP)

        val inputEvent = mapper.mapKeyEvent(keyEvent)
        assertTrue(inputEvent is InputEvent.KeyEvent)

        val keyInputEvent = inputEvent as InputEvent.KeyEvent
        assertEquals(false, keyInputEvent.isDown)
    }
}
