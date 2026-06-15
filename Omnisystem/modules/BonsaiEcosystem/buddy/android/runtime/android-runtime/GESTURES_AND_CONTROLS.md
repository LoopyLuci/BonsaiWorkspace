# Gestures and Controls Reference

Complete guide to all gestures, keyboard shortcuts, and input methods in Bonsai Buddy's remote desktop client.

## Touch Gestures

### Basic Gestures

#### Tap (Single-finger)
- **Action:** Press screen briefly and release
- **Result:** Left-click at tap position
- **Use Cases:** Click buttons, links, menu items
- **Latency:** <5ms

```
[Finger down at x,y]
    ↓ (50ms hold)
[Finger up]
    ↓
Sends: LEFT_CLICK at (x,y)
```

#### Long Press
- **Action:** Press screen and hold for 500ms+
- **Result:** Right-click at press position
- **Use Cases:** Context menus, alternate actions
- **Visual Feedback:** Screen may show visual feedback (desktop-dependent)

```
[Finger down at x,y]
    ↓ (500ms+ hold)
[Finger up or move]
    ↓
Sends: RIGHT_CLICK at (x,y)
```

#### Double Tap
- **Action:** Two quick taps in same location (<300ms apart, <50px distance)
- **Result:** Double-click at tap position
- **Use Cases:** Open files, select items, activate windows
- **Timing:** Must be within 300ms and 50 pixels

```
[Tap 1]
    ↓ (<300ms)
[Tap 2]
    ↓
Sends: DOUBLE_CLICK at position
```

#### Drag (One-finger)
- **Action:** Press, move finger, release
- **Result:** Mouse move + drag (left button held)
- **Use Cases:** Select text, move windows, draw
- **Coordinate Scaling:** Device pixels → desktop pixels automatically

```
[Finger down at (x1,y1)]
    ↓
[Move to (x2,y2)]
    ↓
Sends: MOUSE_MOVE_DOWN events as you drag
    ↓
[Finger up]
    ↓
Sends: MOUSE_UP at final position
```

#### Two-Finger Tap
- **Action:** Tap with two fingers simultaneously
- **Result:** Middle-click at tap position
- **Use Cases:** Paste (X11), alternative action
- **Detection:** Requires both pointers down within 50ms

```
[Finger 1 down]
[Finger 2 down] (<50ms)
    ↓
[Both fingers up]
    ↓
Sends: MIDDLE_CLICK
```

#### Two-Finger Drag (Scroll)
- **Action:** Press two fingers and drag vertically/horizontally
- **Result:** Scroll wheel event
- **Use Cases:** Scroll web pages, lists, documents
- **Sensitivity:** Adjustable in settings

```
[Finger 1 down at (x1,y1)]
[Finger 2 down at (x2,y2)]
    ↓
[Move both fingers up/down by ΔY]
    ↓
Sends: SCROLL_UP or SCROLL_DOWN × (ΔY / 50)
```

#### Pinch (Two-finger)
- **Action:** Two fingers moving closer (pinch in) or apart (pinch out)
- **Result:** Zoom in or out
- **Use Cases:** Zoom on maps, photos, documents
- **Scale Detection:** Ratio > 1.1x triggers zoom

```
[Finger 1 and 2 pressed at distance D1]
    ↓
[Fingers move to distance D2]
    ↓
Scale = D2 / D1
    ↓
If scale > 1.1: ZOOM_OUT
If scale < 0.91: ZOOM_IN
```

### Advanced Gestures

#### Three-Finger Swipe Up
- **Action:** Three fingers on screen, drag upward
- **Result:** Show on-screen keyboard
- **Use Cases:** Text input, password entry
- **Detection:** All three pointers moving predominantly upward

```
[3 fingers down]
    ↓
[All move up together]
    ↓
Show on-screen keyboard overlay
```

#### Three-Finger Swipe Down
- **Action:** Three fingers on screen, drag downward
- **Result:** Hide on-screen keyboard
- **Use Cases:** Maximize screen space
- **Detection:** All three pointers moving predominantly downward

