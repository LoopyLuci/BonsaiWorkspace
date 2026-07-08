# OmniOS Desktop Window Manager

## Design Goals

- **Real desktop feel**: windows behave exactly like native OS windows — proper z-ordering, snap zones, keyboard navigation, multi-monitor awareness
- **Zero data loss**: all window positions, sizes, and open apps persist across sessions
- **Performance**: all animations run at 60fps using CSS transforms (GPU composited), never layout-triggering properties
- **Accessibility**: full keyboard navigation, screen reader support, reduced-motion support

---

## Z-Order Management

### The Problem with Simple Incrementing
A naive window manager assigns `z-index: ++zTop` on every focus event. This works until `zTop` reaches the browser's integer limit (2^31 - 1), or until it becomes visually confusing after many opens and closes. It also cannot reconstruct a valid z-order from persisted state.

### The Stack Array Solution
Windows are tracked in `zStack: string[]` — an ordered array of app IDs, where index 0 is the bottommost window and the last index is the topmost (focused) window.

```javascript
// Focus window 'terminal'
function bringToFront(appId) {
  zStack = zStack.filter(id => id !== appId);
  zStack.push(appId);
  recomputeZIndexes();
}

// Recompute all z-indexes from the array
function recomputeZIndexes() {
  zStack.forEach((id, index) => {
    const win = document.getElementById('win-' + id);
    if (win) win.style.zIndex = (10 + index * 10).toString();
  });
}
```

Starting at z-index 10, incrementing by 10 for each window (leaves room for sub-elements like resize handles and dropdowns within each window).

---

## Window State

Each window in the `windows` registry:

```typescript
interface WindowState {
  id: string;            // app identifier: 'terminal', 'files', etc.
  title: string;
  icon: string;
  x: number;            // pixels from left of desktop area
  y: number;            // pixels from top of desktop area
  w: number;            // width in pixels
  h: number;            // height in pixels
  minimized: boolean;
  maximized: boolean;
  prevRect: {x,y,w,h} | null;  // rect before maximize, for restore
  snapSide: 'left'|'right'|'topleft'|'topright'|'bottomleft'|'bottomright'|null;
  virtualDesktop: number;  // 0-indexed
}
```

---

## Dragging

Dragging uses `mousemove` on `document` (not the element), so fast mouse movement does not lose tracking.

```javascript
function makeDraggable(win, titlebar, appId) {
  let dragging = false, ox = 0, oy = 0;

  titlebar.addEventListener('mousedown', e => {
    if (e.target.closest('.wc-btn')) return;  // don't drag via control buttons
    const state = windows[appId];
    if (state.maximized) {
      // Un-maximize first, then start dragging from normalized position
      toggleMaximize(appId);
      ox = state.w * 0.3;  // maintain 30% from left as grab point
      oy = 18;             // near top of titlebar
    } else {
      ox = e.clientX - win.offsetLeft;
      oy = e.clientY - win.offsetTop;
    }
    dragging = true;
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  });

  function onMove(e) {
    if (!dragging) return;
    const area = document.getElementById('desktop-area');
    const x = Math.max(0, Math.min(e.clientX - ox, area.offsetWidth - 50));
    const y = Math.max(0, Math.min(e.clientY - oy, area.offsetHeight - 36));
    win.style.left = x + 'px';
    win.style.top = y + 'px';
    windows[appId].x = x;
    windows[appId].y = y;
    updateSnapPreview(e.clientX, e.clientY);
  }

  function onUp(e) {
    dragging = false;
    document.removeEventListener('mousemove', onMove);
    document.removeEventListener('mouseup', onUp);
    applySnap(appId, e.clientX, e.clientY);
    hideSnapPreview();
    saveDesktopState();
  }
}
```

---

## Snap Zones

Snap zones activate when the dragged window's cursor position is within 20px of a desktop edge or corner.

```
┌──────────┬──────────┐
│ topleft  │ topright │  ← 20px strip at top
├────┬─────┴─────┬────┤
│    │           │    │  ← 20px strip at left/right
│left│  (center) │rght│
│    │           │    │
├────┴─────┬─────┴────┤
│btm-left  │ btm-rght │  ← 20px strip at bottom
└──────────┴──────────┘
```

**Snap targets:**
- `left`: x=0, y=0, w=50%, h=100%
- `right`: x=50%, y=0, w=50%, h=100%
- `topleft`: x=0, y=0, w=50%, h=50%
- `topright`: x=50%, y=0, w=50%, h=50%
- `bottomleft`: x=0, y=50%, w=50%, h=50%
- `bottomright`: x=50%, y=50%, w=50%, h=50%

While dragging near a zone, a semi-transparent ghost overlay appears at the snap target position. On mouseup inside the zone, the window animates (150ms ease-out) to the snap rect. The pre-snap rect is saved in `state.prevRect` for restoration.

---

## Maximize / Restore

