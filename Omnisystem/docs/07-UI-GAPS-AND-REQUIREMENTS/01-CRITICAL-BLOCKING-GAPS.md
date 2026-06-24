# CRITICAL BLOCKING GAPS - UI Widgets & Assets Required

**What Must Be Built Before Users Can Use Omnisystem**  
**Status**: Currently 0-10% Complete  
**Total Effort**: 85-130 hours  
**Priority**: HIGHEST - System Cannot Function Without These

---

## Overview

These systems have backend logic but NO functional UI. Users cannot interact with or manage Omnisystem without these UIs being implemented.

---

## 1. CONTROL PANEL - System Management Console

### Current Status
- **Backend**: Architecture document exists (21KB)
- **Implementation**: ZERO source code
- **Can User Do This?**: Cannot monitor or manage Omnisystem at all

### What Users Need To Do
- Check system health and performance
- Start/stop/restart services
- View and filter system logs
- Manage system capabilities
- Create system snapshots
- Adjust system settings and configuration
- Monitor CPU, memory, disk, network usage

### UI Type Required
**Multi-tab desktop application** with real-time updates

### Required Widgets & Assets

| Tab | Widgets Needed | Assets Needed | Complexity |
|-----|----------------|---------------|-----------|
| **Status** | Uptime display, version info, health indicator, status grid | System icons, status colors | Low-Medium |
| **Resources** | Line charts (CPU/memory/disk), gauge charts (current %), resource alerts | Chart components, meter icons, color indicators | Medium |
| **Services** | Service list, status badges, control buttons (start/stop/restart), dependency tree | Service icons, status indicators, control icons | Medium-High |
| **Logs** | Log viewer (scrollable), filter controls, search box, log level colors | Icon assets for log types, color scheme | Medium |
| **Capabilities** | Tree view of registered capabilities, capability description panels | Tree icons, capability badges | Low-Medium |
| **Snapshots** | Snapshot list, creation dialog, restore confirmation, snapshot size display | Snapshot icons, time/date display | Low |
| **Settings** | Configuration form (toggles, dropdowns, text inputs), validation messages | Settings icons, form controls | Medium |

### Specific Widget Requirements

```
Core Components Needed:
├── Dashboard/Status Panel
│   ├── Key Metrics Display (4 main cards: CPU, Memory, Disk, Uptime)
│   ├── System Health Indicator (ring gauge or status light)
│   ├── Service Status Grid (visual status for each service)
│   └── Quick Stats (services running, capabilities registered, etc.)
│
├── Charts/Graphs
│   ├── Real-time line charts (CPU, memory, disk, network usage over 1 hour)
│   ├── Gauge charts (current % usage)
│   ├── Stacked bar charts (if multi-core CPU display)
│   └── History viewer (click to see historical data)
│
├── Service Management
│   ├── Service list with status badges
│   ├── Start/Stop/Restart buttons
│   ├── Service dependency visualization
│   ├── Service log tail display
│   └── Resource usage per service
│
├── Configuration
│   ├── Settings form with groups/categories
│   ├── Input validation and error display
│   ├── Settings search
│   └── Reset to defaults confirmation
│
└── Logging
    ├── Log level filter controls
    ├── Full-text search
    ├── Auto-scrolling log display
    ├── Log export functionality
    └── Date/time range filter
```

### Asset Requirements

**Icons**: 30+ icons needed
- System icons (cpu, memory, disk, network, services, settings, etc.)
- Action icons (start, stop, restart, refresh, export, settings)
- Status icons (running, stopped, error, warning, ok)
- Chart icons (line chart, bar chart, etc.)

**Colors/Theme**:
- Status colors (green=ok, red=error, yellow=warning, gray=stopped)
- Chart colors for different metrics

**Fonts**: 
- Mono font for log viewer and technical values

### Priority & Blocking Status
**🔴 CRITICAL - BLOCKING**
- Users cannot manage the system without this
- All other UIs depend on understanding system state
- Cannot troubleshoot without logs

### Estimated Effort
- Windows (VERA): 40-50 hours
- macOS (VERA): 35-45 hours  
- Linux (VERA): 35-45 hours
- **Total**: 110-140 hours (2-3 developers, 2-3 weeks)

