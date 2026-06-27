# Omnisystem Developer Guide

**Version**: 1.0  
**Status**: ✅ **COMPLETE**  
**Last Updated**: 2026-06-28  

---

## TABLE OF CONTENTS

1. [Quick Start](#quick-start)
2. [Development Environment](#development-environment)
3. [Project Structure](#project-structure)
4. [Building & Compiling](#building--compiling)
5. [Testing](#testing)
6. [Contributing](#contributing)
7. [Code Standards](#code-standards)
8. [API Documentation](#api-documentation)

---

## QUICK START

### Prerequisites
```bash
# Install TITAN compiler
brew install titan-lang

# Install Rust toolchain
curl https://sh.rustup.rs | sh

# Clone repository
git clone https://github.com/omnisystem/omnisystem.git
cd omnisystem
```

### First Build
```bash
# Build entire system
make build

# Run tests
make test

# Start development server
make dev
```

### Your First Module

```titan
// modules/my-module/hello.titan
pub struct HelloWorld {
    message: String
}

impl HelloWorld {
    pub fn new(msg: String) -> Self {
        HelloWorld {
            message: msg
        }
    }

    pub fn greet(self: Self) {
        println!("Hello: {}", self.message)
    }
}

pub fn create_hello(msg: String) -> HelloWorld {
    HelloWorld::new(msg)
}
```

### Building Your Module
```bash
make build-module MODULE=my-module
```

---

## DEVELOPMENT ENVIRONMENT

### Using Docker (Recommended)
```bash
# Build dev container
docker build -t omnisystem-dev .

# Run development environment
docker run -it -v $(pwd):/workspace omnisystem-dev bash
```

### Local Setup

#### macOS
```bash
# Install dependencies
brew install llvm
brew install cmake
brew install libexecinfo

# Set environment
export PATH="/usr/local/opt/llvm/bin:$PATH"
```

#### Ubuntu/Linux
```bash
# Install dependencies
sudo apt-get install build-essential
sudo apt-get install llvm-dev
sudo apt-get install cmake

# Install Rust
curl https://sh.rustup.rs -sSf | sh
```

#### Windows
```powershell
# Install MSVC Build Tools
# Install LLVM
# Install Rust

# Add to PATH
$env:PATH += ";C:\Program Files\LLVM\bin"
```

### IDE Setup

#### VS Code
```json
{
  "extensions": [
    "rust-lang.rust",
    "eamodio.gitlens",
    "ms-vscode.makefile-tools"
  ],
  "settings": {
    "[titan]": {
      "editor.formatOnSave": true,
      "editor.defaultFormatter": "titan.format"
    }
  }
}
```

#### JetBrains IDE
```
Settings → Languages & Frameworks → TITAN
├─ TITAN SDK: /usr/local/bin/titan
├─ Format on save: ✓
└─ Run tests on save: ✓
```

---

## PROJECT STRUCTURE

```
omnisystem/
├── modules/
│   ├── base-modules/
│   │   ├── frameworks/
│   │   │   ├── neural-network/
│   │   │   ├── web/
│   │   │   ├── game/
│   │   │   ├── graphics/
│   │   │   ├── audio/
│   │   │   ├── data/
│   │   │   ├── visualization/
│   │   │   └── physics/
│   │   ├── applications/
│   │   ├── deployment/
│   │   ├── orchestration/
│   │   ├── monitoring/
│   │   └── cicd/
│   ├── omnisystem/          # Core OS modules
│   ├── bonsai-ecosystem/    # Applications
│   └── uosc/               # Microkernel
├── src/
│   ├── main.rs            # Rust HAL
│   ├── lib.rs
│   └── ...
├── tests/
│   ├── unit/
│   ├── integration/
│   └── e2e/
├── docs/
│   ├── testing/
│   ├── operations/
│   ├── development/
│   └── architecture/
├── .github/
│   └── workflows/         # CI/CD pipelines
├── Makefile
├── Cargo.toml
├── titan.toml
└── README.md
```

---

## BUILDING & COMPILING

### Build Commands

```bash
# Full build
make build

# Build with optimizations
make build-release

# Build specific module
make build-module MODULE=web

# Build and run tests
make test

# Build documentation
make docs

# Clean build artifacts
make clean
```

### Makefile Targets

```makefile
.PHONY: build test docs clean

build:
    titan compile --target modules --output bin/

test:
    titan test --target tests/ --report html

docs:
    titan docs --output docs/generated/

deploy:
    ./scripts/deploy.sh
```

### Build Configuration

```toml
# titan.toml
[project]
name = "omnisystem"
version = "1.0.0"
edition = "2026"

[dependencies]
neural-network = "1.0.0"
web-framework = "1.0.0"
data-framework = "1.0.0"

[build]
opt-level = 3
debug = false
```

---

## TESTING

### Running Tests

```bash
# Run all tests
make test

# Run specific test suite
make test-unit
make test-integration
make test-performance

# Run tests in parallel
make test-parallel

# Generate coverage report
make coverage
```

### Writing Tests

```titan
#[cfg(test)]
mod tests {
    use super::*

    #[test]
    fn test_basic_functionality() {
        let hello = HelloWorld::new("World".to_string())
        assert_eq!(hello.message, "World")
    }

    #[test]
    fn test_error_handling() {
        let result = some_operation()
        assert!(result.is_ok())
    }

    #[test]
    #[timeout(5000)]  // 5 second timeout
    fn test_performance() {
        let iterations = 1_000_000
        // Test should complete in <5s
    }
}
```

### Test Coverage

```bash
# Generate detailed coverage report
make coverage

# Coverage requirements:
# - Total: >95%
# - Functions: >90%
# - Lines: >95%
```

---

## CONTRIBUTING

### Development Workflow

```
1. Create branch
   git checkout -b feature/my-feature

2. Make changes
   - Write code
   - Write tests
   - Update docs

3. Validate locally
   make test
   make lint
   make build-release

4. Commit changes
   git commit -m "feat: Add my feature"

5. Push and create PR
   git push origin feature/my-feature
   # Create PR on GitHub

6. CI/CD pipeline runs
   - Tests run automatically
   - Code review
   - Deployment to staging

7. Merge after approval
   - Squash commits
   - Delete branch
```

### Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>

Examples:
feat(web-framework): Add WebSocket support
fix(neural-network): Fix gradient calculation bug
docs(dev-guide): Update API documentation
test(integration): Add end-to-end tests
refactor(core): Optimize module loading
```

### Pull Request Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Performance acceptable

## Checklist
- [ ] Code follows style guide
- [ ] Documentation updated
- [ ] Tests written
- [ ] No breaking changes
```

---

## CODE STANDARDS

### TITAN Code Style

```titan
// Function naming: snake_case
pub fn create_web_server(port: Int) -> WebServer {
    // Implementation
}

// Type naming: PascalCase
pub struct WebServer {
    name: String
}

// Constant naming: SCREAMING_SNAKE_CASE
pub const DEFAULT_PORT: Int = 8080

// Indentation: 4 spaces
pub fn example() {
    let x = 5
    if x > 0 {
        println!("Positive")
    }
}

// Comments: Explain WHY, not WHAT
pub fn complex_calculation(x: Int) -> Int {
    // We use this formula because it handles edge cases
    // See: https://paper.example.com/algorithm
    (x * 2) + 1
}

// Trailing comma in multi-line
let config = Config {
    name: "test",
    value: 42,
}
```

### Code Organization

```
Module Structure:
├── Types & Structs
├── Implementations
├── Public Functions
├── Private Functions
└── Tests
```

### Error Handling

```titan
// Use Result for fallible operations
pub fn risky_operation(input: String) -> Result[Output] {
    if input.is_empty() {
        return Err("Input cannot be empty".to_string())
    }

    let processed = process(input)
    Ok(processed)
}

// Propagate errors with ?
pub fn caller() -> Result[String] {
    let output = risky_operation("test")?
    Ok(output.to_string())
}
```

### Documentation

```titan
/// Creates a new web server on the specified port.
/// 
/// # Arguments
/// * `port` - The port to listen on (1-65535)
/// 
/// # Returns
/// A new WebServer instance ready to serve requests
/// 
/// # Examples
/// ```
/// let server = create_web_server(8080)
/// server.start()
/// ```
pub fn create_web_server(port: Int) -> WebServer {
    // Implementation
}
```

---

## API DOCUMENTATION

### Framework APIs

#### Neural Network Framework
```titan
let model = ModelZoo::new()
let resnet = model.load_model("resnet50")
let server = ModelServer::new(resnet, "cuda:0")
let prediction = server.predict(input_tensor)
```

#### Web Framework
```titan
let mut server = WebServer::new(8080)
server.register_route("GET", "/api/users", "handle_get_users")
server.use_middleware("cors")
server.start()
```

#### Game Framework
```titan
let mut engine = GameEngine::new("MyGame", 1920, 1080)
let player = engine.create_game_object("Player")
engine.load_scene("Level1")
engine.start()
```

#### Data Framework
```titan
let mut db = Database::new("postgresql://localhost/mydb", "PostgreSQL")
db.connect()
let results = db.query("users").where_clause("age > 18").limit(10).build()
```

See [API Reference](../api/) for complete documentation.

---

## TROUBLESHOOTING

### Common Issues

#### Module Not Found
```
Error: Module 'web' not found
Solution: 
1. Check module exists in modules/
2. Verify Cargo.toml includes module
3. Run: make build-module MODULE=web
```

#### Type Errors
```
Error: Type mismatch: expected String, found Int
Solution:
1. Check variable declaration
2. Verify function signature
3. Add explicit type: let x: String = ...
```

#### Test Failures
```
Error: Test failed with timeout
Solution:
1. Increase timeout: #[timeout(10000)]
2. Check for infinite loops
3. Reduce test complexity
```

---

## RESOURCES

- **API Documentation**: `docs/api/`
- **Architecture Guide**: `docs/architecture/`
- **Operations Guide**: `docs/operations/`
- **Testing Guide**: `docs/testing/`
- **Community Forum**: https://discuss.omnisystem.local
- **Bug Reports**: https://github.com/omnisystem/omnisystem/issues

---

**Status**: ✅ **DEVELOPER GUIDE COMPLETE**

Welcome to the Omnisystem development community! Happy coding! 🚀