```javascript
function toggleMaximize(appId) {
  const state = windows[appId];
  const area = document.getElementById('desktop-area');

  if (state.maximized) {
    // Restore from prevRect
    const r = state.prevRect;
    animateWindow(appId, r.x, r.y, r.w, r.h, 150);
    windows[appId].el.classList.remove('maximized');
    state.maximized = false;
    state.prevRect = null;
  } else {
    // Save current rect, then maximize
    state.prevRect = {x: state.x, y: state.y, w: state.w, h: state.h};
    animateWindow(appId, 0, 0, area.offsetWidth, area.offsetHeight, 150);
    windows[appId].el.classList.add('maximized');
    state.maximized = true;
  }
  saveDesktopState();
}
```

`animateWindow` uses CSS transitions on `left`, `top`, `width`, `height` with `ease-out` timing. The `maximized` class removes `border-radius` so corners are flush with the screen edge.

---

## Minimize / Restore

Minimized windows are hidden (`display: none`) but remain in the `windows` registry and in `zStack`. Their taskbar chip stays visible and changes to a dimmed state.

```javascript
function minimizeWindow(appId) {
  const state = windows[appId];
  // Animate sliding to taskbar chip position
  const chip = state.chip;
  const chipRect = chip.getBoundingClientRect();
  // Brief scale-to-chip animation, then hide
  state.el.style.transition = 'all 150ms ease-in';
  state.el.style.transform = `scale(0.1) translate(${chipRect.left}px, ${chipRect.top}px)`;
  state.el.style.opacity = '0';
  setTimeout(() => {
    state.el.style.display = 'none';
    state.el.style.transform = '';
    state.el.style.opacity = '';
    state.el.style.transition = '';
  }, 150);
  state.minimized = true;
  state.chip.classList.remove('active');
  saveDesktopState();
}
```

---

## Window Animations

All animations respect `prefers-reduced-motion`:

```css
@media (prefers-reduced-motion: reduce) {
  .window { transition: none !important; }
  .window-open-anim { animation: none !important; }
}
```

**Open animation** (new window appears):
```css
@keyframes windowOpen {
  from { transform: scale(0.9); opacity: 0; }
  to   { transform: scale(1);   opacity: 1; }
}
.window.opening { animation: windowOpen 150ms ease-out forwards; }
```

**Close animation**:
```css
@keyframes windowClose {
  from { transform: scale(1);   opacity: 1; }
  to   { transform: scale(0.9); opacity: 0; }
}
```

---

## Keyboard Navigation

| Shortcut | Action |
|---|---|
| `Alt+Tab` | Cycle focus through open windows |
| `Alt+Shift+Tab` | Cycle focus in reverse |
| `Alt+F4` | Close focused window |
| `Super+Left` | Snap focused window to left half |
| `Super+Right` | Snap focused window to right half |
| `Super+Up` | Maximize focused window |
| `Super+Down` | Restore / minimize focused window |
| `Ctrl+Alt+Left` | Switch to previous virtual desktop |
| `Ctrl+Alt+Right` | Switch to next virtual desktop |
| `Ctrl+Alt+1..9` | Jump to virtual desktop N |

---

## Virtual Desktops

```typescript
interface VirtualDesktop {
  id: number;       // 0-indexed
  label: string;    // user-defined, default "Desktop 1"
  windows: string[]; // app IDs on this desktop
}
```

The taskbar shows desktop indicators at the far right: `○ ● ○` where `●` is the current desktop. Clicking an indicator switches desktops. Right-clicking shows: Rename, Add Desktop, Remove Desktop.

Moving a window to another desktop:
- Right-click the taskbar chip → "Move to Desktop 2"
- The window is removed from `currentDesktop.windows`, hidden, added to `targetDesktop.windows`
- When the user switches to the target desktop, it appears at its saved position

---

## Taskbar Jump Lists

Right-click a taskbar chip for app-specific quick actions:

| App | Jump List Items |
|---|---|
| Terminal | New Tab, Split Horizontal, New Window, Close |
| Files | Open Home, Open Project Root, New Window, Close |
| OmniCC Build | Build, Build Release, Run, Test, Clean |
| OmniPM | Install All, Search, Update All |
| Code Studio | New Titan File, New Vera File, Open Recent |
| Bonsai Hub | Launch App, Open Dashboard, Status |

---

## Persistence Format

Saved to `~/.omnisystem/state/desktop.json`:

```json
{
  "version": 1,
  "virtualDesktops": [
    {"id": 0, "label": "Desktop 1", "windows": ["files", "terminal"]},
    {"id": 1, "label": "Dev", "windows": ["compiler", "code-studio"]}
  ],
  "currentDesktop": 0,
  "windows": {
    "files":    {"x":80,"y":30,"w":680,"h":520,"minimized":false,"maximized":false,"prevRect":null,"snapSide":null},
    "terminal": {"x":300,"y":100,"w":720,"h":440,"minimized":true,"maximized":false,"prevRect":null,"snapSide":null}
  },
  "zStack": ["files", "terminal"]
}
```

State is written on every window move, resize, minimize, maximize, or close. It is read at startup before the first render so the desktop appears in its previous configuration within 100ms of opening.