```
[3 fingers down]
    ↓
[All move down together]
    ↓
Hide on-screen keyboard overlay
```

## Keyboard Input

### On-Screen Keyboard

Located at bottom of screen when toggled visible. Can be dismissed by:
1. Tapping the X button (top-right)
2. Three-finger swipe down
3. Tapping the keyboard button in toolbar again

#### Layout

```
┌─ Function Row ─────────────────────────────────────────┐
│ F1  F2  F3  F4  F5  F6  F7  F8  F9  F10 F11 F12       │
├─ Number Row ──────────────────────────────────────────┐
│ 1!  2@  3#  4$  5%  6^  7&  8*  9(  0)  -_  =+       │
├─ QWERTY Row ──────────────────────────────────────────┐
│ Q   W   E   R   T   Y   U   I   O   P   [{  ]}       │
├─ ASDF Row ────────────────────────────────────────────┐
│ A   S   D   F   G   H   J   K   L   ;:  '"             │
├─ ZXCV Row ────────────────────────────────────────────┐
│ Z   X   C   V   B   N   M   ,<  .>  /?               │
├─ Control Row ─────────────────────────────────────────┐
│ Esc Ctrl Alt Shift Win Enter         Space           │
├─ Arrow Row ───────────────────────────────────────────┐
│     ←  ↓  →  | Home End PgUp PgDn  Tab  Del          │
└────────────────────────────────────────────────────────┘
```

#### Modifier Keys

**Ctrl (Control)**
- Tap to toggle on/off (button highlights when active)
- Next key press will include Ctrl modifier
- Auto-deactivates after key press (one-shot mode)

**Alt (Alternate)**
- Same behavior as Ctrl
- For Alt+Tab window switching

**Shift**
- Tap to toggle uppercase mode
- All letters will be capitalized until tapped again
- Number row shows symbols when Shift is active

**Win (Super/Meta)**
- Tap to send Windows/Meta key
- Used for Windows menu, command key, etc.

#### Text Input Methods

1. **Direct typing:** Tap letters on on-screen keyboard
2. **Shift for uppercase:** Tap Shift, then tap letter
3. **Symbols:** Tap Shift for number row symbols
4. **Copy-paste:** Use system clipboard (future feature)

### Physical Keyboard Integration

If a physical keyboard is connected via Bluetooth:

1. Physical key presses automatically map to Linux keycodes
2. Modifier keys (Ctrl, Alt, Shift) are tracked
3. On-screen keyboard still available for function/special keys
4. Hardware keyboard takes priority (no on-screen key repeat)

## Toolbar Controls

Floating toolbar at bottom of screen with 4 buttons:

```
[⌨] [Mouse Mode] [✕]
 │        │         └─ Disconnect
 │        └─ Toggle: Absolute ↔ Relative
 └─ Toggle: Show/Hide on-screen keyboard
```

### Keyboard Button (⌨)
- **Tap:** Toggle on-screen keyboard visibility
- **State:** Highlighted when keyboard is visible
- **Visual:** Keyboard icon

### Mouse Mode Button
- **Tap:** Toggle between Absolute and Relative modes
- **Absolute:** Coordinates map directly to desktop coordinates
  - Touch at (540, 1000) on phone → (960, 1000) on desktop
  - Better for precise clicking
- **Relative:** Movement deltas from previous position
  - More like traditional mouse movement
  - Better for large screens or gaming

### Disconnect Button (✕)
- **Tap:** Disconnect from remote desktop and go back
- **Confirmation:** Immediate disconnect
- **State:** Always red/error color

## Modifier Key Combinations

### Common Shortcuts

