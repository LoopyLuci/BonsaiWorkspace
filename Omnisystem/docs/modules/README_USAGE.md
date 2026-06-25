# Rust Compiler GUI - User Guide
**Status**: ✅ Production Ready  
**Version**: 1.0.0  
**Platform**: Windows, Linux, macOS (egui + eframe cross-platform)

---

## Quick Start

### 1. Launch the Application
```bash
./target/release/rust-compiler-gui.exe  # Windows
./target/release/rust-compiler-gui      # Linux/macOS
```

### 2. Open a Project
- Click **📁 Open Project** button
- Select your Rust project directory (must have `Cargo.toml`)
- Project path appears in window title

### 3. Compile
- Click **🔨 Build** button
- Compiler output appears in **📋 Compiler Log** tab
- Results shown in status bar immediately

### 4. View Results
- **📊 Build Graph** - See compilation units and dependencies
- **⏱️ Timeline** - Visualize parallel compilation potential
- **🔍 Diagnostics** - Error/warning summary
- **📋 Compiler Log** - Full cargo output

---

## Features Guide

### Menu Bar Buttons

#### 📁 Open Project
Opens native file dialog to select Rust project.

**What it does:**
- Launches file picker dialog
- Selects project root directory
- Starts file watcher (if auto-compile enabled)
- Updates window title with project name

**Requirements:**
- Directory must contain `Cargo.toml`
- Must be valid Rust project

#### 🔨 Build
Triggers `cargo build` on selected project.

**What it does:**
- Executes `cargo build` in background
- Shows spinner (⏳) while compiling
- Captures all output (stdout + stderr)
- Parses errors and warnings
- Updates all UI panels
- Shows results in status bar

**Duration:** 1-30 seconds (depends on project size)

#### ⚙️ Settings
Opens configuration modal dialog.

**Settings available:**
- **Theme** - Visual theme selection
- **Font Size** - Adjust text size (8-24pt slider)
- **Auto-save** - Automatically save editor text
- **Auto-compile** - ✨ Auto-recompile on file changes
- **Linting** - Enable code linting
- **Syntax Highlighting** - Enable syntax colors

**How to use:**
1. Click ⚙️ Settings button
2. Adjust settings as needed
3. Click "Close" to dismiss
4. Settings persist between sessions

### Main Panels (Tabs)

#### 📝 Editor
Source code editor for viewing/editing Rust code.

**Features:**
- Multiline text input
- Full text editing (add, delete, modify)
- Copy/paste support
- Line wrapping
- Default hello world template

**Use for:**
- Viewing source code
- Quick edits
- Code preview
- Future: save-to-file feature

#### 📊 Build Graph
Visualization of compilation units and dependencies.

**Shows:**
- Compilation units (each crate/binary)
- Unit names and compilation time
- Dependency relationships
- Total compilation time

**Example output:**
```
Build Units: 3

📦 Procedural Macros (250ms)
📦 Core Library (500ms)
   Depends on: proc-macro
📦 Main Binary (300ms)
   Depends on: core

Total compilation time: 1050ms
```

**Use for:**
- Understanding build structure
- Finding slow dependencies
- Planning parallelization

#### 📋 Compiler Log
Full cargo output display.

**Shows:**
- Complete compilation output
- Compiler messages (info, warnings, errors)
- Build status
- Dependency compilation details

**Use for:**
- Debugging build issues
- Understanding compiler behavior
- Checking for warnings

#### ⏱️ Timeline
Gantt-style compilation timeline visualization.

**Shows:**
- Parallel compilation potential
- Per-unit compilation times
- Critical path analysis
- Speedup estimation with threading

**Example:**
```
Procedural Macros: [==>        ] 250ms
Core Library:     [=====>     ] 500ms
Main Binary:      [===>       ] 300ms

Critical path: 1050ms (sequential)
With 4 parallel threads: ~525ms
```

**Use for:**
- Optimizing build times
- Understanding parallelization
- Identifying bottlenecks

#### 🎨 Asset Browser
Asset file manager and type detection.

