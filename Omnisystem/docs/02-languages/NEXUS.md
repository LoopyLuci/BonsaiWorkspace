# NEXUS Guide - Mobile & IoT

**NEXUS** is Omnisystem's mobile and IoT language, providing unified access to mobile and embedded device features.

## Core Features

### Mobile UI
- Activity lifecycle
- Native components
- Layouts and positioning
- Event handling

### Sensors
- GPS location
- Accelerometer
- Gyroscope
- Magnetometer
- Light sensor
- Proximity sensor

### System Integration
- Permissions
- Notifications
- Local storage
- Network connectivity

## Common Usage

```nexus
let activity = Activity::new("MyApp".to_string());
let sensors = SensorManager::new();

// Access GPS
if let Some(gps) = sensors.get_sensor("gps") {
    println!("Location: {}", gps.values[0]);
}

// Request permission
permissions.request("CAMERA")?;

// Show notification
notify("Title", "Message")?;
```

## Related Documentation

- [API Reference](../05-reference/NEXUS_API.md)
- [Building Mobile Apps](../04-guides/MOBILE_APPS.md)

---

**Status**: Production Ready | **Updated**: 2026-06-16