| Combination | Windows | macOS | Linux | Result |
|---|---|---|---|---|
| **Ctrl+C** | Copy | Copy | Copy | Copy selection |
| **Ctrl+V** | Paste | Cmd+V | Paste | Paste clipboard |
| **Ctrl+X** | Cut | Cmd+X | Cut | Cut selection |
| **Ctrl+Z** | Undo | Cmd+Z | Undo | Undo last action |
| **Ctrl+A** | Select All | Cmd+A | Select All | Select all |
| **Alt+Tab** | Switch window | Cmd+Tab | Switch window | Switch active window |
| **Ctrl+Alt+Del** | Lock/Task Manager | N/A | Terminal | Special action |
| **Win+D** | Show Desktop | Cmd+F3 | N/A | Show desktop |
| **Shift+Tab** | Reverse Tab | Shift+Tab | Reverse Tab | Previous field |

### How to Type Combinations on On-Screen Keyboard

1. **Tap Ctrl** (button highlights)
2. **Tap C** (sends Ctrl+C)
3. Ctrl automatically deactivates

For three-key combos (Ctrl+Alt+Del):
1. **Tap Ctrl** (highlights)
2. **Tap Alt** (both highlight)
3. **Tap Del** (sends Ctrl+Alt+Del)
4. Both automatically deactivate

## Connection Bar Information

Located at top of screen during active connection:

```
┌─────────────────────────────────────────────────────┐
│ Desktop 1          ▪ Connected  │ 60 fps  2000 kbps │
│ (peer name)        (status)     │ (FPS)   (bitrate) │
│                                 │ 5ms  0.2% loss    │
│                                 │(latency)(packet)  │
└─────────────────────────────────────────────────────┘
```

### Status Indicators

- **Green dot + "Connected":** Active connection, streaming video
- **Yellow + "Connecting...":** Establishing connection
- **Red + "Error":** Connection failed, check error message
- **Gray + "Disconnected":** Session ended

### Performance Metrics

- **FPS:** Frames decoded per second (30-120)
- **Kbps:** Kilobits per second network usage
- **ms:** Decode latency in milliseconds (<10 typical)
- **% loss:** Percentage of frames dropped

## Accessibility

### Voice Control (Future)
Will support voice commands for:
- "Show keyboard"
- "Click at [position]"
- "Type [text]"

### High Contrast Mode
On-screen keyboard supports high contrast theme:
- Settings → Display → High Contrast

### Screen Reader Support
Toolbar buttons include contentDescription for TalkBack:
- Keyboard button: "Toggle on-screen keyboard"
- Mouse mode button: "[Absolute/Relative] mode"
- Disconnect button: "Disconnect from remote desktop"

## Troubleshooting Gestures

### Gesture Not Recognized

**Issue:** Tap not registering as click
**Solutions:**
- Ensure clean screen (no fingerprints affecting accuracy)
- Tap duration should be 50-300ms (not too fast, not too long)
- Make sure you're within touch bounds (not off-screen)

**Issue:** Drag not working smoothly
**Solutions:**
- Check that "Absolute" mode is enabled (for most use cases)
- Try switching to "Relative" mode for mouse-like behavior
- Ensure video connection is stable (check FPS/latency)

**Issue:** Keyboard shortcuts not working
**Solutions:**
- Verify modifier key is highlighted (active)
- Check that on-screen keyboard is visible
- Try physical keyboard if available
- Some Windows shortcuts may be intercepted by Android

### Performance Issues

If gestures feel laggy:

1. **Check connection stats** in top bar
   - FPS should be >30
   - Latency should be <50ms
   - Packet loss should be <5%

2. **Reduce resolution** on desktop
   - Lower bitrate reduces decoding latency

3. **Use absolute mouse mode** (not relative)
   - More direct coordinate mapping

4. **Close other apps** on phone
   - Free up CPU for decoding

## Customization

Via Settings (future):
- Gesture sensitivity (tap time, distance thresholds)
- Keyboard layout (QWERTY, DVORAK, etc.)
- Modifier key behavior (one-shot vs toggle)
- Gesture hotkeys (custom actions for gestures)
- Touch feedback (haptic, visual, audio)
