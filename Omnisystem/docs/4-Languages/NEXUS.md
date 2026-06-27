# NEXUS Language Guide
## Mobile & IoT Language | 1,000+ Functions
**Status:** ✅ Production Ready | **Tier:** Cross-Platform Mobile & Embedded

---

## Overview

**NEXUS** is the mobile and IoT language for building cross-platform apps, wearables, and embedded systems. Single codebase compiles to iOS, Android, and IoT platforms natively.

### Key Characteristics
- **Cross-Platform:** iOS, Android, watchOS, embedded
- **Hardware Access:** Full sensor/GPIO/connectivity
- **Native Performance:** Direct platform APIs
- **Reactive UI:** Data-driven interfaces
- **Offline-First:** Works without connectivity
- **Quantum IoT Ready:** Quantum sensor support

### Best Use Cases
- Native mobile applications
- Wearable applications
- IoT device firmware
- Smart home systems
- Embedded systems
- Sensor networks

---

## Mobile Application Development

### 1. Activity & UI

#### Activity Lifecycle
```nexus
pub activity MainActivity {
    let user_profile = reactive(Option::None);
    let loading = reactive(false);
    
    fn on_create() {
        // Called when activity is created
        set_content_view(R.layout.main);
        
        let toolbar = find_view_by_id(R.id.toolbar);
        toolbar.set_title("My App");
        toolbar.set_elevation(4.0);
        
        // Setup UI components
        setup_navigation_drawer();
        setup_bottom_nav_bar();
        
        // Load initial data
        load_user_profile();
    }
    
    fn on_resume() {
        // Called when activity becomes visible
        refresh_data();
        enable_sensors();
    }
    
    fn on_pause() {
        // Called when activity loses focus
        disable_sensors();
        save_state();
    }
    
    fn on_destroy() {
        // Called before activity is destroyed
        cleanup_resources();
    }
    
    fn on_request_permissions_result(request_code: i32, permissions: Vec<String>, results: Vec<i32>) {
        // Handle permission results
        if request_code == CAMERA_REQUEST {
            if results[0] == PERMISSION_GRANTED {
                open_camera();
            } else {
                show_error("Camera permission denied");
            }
        }
    }
}
```

#### Screen & Widget Components
```nexus
pub screen LoginScreen {
    let email = reactive("".to_string());
    let password = reactive("".to_string());
    let errors = reactive(vec![]);
    let loading = reactive(false);
    
    fn validate_form() -> Result<(), String> {
        if email.value.is_empty() {
            return Err("Email required".to_string());
        }
        if !email.value.contains("@") {
            return Err("Invalid email format".to_string());
        }
        if password.value.len() < 8 {
            return Err("Password must be 8+ characters".to_string());
        }
        Ok(())
    }
    
    async fn handle_login() {
        errors.value.clear();
        
        if let Err(e) = validate_form() {
            errors.value.push(e);
            return;
        }
        
        loading.value = true;
        
        match login_api(email.value.clone(), password.value.clone()).await {
            Ok(user) => {
                save_user_session(user);
                navigate_to("HomeScreen");
            }
            Err(e) => {
                errors.value.push(format!("Login failed: {}", e));
            }
        }
        
        loading.value = false;
    }
    
    render() -> Layout {
        Layout::vertical(vec![
            // Header
            Text::new("Welcome")
                .size(32)
                .weight(Bold),
            
            // Email input
            TextInput::new()
                .hint("Email")
                .value(email)
                .keyboard(EmailKeyboard)
                .on_change(|text| email.value = text),
            
            // Password input
            TextInput::new()
                .hint("Password")
                .value(password)
                .keyboard(PasswordKeyboard)
                .secure(true)
                .on_change(|text| password.value = text),
            
            // Error messages
            if errors.value.is_empty() {
                Empty::new()
            } else {
                Column::new(
                    errors.value.iter().map(|err| {
                        Text::new(err)
                            .color(Color::Red)
                            .size(14)
                    }).collect()
                )
            },
            
            // Login button
            Button::new("Login")
                .on_click(|| handle_login())
                .enabled(!loading.value),
            
            // Loading indicator
            if loading.value {
                ProgressIndicator::circular()
            } else {
                Empty::new()
            },
            
            // Register link
            Button::new("Create Account")
                .style(TextButton)
                .on_click(|| navigate_to("RegisterScreen")),
        ])
    }
}
```