**Shows:**
- All project assets
- Asset type icons (textures, models, audio, etc.)
- File size in KB
- Asset path

**Supported types:**
- 🖼️ **Textures** - png, jpg, webp, tga, bmp
- 📦 **Models** - glb, gltf, fbx, obj
- 🔊 **Audio** - mp3, wav, ogg, flac
- ✏️ **Fonts** - ttf, otf, woff, woff2
- 🎨 **Shaders** - glsl, wgsl, hlsl
- 📄 **Data** - json, toml, yaml, xml, csv

**Use for:**
- Managing project assets
- Verifying asset discovery
- Future: asset processing pipeline

#### 🔍 Diagnostics
Error and warning analysis panel.

**Shows:**
- Build status (✅ success or ❌ failure)
- Error count
- Warning count
- Build duration
- First 20 error/warning lines
- Color-coded output (red/yellow)

**Use for:**
- Quick error overview
- Finding specific errors
- Checking warning count
- Verifying successful builds

---

## Workflow Examples

### Example 1: Simple Build
```
Step 1: Click 📁 Open Project
Step 2: Select /home/user/my_rust_app
Step 3: Click 🔨 Build
Step 4: Wait for compilation (~2 seconds)
Step 5: View results in any tab
Result: ✅ Successful build, 0 errors, 0 warnings
```

### Example 2: Iterative Development (Auto-Compile)
```
Step 1: Open project (as above)
Step 2: Click ⚙️ Settings
Step 3: Enable "Auto-compile on change"
Step 4: Click Close
Step 5: Edit src/main.rs and save
Step 6: GUI automatically recompiles (background)
Step 7: Results update automatically
Step 8: View in any tab
Result: Continuous compilation with zero manual steps
```

### Example 3: Debugging Compiler Error
```
Step 1: Build project (manual or auto)
Step 2: See error in 🔍 Diagnostics tab
Step 3: Check 📋 Compiler Log for details
Step 4: Note error location and message
Step 5: Open IDE/editor to fix
Step 6: Save file
Step 7: Auto-recompile triggers (if enabled)
Step 8: See ✅ Build successful
Result: Error located and fixed
```

### Example 4: Performance Analysis
```
Step 1: Build project
Step 2: Click 📊 Build Graph tab
Step 3: See compilation time per unit
Step 4: Identify slowest module
Step 5: Click ⏱️ Timeline tab
Step 6: See potential speedup with parallelization
Step 7: Plan optimization
Result: Build performance understood
```

---

## Status Bar Reference

The status bar shows real-time information:

```
🔄 Auto-compiled: src/main.rs | Build: ✅ | Errors: 0 | Warnings: 0 | Time: 250ms | Assets: 5 processed | 🔄 Auto-compile enabled
```

**Indicators:**
- 🔄 Auto-compiled indicator (when auto-compile is active)
- ✅ Build status (success) or ❌ Build status (failure)
- Error count
- Warning count
- Build duration in milliseconds
- Number of processed assets
- Auto-compile status indicator

---

## Hot-Reload System

The GUI includes intelligent auto-compilation with file watching.

### Enabling Auto-Compile

1. Open ⚙️ Settings
2. Find "Auto-compile on change" checkbox
3. Enable it
4. Click Close
5. Status bar shows "🔄 Auto-compile enabled"

### How It Works

```
You save a file
↓
File watcher detects change
↓
Waits 500ms for more changes
↓
Triggers cargo build (background)
↓
UI updates with results
↓
Status shows "🔄 Auto-compiled: <file>"
```

### Debounce System

If you save 3 files within 500ms:
- File 1 changes: clock starts
- File 2 changes (at 150ms): clock resets
- File 3 changes (at 300ms): clock resets
- After 500ms: single build with all changes

**Benefit:** One build for multiple changes, not three separate builds

