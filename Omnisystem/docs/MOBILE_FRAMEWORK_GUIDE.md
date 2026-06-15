# Mobile Framework Guide - Build Cross-Platform Apps

**Complete guide to building mobile applications with Omnisystem**

---

## Overview

The Mobile Framework provides:
- **UI Components**: Widgets, layouts, navigation
- **Native Access**: Camera, geolocation, sensors, storage
- **Performance**: Native compilation, minimal overhead
- **Cross-Platform**: iOS, Android, Web from single codebase

---

## Quick Start

### Hello Mobile App

```titan
use omnisystem::mobile::*

fun main() -> Result<(), str> {
    let mut app = MobileApp::new("MyApp")
    
    let screen = Screen::new()
        .add(Text::new("Hello, Mobile!"))
        .add(Button::new("Press me").on_tap(|_| {
            println!("Button pressed")
        }))
    
    app.add_screen("home", screen)
    app.show()?
    
    Ok(())
}
```

---

## UI Components

### Basic Widgets

```titan
// Text
let text = Text::new("Hello")
    .with_size(18)
    .with_color(Color::Blue)

// Button
let btn = Button::new("Click")
    .on_tap(|_| println!("Tapped"))

// TextInput
let input = TextInput::new()
    .with_placeholder("Enter name")
    .on_change(|text| println!("{}", text))

// Image
let img = Image::load("asset.png")?
    .with_width(100)
    .with_height(100)

// Checkbox
let cb = Checkbox::new("Agree")
    .on_change(|checked| println!("{}", checked))

// Switch
let sw = Switch::new()
    .with_initial(true)
```

### Layouts

```titan
// VStack (vertical)
let column = VStack::new()
    .add(Text::new("Top"))
    .add(Text::new("Middle"))
    .add(Text::new("Bottom"))

// HStack (horizontal)
let row = HStack::new()
    .add(Text::new("Left"))
    .add(Text::new("Right"))

// Grid
let grid = Grid::new(columns: 2, rows: 2)
    .add(Button::new("1"))
    .add(Button::new("2"))
    .add(Button::new("3"))
    .add(Button::new("4"))

// Spacer
let spacer = Spacer::new().with_height(20)
```

---

## Navigation

### Screen Navigation

```titan
fun main() -> Result<(), str> {
    let mut app = MobileApp::new("MyApp")
    
    // Home screen
    let home = Screen::new()
        .add(Text::new("Home"))
        .add(Button::new("Go to Details").on_tap(|nav| {
            nav.push("details")
        }))
    
    // Details screen
    let details = Screen::new()
        .add(Text::new("Details"))
        .add(Button::new("Back").on_tap(|nav| {
            nav.pop()
        }))
    
    app.add_screen("home", home)
    app.add_screen("details", details)
    app.set_initial("home")
    
    app.show()?
    Ok(())
}
```

### Tab Navigation

```titan
let mut app = MobileApp::new("MyApp")

let tab1 = Screen::new().add(Text::new("Home"))
let tab2 = Screen::new().add(Text::new("Search"))
let tab3 = Screen::new().add(Text::new("Settings"))

let tabs = TabNavigator::new()
    .add("Home", "home_icon.png", tab1)
    .add("Search", "search_icon.png", tab2)
    .add("Settings", "settings_icon.png", tab3)

app.set_navigator(tabs)
app.show()?
```

---

## Native Features

### Camera

```titan
use omnisystem::mobile::*

fun take_photo() -> Result<Image, str> {
    let camera = Camera::new()
    let photo = camera.take_photo()?
    Ok(photo)
}

fun access_gallery() -> Result<Vec<Image>, str> {
    let gallery = Gallery::new()
    gallery.select_images(max_count: 5)
}
```

### Geolocation

```titan
fun get_location() -> Result<Location, str> {
    let location = Geolocation::current()?
    println!("Lat: {}, Long: {}", location.latitude, location.longitude)
    Ok(location)
}

fun watch_location() -> Result<(), str> {
    let mut geo = Geolocation::new()
    
    geo.on_location_change(|loc| {
        println!("Moved to: {} {}", loc.latitude, loc.longitude)
    })
    
    geo.start_watching()?
    Ok(())
}
```

### Storage

```titan
fun save_data() -> Result<(), str> {
    let storage = Storage::new()
    
    // Preferences
    storage.set_preference("theme", "dark")?
    let theme = storage.get_preference("theme")?
    
    // Files
    storage.save_file("data.json", &json_content)?
    let content = storage.load_file("data.json")?
    
    Ok(())
}
```