### Implementation Roadmap
1. **Week 1**: Status tab (uptime, version, basic metrics)
2. **Week 2**: Charts tab (real-time line/gauge charts)
3. **Week 3**: Services tab (list, start/stop controls)
4. **Week 4**: Logs, settings, capabilities tabs

---

## 2. INSTALLER - Cross-Platform Installation

### Current Status
- **Backend**: Host detection code only (273 lines of Titan)
- **UI**: COMPLETELY MISSING
- **Can User Do This?**: Cannot install Omnisystem on any platform

### What Users Need To Do
- Download and run installer
- Accept license agreement
- Choose installation path
- Select components to install
- Track installation progress
- See completion and launch options
- Get help with errors
- Uninstall cleanly

### UI Type Required
**Installation wizard** with multi-step flow

### Required Widgets & Assets

| Step | Widgets Needed | Assets Needed |
|------|----------------|---------------|
| **Welcome** | Logo/branding, intro text, terms agreement link, Next button | App logo, brand colors, installer banner |
| **License** | Scrollable text area, license content, Accept/Decline buttons | License document, status text |
| **Components** | Checkbox tree view, component descriptions, dependency warnings, estimated size | Component icons, folder icons |
| **Destination** | File path input, Browse button, path validation, free space display | Folder icon, path validation icons |
| **Installation** | Progress bar, step-by-step list, current step highlight, cancel button, error dialogs | Progress indicators, warning/error icons |
| **Completion** | Summary text, launch checkbox, launch button, view logs link | Success icon, document icon |

### Platform-Specific Requirements

**Windows:**
- Visual installer exe (using NSIS or similar)
- Windows registry entries
- Add/Remove Programs integration
- Desktop shortcut creation
- Start Menu integration
- Uninstaller generation

**macOS:**
- DMG creation
- .app bundle packaging
- Gatekeeper/notarization (code signing)
- Launchd plist for auto-start (optional)
- Uninstaller script

**Linux (Multiple Formats):**
- .deb package (Debian/Ubuntu)
- .rpm package (CentOS/RHEL)
- .pacman package (Arch)
- Systemd integration
- AppImage for universal Linux

### Priority & Blocking Status
**🔴 CRITICAL - BLOCKING**
- Users cannot install without this
- First impression of Omnisystem
- Must be polished and professional

### Estimated Effort
- Windows installer: 25-30 hours
- macOS installer: 20-25 hours
- Linux installers (.deb, .rpm, .pacman): 25-30 hours
- Cross-platform wizard UI: 10-15 hours
- **Total**: 80-100 hours (2-3 developers, 2-3 weeks)

---

## 3. SYSTEM TRAY INTEGRATION - Background App Access

### Current Status
- **Backend**: Mentioned in architecture only
- **Implementation**: COMPLETELY MISSING
- **Can User Do This?**: Cannot minimize or access app from system tray

### What Users Need To Do
- Minimize Omnisystem to system tray
- Click tray icon to restore window
- Access quick-launch menu from tray
- See system status in tray icon
- Close app from tray

### UI Type Required
**Native system tray widget** with context menu

### Required Widgets & Assets

| Platform | Widgets Needed | Assets Needed |
|----------|----------------|---------------|
| **Windows** | Tray icon object, context menu, balloon tooltip | Tray icon (16x16, 32x32), animated icon for activity |
| **macOS** | NSStatusBar item, popup menu, icon animation | Tray icon for light/dark menu bar, retina versions |
| **Linux** | D-Bus StatusNotifierItem, menu, icon | Tray icon, freedesktop icon theme compliance |

### Features Required
- Tray icon display (always visible)
- Context menu with options:
  - Show/Hide window
  - Status submenu (CPU %, Memory %)
  - Services status
  - Exit
- Icon animation for activity
- Tooltip with system stats
- Click to restore window

### Asset Requirements
**Icons**: 
- Base tray icon (16x16, 32x32, 48x48 for different scales)
- Active/inactive variants
- Error state variant
- Dark/light theme variants
- Retina versions (@2x) for macOS

### Priority & Blocking Status
**🔴 CRITICAL - BLOCKING**
- Users need background mode for system management
- Essential UX feature
- Users expect to minimize apps to tray