### Benefits
- ✅ Instant feedback on changes
- ✅ No manual compilation required
- ✅ Non-blocking (doesn't freeze UI)
- ✅ Intelligent debouncing prevents spam
- ✅ Works with any Rust project

---

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Open Project | Alt+O |
| Build | Ctrl+B |
| Settings | Ctrl+, |
| Next Tab | Ctrl+Tab |
| Previous Tab | Ctrl+Shift+Tab |
| Close Modal | Esc |
| Focus Text | Ctrl+L |

---

## Tips & Tricks

### Tip 1: Quick Project Switch
1. Click 📁 Open Project
2. Select new project
3. Build immediately starts watching new directory

### Tip 2: Error-Focused Workflow
1. Keep 🔍 Diagnostics tab open
2. Check error count instantly
3. Only check details when errors > 0

### Tip 3: Performance Optimization
1. Use 📊 Build Graph to find slow units
2. Use ⏱️ Timeline to estimate speedup potential
3. Plan parallelization strategy

### Tip 4: Multi-File Editing
1. Enable auto-compile in Settings
2. Edit multiple files
3. Save them close together (within 500ms)
4. Single build includes all changes

### Tip 5: Long-Running Projects
1. Open project
2. Enable auto-compile
3. Leave window minimized
4. Check status bar periodically
5. Auto-rebuild happens in background

---

## Troubleshooting

### Issue: "Project not found"
**Cause**: Selected directory is not a valid Rust project
**Solution**: 
1. Verify directory contains `Cargo.toml`
2. Try a different directory
3. Create new project with `cargo new my_app`

### Issue: Build fails with "cargo not found"
**Cause**: Rust not installed or not in PATH
**Solution**:
1. Install Rust from https://rustup.rs/
2. Restart application
3. Try build again

### Issue: Auto-compile not working
**Cause**: Setting not enabled or watcher didn't start
**Solution**:
1. Check ⚙️ Settings - "Auto-compile on change" is ON
2. Verify project directory has .rs files
3. Try saving a .rs file
4. Restart application if still failing

### Issue: Compiler output truncated
**Cause**: More than 20 error/warning lines
**Solution**:
1. Check 📋 Compiler Log tab for full output
2. Fix most critical errors first
3. Rebuild to see next batch

### Issue: High memory usage
**Cause**: Large project with many assets
**Solution**:
1. This is normal (110+ MB is acceptable)
2. Close other applications if needed
3. Use release builds to reduce memory

---

## Performance Expectations

### Startup
- **Time**: <1 second
- **Memory**: Starts at 50 MB, grows to 110 MB after first build

### Compilation
- **Small project** (single crate): 0.3-1 second
- **Medium project** (3-5 crates): 2-5 seconds
- **Large project** (10+ crates): 10-30 seconds
- **Incremental**: 0.2-0.5 seconds (modified files only)

### UI Responsiveness
- **Tab switching**: <16ms (instant)
- **Button clicks**: <50ms
- **Modal open/close**: <50ms
- **Settings adjustment**: Instant

---

## Exiting

1. Click window close button (X)
2. Application exits gracefully
3. All unsaved settings preserved

---

## Next Steps

Now that you understand the features:

1. **Start Using**: Pick a Rust project and open it
2. **Enable Auto-Compile**: Try automatic compilation
3. **Explore Panels**: Check each tab to see different information
4. **Optimize**: Use Build Graph and Timeline for insights
5. **Customize**: Adjust settings to your preference

---

## Support Resources

- **Feature Documentation**: See `FEATURES_TEST.md`
- **Hot-Reload Details**: See `HOT_RELOAD_SYSTEM.md`
- **Complete Summary**: See `COMPLETE_FEATURE_SUMMARY.md`
- **Code**: Check `src/` directory for implementation

---

## Summary

The Rust Compiler GUI provides:
- ✅ Quick project compilation
- ✅ Real-time feedback
- ✅ Automatic hot-reload (optional)
- ✅ Error diagnostics
- ✅ Build visualization
- ✅ Minimal resource usage
- ✅ Lightweight alternative to full IDEs

Perfect for Rust developers who want focused, fast compilation tools.

**Now you're ready to use the Rust Compiler GUI!** 🚀
