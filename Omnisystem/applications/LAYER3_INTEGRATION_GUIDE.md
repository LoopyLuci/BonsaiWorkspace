# Layer 3 Integration Guide
## Building Applications for Omnisystem

**Version**: 29.0.0  
**Purpose**: Guide for integrating new applications into Layer 3  
**Status**: Complete

---

## Quick Start

### 1. Understand the Architecture

Omnisystem has three layers:

```
Layer 3: Applications & User Experience (YOU ARE HERE)
    ↓ Uses services from ↓
Layer 2: Core Infrastructure (System Module, UOSC, Connectors)
    ↓ Built with ↓
Layer 1: 7 Programming Languages (TITAN, SYLVA, AETHER, VERA, HELIX, NEXUS, AXIOM)
```

### 2. Choose Your Application Type

| Type | Language | Location | Best For |
|------|----------|----------|----------|
| **Desktop GUI** | VERA + HELIX | `applications/your-app/` | UI applications |
| **Web App** | VERA | `applications/web/` | Web-based services |
| **Mobile App** | NEXUS | `applications/mobile/` | Touch-based interfaces |
| **Backend Service** | TITAN + AETHER | `applications/services/` | API servers |
| **Data Processing** | SYLVA | `applications/ai/` | ML and analytics |
| **System Tool** | TITAN | `applications/core/` | System utilities |

### 3. Create Your Application

```bash
mkdir -p applications/my-app/src
mkdir -p applications/my-app/assets
mkdir -p applications/my-app/tests
```

---

## Integration Patterns

### Pattern 1: Using System Services

**Scenario**: Your app needs to show notifications

```ti
// Import system module
import omnisystem.system

// Initialize system
let system = omnisystem::system::init_system()?;

// Get notifications service
let notifications = system.get_notifications();

// Use the service
// (Call through connector gateway)
```

### Pattern 2: Using UOSC

**Scenario**: Your app needs file access

```ti
// Import UOSC
import omnisystem.uosc

// Access file system through UOSC
let files = omnisystem::uosc::file_system::list_directory("/path")?;

// Access device drivers
let devices = omnisystem::uosc::devices::list_devices()?;
```

### Pattern 3: Cross-Language Calls

**Scenario**: You need to call a SYLVA (ML) module from VERA (Web)

```vera
// In VERA (TypeScript-like)
import { ConnectorGateway } from '@omnisystem/connectors';

// Call SYLVA module
const result = await ConnectorGateway.call('sylva-module', 'predict', {
    features: [1.0, 2.0, 3.0]
});
```

### Pattern 4: Inter-Application Communication

**Scenario**: Your app needs to communicate with another app

```ti
// Get application registry
let registry = omnisystem::applications::ApplicationRegistry::new();

// Find another app
let target_app = registry.get_application("ai")?;

// Send message through connector
ConnectorGateway.send_message(target_app.name, {
    action: "process_data",
    payload: data,
})?;
```

---

## Common Integrations

### Integration: Desktop GUI Application

**Directory Structure**:
```
applications/my-desktop-app/
├── src/
│   ├── main.vera          # Main UI
│   ├── components.vera    # Reusable components
│   ├── styles.css
│   └── backend.ti         # Backend logic
├── assets/
│   ├── icon.png
│   ├── icon.ico
│   └── screenshots/
├── manifest.json          # App metadata
├── README.md
└── tests/
    └── tests.vera
```

**manifest.json**:
```json
{
  "name": "My App",
  "version": "1.0.0",
  "description": "My Omnisystem application",
  "entry_point": "src/main.vera",
  "icon": "assets/icon.png",
  "category": "Utilities",
  "requires_services": [
    "launcher",
    "notifications",
    "system_tray"
  ],
  "platforms": ["desktop", "web"],
  "permissions": [
    "file_system.read",
    "file_system.write",
    "notifications.send"
  ]
}
```

