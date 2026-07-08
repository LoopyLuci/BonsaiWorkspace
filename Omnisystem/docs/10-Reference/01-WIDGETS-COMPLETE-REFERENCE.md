# Complete UI Widget Reference Guide

**Omnisystem Widget Systems - Comprehensive Documentation**  
**Version**: 29.0.0  
**Updated**: June 16, 2026  
**Status**: Production-Ready

---

## Table of Contents

1. [Widget Systems Overview](#widget-systems-overview)
2. [VERA Core Widgets](#vera-core-widgets)
3. [Universal Widget System (TITAN)](#universal-widget-system-titan)
4. [Widget Framework Integration](#widget-framework-integration)
5. [Widget Specifications](#widget-specifications)
6. [Usage Patterns](#usage-patterns)
7. [Best Practices](#best-practices)

---

## Widget Systems Overview

The Omnisystem provides multiple widget frameworks designed for different use cases:

### Three-Tier Widget Architecture

```
┌─────────────────────────────────────────────────┐
│  APPLICATION LAYER - Specialized Widgets       │
│  (Domain-specific UI components)               │
├─────────────────────────────────────────────────┤
│  FRAMEWORK LAYER - Widget Frameworks           │
│  (VERA, Titan UI, Sylva UI, Web Components)   │
├─────────────────────────────────────────────────┤
│  CORE LAYER - Universal Widget System          │
│  (Base widget definitions, shared APIs)        │
└─────────────────────────────────────────────────┘
```

### Widget Framework Comparison

| Framework | Platform | Language | Widget Count | Use Case |
|-----------|----------|----------|--------------|----------|
| **VERA** | Desktop | VERA | 18+ core, 40+ total | Native desktop GUI |
| **Titan UI** | Multi-platform | TITAN | 50+ specialized | Systems programming |
| **Sylva UI** | Multi-platform | SYLVA | 10+ domains | ML/Data science |
| **Web Components** | Web Browser | React/TSX | 6,146+ | Web applications |
| **Rust/egui** | Native | Rust | 50+ | Performance-critical |
| **Universal** | All platforms | TITAN | 28 types | Cross-platform |

---

## VERA Core Widgets

VERA is the primary UI framework for the Omnisystem desktop environment, built on HELIX graphics engine with NEXUS responsiveness.

### Location
`Z:\Projects\Omnisystem\Omnisystem\applications\omnisystem-desktop-environment\src\`

### Widget Categories

#### 1. Basic Input Widgets

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **Button** | `widgets/WidgetSystem.vera` | Clickable action trigger | label, onClick, disabled, size, color |
| **TextInput** | `widgets/WidgetSystem.vera` | Single-line text entry | placeholder, value, onChange, validation |
| **Checkbox** | `widgets/WidgetSystem.vera` | Boolean toggle | label, checked, onChange, indeterminate |
| **RadioButton** | `widgets/WidgetSystem.vera` | Exclusive selection | label, selected, group, onChange |
| **Slider** | `widgets/WidgetSystem.vera` | Range selection | min, max, value, onChange, step, ticks |
| **Spinner** | `widgets/WidgetSystem.vera` | Numeric input | min, max, value, onChange, step |
| **DatePicker** | `widgets/WidgetSystem.vera` | Date selection | value, onChange, format, minDate, maxDate |
| **ColorPicker** | `widgets/WidgetSystem.vera` | Color selection | value, onChange, format, presets |
| **FilePicker** | `widgets/WidgetSystem.vera` | File selection | filters, multiple, onSelect, defaultPath |

#### 2. Display Widgets

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **Label** | `widgets/WidgetSystem.vera` | Text display | text, fontSize, color, alignment |
| **Image** | `widgets/WidgetSystem.vera` | Image rendering | src, alt, width, height, fit |
| **Icon** | `theme/ThemeEngine.vera` | Icon display | name, size, color, animation |
| **Badge** | `widgets/WidgetSystem.vera` | Status indicator | content, variant, color |
| **Progress** | `widgets/WidgetSystem.vera` | Progress indication | value, max, determinate, label |
| **Separator** | `layout/ResponsiveLayoutEngine.vera` | Visual divider | orientation, color, spacing |

#### 3. Container Widgets

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **Panel** | `widgets/AdvancedWidgets.vera` | Grouped content container | title, padding, border, backgroundColor |
| **Card** | `widgets/AdvancedWidgets.vera` | Standalone content card | title, content, elevation, onClick |
| **ScrollView** | `widgets/AdvancedWidgets.vera` | Scrollable content area | direction, scrollbar, onScroll, virtualScrolling |
| **Modal** | `dialogs/DialogSystem.vera` | Modal overlay | title, content, buttons, backdrop, animation |
| **Dialog** | `dialogs/DialogSystem.vera` | Modal dialog | type, title, message, buttons, icon |
| **Drawer** | `widgets/AdvancedWidgets.vera` | Side panel | position, width, onClose, persistent |

#### 4. Navigation Widgets

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **Menu** | `widgets/WidgetSystem.vera` | Dropdown menu | items, onChange, icon, position |
| **MenuItem** | `widgets/WidgetSystem.vera` | Menu item entry | label, icon, onClick, submenu |
| **Tabs** | `widgets/AdvancedWidgets.vera` | Tabbed interface | tabs, activeTab, onChange, variant |
| **Breadcrumb** | `widgets/AdvancedWidgets.vera` | Navigation path | items, separator, onClick |
| **Navbar** | `ui/SystemUI.vera` | Top navigation bar | title, items, logo, actions |
| **Taskbar** | `ui/SystemUI.vera` | Bottom application bar | apps, startMenu, systemTray, clock |

#### 5. Data Display Widgets

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **DataGrid** | `widgets/AdvancedWidgets.vera` | Tabular data display | columns, data, sortable, filterable, editable |
| **List** | `widgets/AdvancedWidgets.vera` | Item list | items, onSelect, virtual, pagination |
| **Tree** | `widgets/AdvancedWidgets.vera` | Hierarchical data | items, onSelect, expanded, icons |
| **TreeView** | `widgets/AdvancedWidgets.vera` | Advanced tree | items, multiSelect, draggable, checkboxes |
| **Chart** | `widgets/AdvancedWidgets.vera` | Data visualization | type, data, options, responsive |
| **GanttChart** | `widgets/AdvancedWidgets.vera` | Timeline visualization | tasks, milestones, dependencies |
| **Map** | `widgets/AdvancedWidgets.vera` | Geographic display | provider, markers, zoom, interactions |

#### 6. Text Editing Widgets

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **TextArea** | `widgets/AdvancedWidgets.vera` | Multi-line text editor | value, onChange, rows, cols, wrap |
| **RichTextEditor** | `widgets/AdvancedWidgets.vera` | Formatted text editor | content, onChange, toolbar, formats |
| **CodeEditor** | `widgets/AdvancedWidgets.vera` | Code input | language, value, onChange, syntax, theme |

#### 7. Specialized Components

| Widget | File | Purpose | Properties |
|--------|------|---------|-----------|
| **Tooltip** | `widgets/WidgetSystem.vera` | Hover information | content, position, delay, theme |
| **Toast** | `notifications/NotificationSystem.vera` | Temporary notification | message, type, duration, action |
| **Notification** | `notifications/NotificationSystem.vera` | Persistent message | title, message, type, action |
| **Popover** | `widgets/AdvancedWidgets.vera` | Floating content | content, trigger, position, closeOnClick |
| **Dropdown** | `widgets/WidgetSystem.vera` | Dropdown list | items, onChange, multi, searchable |
| **Autocomplete** | `widgets/AdvancedWidgets.vera` | Search suggestions | items, value, onChange, minChars |

### VERA Widget Organization Structure

```
Z:\Projects\Omnisystem\Omnisystem\applications\omnisystem-desktop-environment\src\
├── widgets/
│   ├── WidgetSystem.vera              # Core widget definitions (15+ widgets)
│   └── AdvancedWidgets.vera           # Enterprise widgets (20+ widgets)
├── ui/
│   ├── ApplicationWindow.vera         # Window management
│   ├── SystemUI.vera                  # Taskbar, system tray, desktop
│   └── [other UI components]
├── dialogs/
│   └── DialogSystem.vera              # Modal and dialog system
├── layout/
│   └── ResponsiveLayoutEngine.vera    # Responsive layout system
├── input/
│   ├── InputHandler.vera              # Event handling
│   └── GestureRecognitionSystem.vera  # Touch/gesture input
├── theme/
│   ├── ThemeEngine.vera               # Theme system
│   └── AdvancedThemingEngine.vera     # Advanced theming
├── notifications/
│   └── NotificationSystem.vera        # Toast/notification system
├── graphics/
│   ├── GraphicsEngine.vera            # Rendering engine
│   ├── AnimationEngine.vera           # Animation system
│   └── RenderingPipeline.vera         # Rendering pipeline
└── [other components]
```

---

## Universal Widget System (TITAN)

### File Location
`Z:\Projects\Omnisystem\Omnisystem\languages\universal_widget_system.ti`

### Universal Widget Types (28 Core Types)

```
Core Container Types:
├── Container          # Generic content container
├── Panel              # Grouped content
├── Card               # Standalone content
├── Dialog             # Modal dialog
├── Modal              # Modal overlay
├── Drawer             # Side panel
└── ScrollView         # Scrollable content

Interactive Types:
├── Button             # Clickable action
├── TextField          # Text input
├── Label              # Text display
├── Checkbox           # Boolean toggle
├── RadioButton        # Exclusive selection
├── Slider             # Range selection
├── DatePicker         # Date selection
├── ColorPicker        # Color selection
└── FilePicker         # File selection

Display Types:
├── Image              # Image rendering
├── Icon               # Icon display
├── Badge              # Status indicator
├── Progress           # Progress indication
└── Separator          # Visual divider

Data Types:
├── ListView           # Item list
├── Grid               # Grid layout
├── Chart              # Data visualization
└── Table              # Tabular data

Advanced Types:
├── Menu               # Menu system
├── Tab                # Tab interface
├── Autocomplete        # Search suggestions
└── CustomWidget       # User-defined widget
```

### Universal Widget Properties (Shared Across All)

```rust
pub struct Widget {
    id: String,
    type: String,
    properties: HashMap<String, WidgetProperty>,
    children: Vec<Box<Widget>>,
    layout_mode: LayoutMode,
    position: (f32, f32),
    size: (f32, f32),
    visible: bool,
    enabled: bool,
    focus: bool,
    z_index: i32,
}

pub enum WidgetProperty {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Color(String),      // Hex color code
    List(Vec<String>),
    Null,
}

pub enum LayoutMode {
    Vertical,           // Stack vertically
    Horizontal,         // Stack horizontally
    Absolute,           // Absolute positioning
    Grid,               // Grid layout
    Flex,               // Flexible layout
    Constraint,         // Constraint-based
}
```

### Universal Widget Event System

```rust
pub enum WidgetEvent {
    Click(MouseEvent),
    DoubleClick(MouseEvent),
    MouseEnter,
    MouseLeave,
    MouseMove(Position),
    Focus,
    Blur,
    KeyDown(KeyboardEvent),
    KeyUp(KeyboardEvent),
    Change(String),
    Submit,
    Cancel,
    Scroll(ScrollEvent),
    Resize(Size),
    Drag(DragEvent),
    Drop(DropEvent),
}

pub fn on_event(&mut self, event: WidgetEvent) {
    // Event handling implementation
}
```

### Titan UI Component Libraries (50+ Specialized)

#### Agent Control UI
- Agent status display
- Agent lifecycle management
- Agent communication widgets
- Agent configuration interface

#### Application Management
- App launcher interface (30+ apps)
- Recent applications list
- Favorites system
- Search functionality
- Category browsing

#### Alerting Configuration
- Alert rule builder
- Condition editor
- Action configuration
- Alert templates
- Severity levels

#### Automation Builder
- Workflow visual editor
- Trigger configuration
- Action sequences
- Conditional logic
- Loop structures

#### Charts & Visualization
- BarChart (vertical, horizontal, stacked)
- LineChart (single, multi-line, area)
- PieChart
- DoughnutChart
- ScatterChart
- BubbleChart
- HeatmapChart
- TimeseriesChart
- 3DChart variants

#### Dashboard Builder
- Grid-based layout
- Drag-and-drop widgets
- Widget library
- Preview mode
- Export functionality

#### Form Builder
- Field selection
- Validation rules
- Conditional fields
- Multi-step forms
- Progress indicators

#### Image Management
- Gallery view
- Crop tool
- Filter effects
- Metadata editor
- Batch operations

#### Metrics & Monitoring
- Real-time metrics display
- Historical charts
- Alert indicators
- Metric aggregation
- Custom dashboards

#### Settings Configuration
- Key-value editors
- Grouped settings
- Search functionality
- Settings profiles
- Backup/restore

#### 45+ More Specialized Modules...

---

## Widget Framework Integration

### Framework Layer Integration

```
┌─────────────────────────────────────────────┐
│  Application Uses Widget Framework          │
├─────────────────────────────────────────────┤
│  Widget Framework (VERA/Titan/Sylva/Web)   │
│  • Widget definitions                       │
│  • Event system                             │
│  • Layout system                            │
│  • Styling system                           │
├─────────────────────────────────────────────┤
│  Graphics Rendering (HELIX)                │
│  • GPU acceleration                         │
│  • Shader system                            │
│  • Animation system                         │
│  • 60 FPS rendering                         │
├─────────────────────────────────────────────┤
│  Layout System (NEXUS)                     │
│  • Responsive design                        │
│  • 4 breakpoints                            │
│  • Flex/Grid layouts                        │
│  • Constraints                              │
├─────────────────────────────────────────────┤
│  Theming & Styling (Theme Engine)          │
│  • Color schemes                            │
│  • Typography                               │
│  • Shadows & borders                        │
│  • Custom properties                        │
├─────────────────────────────────────────────┤
│  System Integration (TITAN/AETHER)         │
│  • File I/O                                 │
│  • Process management                       │
│  • Service mesh                             │
│  • IPC                                      │
└─────────────────────────────────────────────┘
```

### Widget Lifecycle

```
Create → Mount → Render → Update → Unmount → Destroy
  ↓        ↓        ↓       ↑        ↓        ↓
Initialize| Props |Redraw|State| Cleanup| Free
          |Changed|      |Changed|Observers|Memory
```

### Event Flow

```
User Input (Mouse/Keyboard)
    ↓
OS Event (Windows API)
    ↓
Event Router (AETHER)
    ↓
Widget Event Handler
    ↓
State Update
    ↓
Render Queue
    ↓
Layout System (NEXUS)
    ↓
Graphics Engine (HELIX)
    ↓
GPU Rendering
    ↓
Display Output
```

---

## Widget Specifications

### Standard Widget Properties

All widgets support these common properties:

```
visibility: boolean              # Show/hide
enabled: boolean                 # Enable/disable
width: number | string          # Width (px, %, auto)
height: number | string         # Height (px, %, auto)
padding: number | object        # Padding around content
margin: number | object         # Margin around widget
border: BorderDefinition        # Border styling
background: Color | Gradient    # Background color
shadow: ShadowDefinition        # Shadow effects
opacity: number                 # Transparency (0-1)
zIndex: number                  # Stacking order
tabIndex: number                # Tab order
className: string               # CSS class
style: object                   # Inline styles
id: string                      # Unique identifier
```

### Standard Events

All interactive widgets support these events:

```
onClick()                       # User clicked
onDoubleClick()                # User double-clicked
onMouseEnter()                 # Mouse entered
onMouseLeave()                 # Mouse left
onMouseMove()                  # Mouse moved
onFocus()                      # Widget focused
onBlur()                       # Widget lost focus
onKeyDown()                    # Key pressed
onKeyUp()                      # Key released
onChange()                     # Value changed
onSubmit()                     # Form submitted
onCancel()                     # Operation cancelled
onScroll()                     # Content scrolled
onResize()                     # Widget resized
onDrag()                       # Drag started
onDrop()                       # Drop completed
```

### Button Widget Specification

```
Button {
  // Base properties
  label: string                 # Button text
  icon: Icon                    # Optional icon
  
  // Styling
  variant: 'primary'           # primary, secondary, danger, success, warning
        | 'secondary'
        | 'danger'
        | 'success'
        | 'warning'
  size: 'small' | 'medium' | 'large'
  color: Color
  
  // State
  disabled: boolean
  loading: boolean
  onClick: (event) => void
  
  // Accessibility
  ariaLabel: string
  ariaPressed: boolean
  tabIndex: number
  
  // Tooltip
  tooltip: string
  tooltipPosition: 'top' | 'right' | 'bottom' | 'left'
}
```

### TextField Widget Specification

```
TextField {
  // Input
  type: 'text' | 'password' | 'email' | 'number' | 'tel' | 'url'
  value: string
  onChange: (value: string) => void
  
  // Display
  placeholder: string
  label: string
  helperText: string
  error: boolean
  errorMessage: string
  
  // Validation
  required: boolean
  pattern: RegExp
  minLength: number
  maxLength: number
  validate: (value: string) => boolean
  
  // Styling
  size: 'small' | 'medium' | 'large'
  variant: 'outlined' | 'filled' | 'standard'
  color: Color
  
  // State
  disabled: boolean
  readOnly: boolean
  focused: boolean
  
  // Events
  onFocus: () => void
  onBlur: () => void
  onKeyDown: (key: string) => void
  onKeyUp: (key: string) => void
  
  // Accessibility
  ariaLabel: string
  ariaDescribedBy: string
}
```

---

## Usage Patterns

### Pattern 1: Container Layout

```vera
Panel {
  title: "Settings",
  padding: 20,
  children: [
    TextField {
      label: "Username",
      value: username,
      onChange: (value) => setUsername(value)
    },
    TextField {
      label: "Email",
      type: "email",
      value: email,
      onChange: (value) => setEmail(value)
    },
    Button {
      label: "Save",
      onClick: () => saveSettings()
    }
  ]
}
```

### Pattern 2: Data Grid with Actions

```vera
DataGrid {
  columns: [
    { key: "name", header: "Name", sortable: true },
    { key: "email", header: "Email", sortable: true },
    { key: "actions", header: "Actions", render: (row) => ActionButtons(row) }
  ],
  data: users,
  onSort: (column) => sortUsers(column),
  onRowClick: (row) => selectUser(row),
  pagination: { pageSize: 10, current: currentPage }
}
```

### Pattern 3: Form Validation

```vera
Form {
  fields: [
    {
      name: "email",
      type: "text",
      label: "Email",
      validation: {
        required: true,
        pattern: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
        errorMessage: "Invalid email address"
      }
    }
  ],
  onSubmit: (formData) => submitForm(formData),
  onError: (errors) => displayErrors(errors)
}
```

### Pattern 4: Modal Dialog

```vera
Modal {
  title: "Confirm Delete",
  visible: showDeleteConfirm,
  onClose: () => setShowDeleteConfirm(false),
  children: [
    Label { text: "Are you sure you want to delete this item?" },
    div {
      Button {
        label: "Cancel",
        onClick: () => setShowDeleteConfirm(false)
      },
      Button {
        label: "Delete",
        variant: "danger",
        onClick: () => deleteItem()
      }
    }
  ]
}
```

### Pattern 5: Responsive Layout

```vera
ResponsiveContainer {
  breakpoints: {
    mobile: 320,
    tablet: 768,
    desktop: 1024,
    wide: 1440
  },
  children: [
    // Mobile: Stack vertically
    MediaQuery { maxWidth: 767 },
    // Tablet: 2-column grid
    MediaQuery { minWidth: 768, maxWidth: 1023 },
    // Desktop: 3-column grid
    MediaQuery { minWidth: 1024, maxWidth: 1439 },
    // Wide: 4-column grid
    MediaQuery { minWidth: 1440 }
  ]
}
```

---

## Best Practices

### 1. Widget Accessibility

✅ Always provide `ariaLabel` for icon-only buttons
✅ Use semantic HTML when available
✅ Support keyboard navigation (Tab, Enter, Esc)
✅ Ensure sufficient color contrast
✅ Provide alt text for images
✅ Use proper heading hierarchy
✅ Make forms properly labeled

### 2. Performance Optimization

✅ Use virtual scrolling for large lists
✅ Lazy-load images
✅ Memoize expensive computations
✅ Debounce event handlers
✅ Use proper component granularity
✅ Avoid unnecessary re-renders
✅ Use requestAnimationFrame for animations

### 3. State Management

✅ Keep widget state minimal
✅ Lift state up when needed
✅ Use unidirectional data flow
✅ Immutable state updates
✅ Single source of truth
✅ Proper cleanup on unmount

### 4. Error Handling

✅ Display clear error messages
✅ Provide recovery options
✅ Use consistent error styling
✅ Log errors for debugging
✅ Graceful degradation
✅ User-friendly error text

### 5. Styling Best Practices

✅ Use theme system for consistency
✅ Responsive design mobile-first
✅ Dark mode support
✅ Avoid inline styles (use classes)
✅ Consistent spacing and sizing
✅ Follow design system guidelines

### 6. Testing Widgets

✅ Unit test widget logic
✅ Integration test widget interactions
✅ Test accessibility features
✅ Test responsive behavior
✅ Test error states
✅ Performance testing for large data

---

## Widget Development Workflow

### Creating a Custom Widget

```vera
// 1. Define widget interface
pub struct MyCustomWidget {
  id: String,
  props: MyWidgetProps,
  state: MyWidgetState,
  children: Vec<Widget>,
}

// 2. Define properties
pub struct MyWidgetProps {
  label: String,
  value: String,
  onChange: Box<dyn Fn(String)>,
}

// 3. Define state
pub struct MyWidgetState {
  focused: bool,
  error: bool,
}

// 4. Implement widget lifecycle
impl Widget for MyCustomWidget {
  fn new(props: MyWidgetProps) -> Self { ... }
  fn render(&self) -> RenderTree { ... }
  fn on_event(&mut self, event: WidgetEvent) { ... }
  fn on_prop_change(&mut self, props: MyWidgetProps) { ... }
}

// 5. Export widget
pub fn MyCustomWidget(props: MyWidgetProps) -> Widget { ... }
```

---

## Summary

The Omnisystem widget ecosystem provides:

✅ **Comprehensive widget library** across all languages  
✅ **Consistent APIs** for cross-framework compatibility  
✅ **Performance optimization** with GPU acceleration  
✅ **Accessibility support** for inclusive design  
✅ **Responsive design** via NEXUS framework  
✅ **Theming system** for visual consistency  
✅ **Event system** for rich interactions  
✅ **Enterprise features** for production use  

---

**Document Version**: 29.0.0  
**Last Updated**: June 16, 2026  
**Status**: Complete and Production-Ready