### Notifications

```titan
fun send_notification() -> Result<(), str> {
    let notif = Notification::new("Title", "Message")
        .with_icon("icon.png")
        .with_sound()
        .on_tap(|_| println!("Tapped"))
    
    notif.show()?
    Ok(())
}
```

### Sensors

```titan
fun accelerometer() -> Result<(), str> {
    let mut sensors = Sensors::new()
    
    sensors.on_accelerometer(|accel| {
        println!("X: {}, Y: {}, Z: {}", accel.x, accel.y, accel.z)
    })
    
    sensors.start()?
    Ok(())
}
```

---

## State Management

### App State

```titan
type AppState {
    user: Option<User>,
    theme: string,
    notifications: Vec<Notification>,
}

fun main() -> Result<(), str> {
    let mut state = AppState {
        user: None,
        theme: "light",
        notifications: vec![],
    }
    
    // Update state
    state.user = Some(user)
    state.theme = "dark"
    
    // Persist state
    persist_state(&state)?
    
    Ok(())
}
```

---

## Styling

### Theming

```titan
let theme = Theme::new()
    .with_primary_color(Color::Blue)
    .with_secondary_color(Color::Green)
    .with_font("Roboto")
    .with_font_size(16)

let text = Text::new("Styled")
    .with_style(&theme)
```

### Layout Styling

```titan
let btn = Button::new("Click")
    .with_padding(10)
    .with_margin(5)
    .with_corner_radius(8)
    .with_shadow(offset: (2, 2), blur: 4)
```

---

## Example: Todo App

```titan
use omnisystem::mobile::*

type Todo {
    id: i32,
    title: string,
    completed: bool,
}

type AppState {
    todos: Vec<Todo>,
    next_id: i32,
}

fun main() -> Result<(), str> {
    let mut app = MobileApp::new("TodoApp")
    let mut state = AppState {
        todos: vec![],
        next_id: 1,
    }
    
    // Home screen - list todos
    let home = build_home_screen(&state)?
    
    app.add_screen("home", home)
    app.set_initial("home")
    app.show()?
    
    Ok(())
}

fun build_home_screen(state: &AppState) -> Result<Screen, str> {
    let mut list = VStack::new()
    
    for todo in &state.todos {
        list = list.add(
            HStack::new()
                .add(Checkbox::new(&todo.title)
                    .with_checked(todo.completed))
                .add(Spacer::new().with_width(10))
                .add(Button::new("Delete").on_tap(|_| {
                    // Delete logic
                }))
        )
    }
    
    let screen = Screen::new()
        .add(Text::new("Todos"))
        .add(list)
        .add(Button::new("Add Todo").on_tap(|nav| {
            nav.push("add_todo")
        }))
    
    Ok(screen)
}
```

---

## Performance Tips

✅ **DO**
- Lazy load images
- Cache data locally
- Minimize re-renders
- Use list virtualization

❌ **DON'T**
- Load all images upfront
- Sync on main thread
- Redraw entire screen
- Store large objects in memory

---

## Platform-Specific Code

```titan
#[cfg(target_os = "ios")]
fn ios_specific() {
    // iOS-only code
}

#[cfg(target_os = "android")]
fn android_specific() {
    // Android-only code
}

#[cfg(target_os = "web")]
fn web_specific() {
    // Web-only code
}
```

---

## Testing Mobile Apps

```titan
#[test]
fn test_ui_render() {
    let screen = Screen::new()
        .add(Text::new("Test"))
    
    assert_eq!(screen.components.len(), 1)
}
```

---

## Deployment

### iOS
```bash
omnisystem build --platform ios --release
# Creates .ipa for App Store
```

### Android
```bash
omnisystem build --platform android --release
# Creates .apk or .aab for Play Store
```

### Web
```bash
omnisystem build --platform web --release
# Creates static HTML/JS for hosting
```

---

## Next Steps

- Study [WEB_FRAMEWORK_GUIDE.md](WEB_FRAMEWORK_GUIDE.md) - Web framework shares concepts
- Check [SYSTEMS_FRAMEWORK_GUIDE.md](SYSTEMS_FRAMEWORK_GUIDE.md) - System integration
- Deploy using [DEPLOYMENT.md](DEPLOYMENT.md)

---

**Mobile Framework** - Build beautiful, fast mobile apps!