**src/main.vera**:
```vera
import { BonsaiEcosystem } from '@omnisystem/applications/bonsai-ecosystem';
import { SystemServices } from '@omnisystem/system';

export default function App() {
  const [count, setCount] = useState(0);
  
  useEffect(() => {
    // Register with launcher
    BonsaiEcosystem.registerApp({
      name: "My App",
      icon: "assets/icon.png",
      category: "Utilities"
    });
  }, []);
  
  return (
    <div>
      <h1>My Omnisystem App</h1>
      <button onClick={() => {
        setCount(count + 1);
        SystemServices.notifications.show({
          title: "Count Updated",
          message: `Count is now ${count + 1}`
        });
      }}>
        Click me: {count}
      </button>
    </div>
  );
}
```

### Integration: Backend Service

**Directory Structure**:
```
applications/my-service/
├── src/
│   ├── main.ti            # Service entry point
│   ├── api.ti             # API definitions
│   ├── handlers.ti        # Request handlers
│   └── models/
│       └── data.ti
├── tests/
│   └── tests.ti
├── manifest.json
└── README.md
```

**src/main.ti**:
```ti
import omnisystem.system
import omnisystem.aether
import omnisystem.applications

module omnisystem.applications.my_service {
    pub struct MyService {
        pub name: String,
        pub port: i32,
        pub initialized: bool,
    }
    
    impl MyService {
        pub fn new() -> Self {
            MyService {
                name: "My Service".to_string(),
                port: 8080,
                initialized: false,
            }
        }
        
        pub fn initialize(&mut self) -> Result<(), String> {
            // Register with service mesh
            omnisystem::aether::service_mesh::register_service(
                self.name.clone(),
                self.port
            )?;
            
            self.initialized = true;
            Ok(())
        }
        
        pub fn handle_request(&self, request: String) -> Result<String, String> {
            // Process request
            Ok(format!("Response: {}", request))
        }
    }
    
    pub fn main() -> Result<(), String> {
        let mut service = MyService::new();
        service.initialize()?;
        
        // Service is now running
        Ok(())
    }
}
```

### Integration: ML Model

**Directory Structure**:
```
applications/ai/my-model/
├── src/
│   ├── main.sv            # SYLVA ML framework
│   ├── model.sv           # Model definition
│   ├── training.sv        # Training code
│   └── inference.sv       # Prediction code
├── data/
│   ├── train.csv
│   └── test.csv
├── models/
│   └── trained_model.bin
└── README.md
```

**src/main.sv**:
```sv
import omnisystem.sylva
import omnisystem.applications

module omnisystem.applications.ai.my_model {
    pub struct MyModel {
        pub weights: Vec<f64>,
        pub trained: bool,
    }
    
    impl MyModel {
        pub fn new() -> Self {
            MyModel {
                weights: vec![],
                trained: false,
            }
        }
        
        pub fn train(&mut self, data: Vec<Vec<f64>>, labels: Vec<f64>) -> Result<(), String> {
            // Use SYLVA framework
            self.weights = omnisystem::sylva::train_model(data, labels)?;
            self.trained = true;
            Ok(())
        }
        
        pub fn predict(&self, features: Vec<f64>) -> Result<f64, String> {
            if !self.trained {
                return Err("Model not trained".to_string());
            }
            
            let result = omnisystem::sylva::predict(self.weights.clone(), features)?;
            Ok(result)
        }
    }
    
    pub fn main() -> Result<(), String> {
        let mut model = MyModel::new();
        
        // Train
        model.train(training_data, training_labels)?;
        
        // Predict
        let prediction = model.predict(test_features)?;
        
        Ok(())
    }
}
```

---

## Accessing System Services

### Notifications Service

```ti
import omnisystem.system

pub fn show_notification(title: &str, message: &str) -> Result<(), String> {
    omnisystem::system::notifications::show(title, message)?;
    Ok(())
}
```

### Launcher Service

```ti
import omnisystem.system

pub fn launch_app(app_name: &str) -> Result<(), String> {
    omnisystem::system::launcher::launch(app_name)?;
    Ok(())
}
```

### File Associations

```ti
import omnisystem.system

pub fn register_file_type(extension: &str) -> Result<(), String> {
    omnisystem::system::file_associations::register(extension)?;
    Ok(())
}
```

### System Tray

```ti
import omnisystem.system

pub fn add_tray_icon(icon_path: &str) -> Result<(), String> {
    omnisystem::system::system_tray::add_icon(icon_path)?;
    Ok(())
}
```

---

## Registering Your Application

