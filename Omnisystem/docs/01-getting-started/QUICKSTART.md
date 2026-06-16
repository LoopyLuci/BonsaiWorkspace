# Quick Start Guide - Omnisystem

Get up and running with Omnisystem in 5 minutes.

## What is Omnisystem?

Omnisystem is a **complete software ecosystem** with 7 languages, universal frameworks, and device driver integration.

**The 7 Languages:**
- **TITAN** - Systems & I/O
- **SYLVA** - ML & Data Science
- **AETHER** - Distributed Systems
- **VERA** - Web Development
- **HELIX** - Graphics & Physics
- **NEXUS** - Mobile & IoT
- **AXIOM** - Formal Verification

## Your First Application

### 1. Web App (VERA)
```vera
module hello_web
import omnisystem.ui.widget_system

fun main() -> Result<(), String> {
    let button = widget_system::create_button(
        "Click Me",
        |_| println!("Clicked!")
    );
    Ok(())
}
```

### 2. Mobile App (NEXUS)
```nexus
module hello_mobile
import nexus.mobile_framework

fun main() {
    let activity = Activity::new("HelloMobile".to_string());
    let sensors = SensorManager::new();
}
```

### 3. Game (HELIX)
```helix
module hello_game
import helix.graphics_engine

fun main() {
    let renderer = Renderer::new(1920, 1080);
    let physics = Physics::new();
}
```

### 4. Data Science (SYLVA)
```sylva
module hello_data
import sylva.dataframe

fun main() -> Result<(), String> {
    let data = vec![
        vec!["Alice", "25", "Engineer"],
    ];
    let df = DataFrame::from_rows(data)?;
    Ok(())
}
```

## Key Concepts

### Universal Widget System
One widget abstraction works on web, mobile, and graphics.

### Cross-Language Communication
All languages communicate via connectors automatically.

### Asset Management
Unified pipeline for textures, models, audio, data.

### Service Registry
Discover and call services across languages.

## Resources

- **Full Documentation**: See [INDEX.md](../INDEX.md)
- **Language Guides**: Section 02 of docs
- **How-To Guides**: Section 04 of docs
- **API Reference**: Section 05 of docs
- **Examples**: See testing/examples/

---

**Status**: Production Ready  
**Last Updated**: 2026-06-16  