### 2. Navigation

#### Navigation Structure
```nexus
pub fn setup_navigation() {
    // Bottom Navigation
    let bottom_nav = BottomNavigationView::new();
    bottom_nav.add_menu_item(MenuItem {
        id: "home",
        label: "Home",
        icon: R.drawable.ic_home,
    });
    bottom_nav.add_menu_item(MenuItem {
        id: "search",
        label: "Search",
        icon: R.drawable.ic_search,
    });
    bottom_nav.add_menu_item(MenuItem {
        id: "profile",
        label: "Profile",
        icon: R.drawable.ic_profile,
    });
    
    bottom_nav.set_on_item_selected(|item| {
        match item.id.as_str() {
            "home" => navigate_to("HomeScreen"),
            "search" => navigate_to("SearchScreen"),
            "profile" => navigate_to("ProfileScreen"),
            _ => {}
        }
    });
    
    // Navigation Drawer
    let drawer = NavigationDrawerView::new();
    drawer.add_header(DrawerHeader {
        title: "My App",
        subtitle: "Version 1.0",
    });
    drawer.add_item(DrawerItem {
        label: "Settings",
        icon: R.drawable.ic_settings,
        action: || navigate_to("SettingsScreen"),
    });
    drawer.add_divider();
    drawer.add_item(DrawerItem {
        label: "About",
        icon: R.drawable.ic_info,
        action: || show_about_dialog(),
    });
}
```

#### Deep Linking
```nexus
pub fn handle_intent(intent: Intent) {
    let uri = intent.get_data();
    
    match uri.scheme.as_str() {
        "myapp" => {
            match uri.host.as_str() {
                "user" => {
                    if let Some(user_id) = uri.get_query_parameter("id") {
                        navigate_to_user_profile(user_id);
                    }
                }
                "post" => {
                    if let Some(post_id) = uri.get_query_parameter("id") {
                        navigate_to_post(post_id);
                    }
                }
                _ => navigate_to("HomeScreen"),
            }
        }
        "https" => {
            // Handle web URLs
            launch_browser(uri);
        }
        _ => {}
    }
}
```

---

## Sensor & Hardware Integration

### 1. Sensor Access

#### Motion Sensors
```nexus
pub fn setup_motion_sensors() {
    // Accelerometer
    let accelerometer = SensorManager::get_sensor(SensorType::Accelerometer);
    accelerometer.set_sampling_period(SamplingPeriod::Default);
    
    accelerometer.on_sensor_event(|event| {
        let x = event.values[0];  // m/s²
        let y = event.values[1];
        let z = event.values[2];
        
        // Detect shake gesture
        let magnitude = (x*x + y*y + z*z).sqrt();
        if magnitude > 50.0 {
            handle_shake_gesture();
        }
    });
    
    accelerometer.start_listening();
    
    // Gyroscope
    let gyroscope = SensorManager::get_sensor(SensorType::Gyroscope);
    gyroscope.on_sensor_event(|event| {
        let roll = event.values[0];    // rad/s
        let pitch = event.values[1];
        let yaw = event.values[2];
        
        update_device_orientation(roll, pitch, yaw);
    });
    
    gyroscope.start_listening();
}
```

#### Environmental Sensors
```nexus
pub fn setup_environmental_sensors() {
    // Temperature
    let temp_sensor = SensorManager::get_sensor(SensorType::AmbientTemperature);
    temp_sensor.on_sensor_event(|event| {
        let celsius = event.values[0];
        println!("Temperature: {}°C", celsius);
    });
    
    // Humidity
    let humidity_sensor = SensorManager::get_sensor(SensorType::RelativeHumidity);
    humidity_sensor.on_sensor_event(|event| {
        let humidity = event.values[0];
        println!("Humidity: {}%", humidity);
    });
    
    // Pressure (barometer)
    let pressure_sensor = SensorManager::get_sensor(SensorType::Pressure);
    pressure_sensor.on_sensor_event(|event| {
        let hpa = event.values[0];
        let altitude = calculate_altitude(hpa);
        println!("Altitude: {}m", altitude);
    });
}
```