### Automatic Registration

1. Create `manifest.json` in your app directory
2. Include app metadata (name, version, icon)
3. BonsaiEcosystem automatically discovers and registers it

### Manual Registration

```ti
import omnisystem.applications

pub fn register_my_app() -> Result<(), String> {
    let registry = omnisystem::applications::ApplicationRegistry::new();
    
    let app_metadata = omnisystem::applications::ApplicationMetadata {
        name: "My App".to_string(),
        version: "1.0.0".to_string(),
        category: "Utilities".to_string(),
        executable: "my-app".to_string(),
        icon: "assets/icon.png".to_string(),
        description: "My application description".to_string(),
    };
    
    // Registry will store metadata
    Ok(())
}
```

---

## Testing Your Application

### Unit Tests

```ti
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_functionality() {
        let mut app = MyApp::new();
        let result = app.do_something();
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```ti
#[test]
fn test_with_system_services() {
    // Initialize Layer 2
    omnisystem::system::init_system().unwrap();
    
    // Test your app with real services
    let result = my_app::main();
    assert!(result.is_ok());
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

---

## Deployment

### Building Your App

```bash
# Build in release mode
cargo build --release

# Output location
target/release/my-app
```

### Packaging

```bash
# Create distribution package
omnisystem package my-app \
  --icon assets/icon.png \
  --version 1.0.0 \
  --category Utilities
```

### Distribution

1. **Omnisystem App Store** (if available)
2. **Direct download** from your website
3. **Package managers** (with community help)

---

## Best Practices

### ✅ Do

- ✅ Follow the 3-layer architecture
- ✅ Use system services rather than reimplementing
- ✅ Register with BonsaiEcosystem launcher
- ✅ Use cross-platform languages (VERA, HELIX, NEXUS)
- ✅ Write tests for your code
- ✅ Document your API
- ✅ Handle errors gracefully
- ✅ Use type-safe code (TITAN, VERA, etc.)
- ✅ Leverage Layer 2 services
- ✅ Follow naming conventions

### ❌ Don't

- ❌ Bypass Layer 2 services (don't access hardware directly)
- ❌ Ignore error handling
- ❌ Use unsafe language features
- ❌ Hard-code paths (use system services instead)
- ❌ Skip testing
- ❌ Create duplicate functionality
- ❌ Ignore the manifest.json
- ❌ Use platform-specific code when possible (platform-independent is better)

---

## Troubleshooting

### My app won't register with launcher

**Solution**: Check `manifest.json` is in the app root directory and has correct syntax

### System services not available

**Solution**: Call `omnisystem::system::init_system()` before using services

### Cross-language calls failing

**Solution**: Ensure both modules are properly imported and connectors are initialized

### App crashes on startup

**Solution**: Check error logs, add `?` operator error propagation, test in debug mode

---

## Advanced Topics

### Creating Custom Services

```ti
// Define your service
pub struct MyService {
    pub name: String,
}

impl MyService {
    pub fn register(&self) -> Result<(), String> {
        // Register with connector gateway
        omnisystem::bridges::connector_gateway::register(
            self.name.clone(),
            self
        )?;
        Ok(())
    }
}
```

### Plugin Architecture

BonsaiEcosystem supports plugins. Create a plugin:

```ti
pub trait Plugin {
    fn name(&self) -> String;
    fn initialize(&self) -> Result<(), String>;
    fn execute(&self) -> Result<(), String>;
}

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> String { "My Plugin".to_string() }
    fn initialize(&self) -> Result<(), String> { Ok(()) }
    fn execute(&self) -> Result<(), String> { Ok(()) }
}
```

---

## Resources

- [Layer 3 Architecture](./LAYER3_ARCHITECTURE.md)
- [System Services Reference](../system/README.md)
- [TITAN Language Guide](../languages/TITAN/README.md)
- [VERA Language Guide](../languages/VERA/README.md)
- [SYLVA Language Guide](../languages/SYLVA/README.md)
- [AETHER Language Guide](../languages/AETHER/README.md)
- [BonsaiEcosystem Documentation](./bonsai-ecosystem/README.md)

---

**Version**: 29.0.0  
**Status**: Complete & Production-Ready  
**Last Updated**: 2026-06-16