### Estimated Effort
- Windows implementation: 15-20 hours
- macOS implementation: 12-15 hours
- Linux implementation: 12-15 hours
- Icon design and variants: 5-8 hours
- **Total**: 44-58 hours (1-2 developers, 1-2 weeks)

---

## 4. NOTIFICATION SYSTEM - Event Alerts

### Current Status
- **Backend**: Not found (architecture referenced)
- **Tauri**: Notification plugin exists in dependencies but not integrated
- **Implementation**: COMPLETELY MISSING
- **Can User Do This?**: No visual feedback for events

### What Users Need To Do
- Receive notifications for system events
- View notification history
- Access notification center
- Dismiss or interact with notifications
- Configure notification preferences (do not disturb, etc.)

### UI Type Required
**Toast notifications + Notification Center panel**

### Required Widgets & Assets

| Component | Widgets Needed | Assets Needed |
|-----------|----------------|---------------|
| **Toast Notification** | Popup widget, title, message, action button, auto-dismiss | Notification icons (info, warning, error, success), notification sound |
| **Notification Center** | Persistent panel, notification list, clear all button, filter by type | Clear icon, archive icon, notification type icons |
| **Notification Detail** | Expanded view, action buttons, timestamp, category | Category badges, action icons |
| **Settings** | Toggle for notification types, DND schedule, sound preferences | Settings icons, toggle switches |

### Features Required

**Toast Notifications:**
- Auto-dismiss after 5-10 seconds
- Click to perform action
- Multiple concurrent notifications (stack)
- Different types (info, warning, error, success)
- Sound/visual indicators

**Notification Center:**
- Persistent history of all notifications
- Organized by category/time
- Mark as read/unread
- Action buttons on notifications
- Search/filter capabilities
- Archive/delete options

**Settings:**
- Enable/disable per notification type
- Do Not Disturb schedule
- Sound on/off
- Grouping preferences
- Retention policy (keep for X days)

### Asset Requirements

**Icons**:
- Info icon
- Warning icon  
- Error icon
- Success/checkmark icon
- Clear/delete icon
- Settings icon
- Clock icon

**Audio**:
- Notification sounds (system, warning, error)
- Should be adjustable by user

### Priority & Blocking Status
**🔴 CRITICAL - BLOCKING**
- Users need feedback when events occur
- Essential for system management
- First way users learn of problems

### Estimated Effort
- Notification daemon: 10-15 hours
- Toast UI implementation: 8-12 hours
- Notification center UI: 10-15 hours
- Settings UI: 5-8 hours
- Icon/audio assets: 5-8 hours
- **Total**: 38-58 hours (1-2 developers, 1-2 weeks)

---

## CRITICAL GAPS SUMMARY

| System | Blocking | Hours | Weeks | Users Affected |
|--------|----------|-------|-------|-----------------|
| Control Panel | YES | 110-140 | 2-3 | ALL - cannot manage system |
| Installer | YES | 80-100 | 2-3 | ALL - cannot install |
| System Tray | YES | 44-58 | 1-2 | ALL - no background mode |
| Notifications | YES | 38-58 | 1-2 | ALL - no event feedback |
| **TOTAL** | | **272-356** | **6-10** | |

---

## Development Timeline for Critical Systems

### Phase 1 (Weeks 1-2): Installer
- Focus: Get Omnisystem installable on all platforms
- Teams: Windows, macOS, Linux teams in parallel
- Blocker for everything else

### Phase 2 (Weeks 3-4): Control Panel - Basic
- Focus: System status and basic service control
- Essential for system management

### Phase 3 (Weeks 5): Control Panel - Advanced
- Focus: Logs, settings, capabilities tabs
- Makes system manageable

### Phase 4 (Week 6): Notifications + System Tray
- Focus: User feedback and background mode
- Essential UX

---

## Success Criteria

**Installer**: Users can successfully install and launch Omnisystem

**Control Panel**: Users can:
- View real-time system metrics
- Start/stop services
- View and search logs
- Understand system health
- Manage system settings

**System Tray**: Users can minimize and restore app

**Notifications**: Users receive event notifications

---

**Document Version**: 29.0.0  
**Last Updated**: June 23, 2026  
**Status**: Defines CRITICAL blocking UI gaps