#### GPS & Location
```nexus
pub fn setup_location_services() {
    let location_manager = LocationManager::get_instance();
    
    location_manager.request_location_updates(
        LocationProvider::GPS,
        UpdateInterval {
            min_time_ms: 1000,
            min_distance_meters: 10.0,
        },
    );
    
    location_manager.on_location_changed(|location| {
        let latitude = location.latitude;
        let longitude = location.longitude;
        let accuracy = location.accuracy;
        let altitude = location.altitude;
        
        println!("Location: ({}, {})", latitude, longitude);
        println!("Accuracy: ±{}m", accuracy);
        println!("Altitude: {}m", altitude);
        
        update_map_position(latitude, longitude);
    });
    
    location_manager.on_provider_enabled(|provider| {
        println!("{} enabled", provider);
    });
    
    location_manager.on_provider_disabled(|provider| {
        println!("{} disabled", provider);
    });
}
```

### 2. Camera Access

#### Photo Capture
```nexus
pub fn setup_camera() {
    let camera_manager = CameraManager::get_instance();
    
    // Get available cameras
    let back_camera = camera_manager.get_camera(CameraFacing::Back);
    let front_camera = camera_manager.get_camera(CameraFacing::Front);
    
    // Open camera
    let camera = camera_manager.open_camera(back_camera);
    
    camera.set_preview_size(1920, 1080);
    camera.set_image_format(ImageFormat::NV21);
    camera.set_fps(30);
    
    // Capture frames
    camera.set_preview_callback(|frame_data| {
        process_camera_frame(frame_data);
    });
    
    camera.start_preview();
}

pub fn take_photo() {
    let camera = CameraManager::current_camera();
    
    camera.take_picture(
        PictureCallback {
            on_picture_taken: |jpeg_data| {
                save_photo(jpeg_data);
                let bitmap = jpeg_to_bitmap(jpeg_data);
                display_preview(bitmap);
            }
        }
    );
}
```

### 3. Connectivity

#### Bluetooth
```nexus
pub fn setup_bluetooth() {
    let bt_manager = BluetoothManager::get_instance();
    
    // Start discovery
    bt_manager.start_discovery();
    
    bt_manager.on_device_discovered(|device| {
        println!("Found: {} ({})", device.name, device.address);
    });
    
    bt_manager.on_discovery_finished(|| {
        println!("Discovery finished");
    });
    
    // Connect to device
    let device = BluetoothDevice {
        address: "XX:XX:XX:XX:XX:XX",
        name: "My Device",
    };
    
    let socket = bt_manager.create_socket(device);
    socket.connect()?;
    
    // Send data
    socket.write(b"Hello, Bluetooth!");
    
    // Receive data
    socket.on_data_received(|data| {
        println!("Received: {:?}", data);
    });
}
```

#### WiFi & Network
```nexus
pub fn setup_network() {
    let network_manager = NetworkManager::get_instance();
    
    // WiFi
    network_manager.wifi_enable(true);
    let networks = network_manager.get_available_networks();
    
    network_manager.connect_to_network(WifiNetwork {
        ssid: "MyNetwork",
        password: "password123",
        security: SecurityType::WPA2,
    });
    
    network_manager.on_wifi_connected(|| {
        println!("WiFi connected");
        perform_sync();
    });
    
    // Check connectivity
    let is_online = network_manager.is_connected();
    let is_metered = network_manager.is_metered();
}
```

---

## Data Storage

### 1. Local Database
```nexus
pub fn setup_database() {
    let db = LocalDatabase::new("myapp.db");
    
    // Create table
    db.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            email TEXT UNIQUE,
            name TEXT,
            created_at INTEGER
        )"
    )?;
    
    // Insert
    db.execute(
        "INSERT INTO users (email, name, created_at) VALUES (?, ?, ?)",
        vec!["user@example.com", "John Doe", "1234567890"],
    )?;
    
    // Query
    let cursor = db.query(
        "SELECT * FROM users WHERE email = ?",
        vec!["user@example.com"],
    )?;
    
    for row in cursor {
        let id: i32 = row.get_int("id");
        let email: String = row.get_string("email");
        let name: String = row.get_string("name");
        println!("User: {} - {}", name, email);
    }
    
    // Update
    db.execute(
        "UPDATE users SET name = ? WHERE id = ?",
        vec!["Jane Doe", "1"],
    )?;
    
    // Delete
    db.execute(
        "DELETE FROM users WHERE id = ?",
        vec!["1"],
    )?;
}
```

### 2. Secure Storage
```nexus
pub fn secure_storage() {
    let secure = SecureStorage::get_instance();
    
    // Store sensitive data
    secure.put("api_key", "secret_key_12345");
    secure.put("password", "hashed_password");
    
    // The data is encrypted using Android Keystore / iOS Keychain
    
    // Retrieve
    if let Some(api_key) = secure.get("api_key") {
        println!("API Key: {}", api_key);
    }
    
    // Delete
    secure.delete("password");
    
    // Clear all
    secure.clear();
}
```

### 3. Local Storage (Key-Value)
```nexus
pub fn preferences_storage() {
    let prefs = SharedPreferences::get_instance();
    
    // Store primitives
    prefs.put_int("user_id", 123);
    prefs.put_string("username", "john_doe");
    prefs.put_bool("is_premium", true);
    prefs.put_float("rating", 4.5);
    
    // Retrieve with defaults
    let user_id = prefs.get_int("user_id", 0);
    let username = prefs.get_string("username", "Anonymous");
    let is_premium = prefs.get_bool("is_premium", false);
    
    // Listen for changes
    prefs.on_change(|key| {
        println!("Preference changed: {}", key);
    });
    
    // Delete
    prefs.delete("is_premium");
}
```

---

## Advanced Features

### 1. Background Services
```nexus
pub service SyncService {
    fn on_create() {
        // Called when service is created
        println!("SyncService created");
    }
    
    fn on_start_command(intent: Intent, flags: i32, start_id: i32) -> i32 {
        // Called when startService() is called
        spawn_async(|| {
            sync_with_server().await;
            stop_service();
        });
        
        // Return START_REDELIVER_INTENT to restart if process is killed
        ServiceStartMode::START_REDELIVER_INTENT
    }
    
    fn on_bind(intent: Intent) -> IBinder {
        // Called when bindService() is called
        ServiceBinder::new(self)
    }
    
    fn on_destroy() {
        // Called when service is destroyed
        cleanup_resources();
    }
}
```

### 2. Notifications
```nexus
pub fn send_notification(title: String, message: String) {
    let notification = Notification {
        id: 1,
        title,
        message,
        channel_id: "default",
        priority: Priority::High,
        small_icon: R.drawable.ic_notification,
        large_icon: load_bitmap("notification_icon.png"),
        actions: vec![
            NotificationAction {
                id: "reply",
                label: "Reply",
                icon: R.drawable.ic_reply,
            },
            NotificationAction {
                id: "dismiss",
                label: "Dismiss",
                icon: R.drawable.ic_dismiss,
            },
        ],
        big_style: Some(BigTextStyle {
            big_text: "This is a long notification message that will be displayed in expanded form".to_string(),
        }),
    };
    
    NotificationManager::show(notification);
}
```

### 3. Work Scheduling
```nexus
pub fn schedule_background_work() {
    let scheduler = WorkScheduler::get_instance();
    
    // One-time work
    let work = WorkRequest {
        tag: "sync_work",
        initial_delay: Duration::minutes(15),
        work_fn: || {
            sync_with_server()
        }
    };
    
    scheduler.enqueue_work(work);
    
    // Periodic work
    let periodic = PeriodicWorkRequest {
        tag: "periodic_sync",
        interval: Duration::minutes(60),
        flex_interval: Duration::minutes(15),
        work_fn: || {
            periodic_sync()
        }
    };
    
    scheduler.enqueue_periodic(periodic);
}
```

---

## Complete App Example

```nexus
pub app MyApp {
    fn on_create() {
        // Initialize
        setup_navigation();
        request_permissions();
        setup_database();
        setup_sensors();
    }
}

pub activity MainActivity {
    fn on_create() {
        set_content_view(R.layout.main);
        setup_initial_screen();
    }
}

pub screen HomeScreen {
    let data = reactive(vec![]);
    let loading = reactive(true);
    
    use_effect(|| {
        async {
            loading.value = true;
            data.value = fetch_data().await.unwrap_or_default();
            loading.value = false;
        }
    }, vec![]);
    
    render() -> Layout {
        if loading.value {
            ProgressIndicator::circular()
        } else {
            ListView::new(
                data.value.iter().map(|item| {
                    ListItemView::new()
                        .title(item.title.clone())
                        .subtitle(item.subtitle.clone())
                        .on_click(|| navigate_to_detail(item.id))
                }).collect()
            )
        }
    }
}
```

---

**NEXUS: Native Mobile & IoT Applications**

🚀 [Back to Language Guide](../LANGUAGES.md)
