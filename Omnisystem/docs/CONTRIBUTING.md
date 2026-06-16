# Contributing to Omnisystem

**Thank you for your interest in contributing to Omnisystem!** This document provides guidelines and instructions for contributing code, documentation, and improvements to the project.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Code Standards](#code-standards)
- [Documentation](#documentation)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Review Process](#review-process)
- [Community](#community)

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inspiring community for all. Please read and adhere to our [Code of Conduct](CODE_OF_CONDUCT.md):

- **Be Respectful** - Treat all community members with respect
- **Be Inclusive** - Welcome people of all backgrounds and experience levels
- **Be Collaborative** - Work together constructively
- **Be Professional** - Keep discussions focused on the project

### Unacceptable Behavior

The following behaviors are not tolerated:
- Harassment, discrimination, or hate speech
- Trolling, personal attacks, or inflammatory language
- Unwelcome sexual attention or advances
- Doxxing or sharing private information
- Any form of abuse

**Violations will result in immediate action, including removal from the project.**

---

## Getting Started

### Prerequisites

Before contributing, ensure you have:
- **Git** - Version control (https://git-scm.com/)
- **Omnisystem** - Latest version installed
- **A GitHub account** - For pull requests
- **Text editor/IDE** - VS Code, JetBrains, or your preference

### Fork & Clone

```bash
# 1. Fork the repository on GitHub
# Click "Fork" on https://github.com/omnisystem/omnisystem

# 2. Clone your fork
git clone https://github.com/YOUR-USERNAME/omnisystem.git
cd omnisystem

# 3. Add upstream remote
git remote add upstream https://github.com/omnisystem/omnisystem.git

# 4. Create a branch for your work
git checkout -b feature/your-feature-name
```

---

## Development Setup

### Install Dependencies

```bash
# Install Omnisystem
curl https://install.omnisystem.io | sh

# Verify installation
omnisystem --version

# Install development tools
omnisystem install-dev
```

### Build from Source

```bash
# Build debug version
omnisystem build

# Build release version
omnisystem build --release

# Run tests
omnisystem test

# Run benchmarks
omnisystem bench
```

### Project Structure

```
omnisystem/
├── docs/              # Documentation (142+ files)
├── src/               # Source code
│   ├── titan/         # TITAN language
│   ├── sylva/         # SYLVA language
│   ├── aether/        # AETHER language
│   └── axiom/         # AXIOM language
├── frameworks/        # Core frameworks
│   ├── graphics/
│   ├── audio/
│   ├── physics/
│   └── game/
├── platforms/         # Creative platforms
│   ├── game-editor/
│   ├── graphic-designer/
│   ├── music-studio/
│   └── cad-modeler/
├── tools/             # Developer tools
├── tests/             # Test suites
├── examples/          # Example code
└── Cargo.toml         # Project manifest
```

---

## Making Changes

### Choose an Issue

1. **Check existing issues** - https://github.com/omnisystem/omnisystem/issues
2. **Claim an issue** - Comment "I'd like to work on this"
3. **Discuss major changes** - For large features, create an issue first

### Creating Your Branch

```bash
# Update main branch
git fetch upstream
git checkout main
git merge upstream/main

# Create feature branch
git checkout -b feature/short-description

# Or bug fix branch
git checkout -b fix/bug-description

# Or documentation branch
git checkout -b docs/documentation-topic
```

### Branch Naming Convention

- **Features:** `feature/feature-name`
- **Bugfixes:** `fix/bug-name`
- **Documentation:** `docs/doc-topic`
- **Performance:** `perf/optimization-name`
- **Refactoring:** `refactor/component-name`

---

## Code Standards

### TITAN Code Style

```titan
// Function naming: snake_case
fun calculate_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1
    let dy = y2 - y1
    (dx * dx + dy * dy).sqrt()
}

// Variable naming: snake_case
let total_sum = 0
let is_valid = true

// Type naming: PascalCase
type Point {
    x: f64,
    y: f64,
}

// Constant naming: SCREAMING_SNAKE_CASE
const MAX_BUFFER_SIZE: usize = 1024 * 1024

// Module organization
use std::collections::Vec
use graphics::{Canvas, Color}

// Comments: Use sparingly, document WHY not WHAT
fun complex_algorithm(data: &[i32]) -> i32 {
    // Use Kadane's algorithm for O(n) time complexity
    // instead of brute force O(n²) which was too slow
    mut max_sum = i32::MIN
    mut current_sum = 0
    
    for &num in data {
        current_sum = (current_sum + num).max(num)
        max_sum = max_sum.max(current_sum)
    }
    
    max_sum
}
```

### SYLVA Code Style

```sylva
// Function naming: snake_case
fun train_model(model: &mut Model, data: DataLoader) -> f32 {
    let mut total_loss = 0.0
    
    for batch in data {
        let output = model.forward(batch.x)
        let loss = mse_loss(output, batch.y)
        loss.backward()
        
        total_loss += loss.item()
    }
    
    total_loss / data.len() as f32
}

// Type naming: PascalCase
type NeuralNetwork {
    layers: Vec<Layer>,
    optimizer: Adam,
}
```

### AETHER Code Style

```aether
// Service definition: PascalCase
service KVStore {
    rpc get(key: string) -> (Value)
    rpc set(key: string, value: Value) -> ()
}

// Message types: PascalCase
enum Message {
    AppendEntries { ... },
    RequestVote { ... },
}

// Consensus algorithm: clear state management
type RaftConsensus {
    term: u64,
    voted_for: Option<string>,
    log: Vec<LogEntry>,
}
```

### AXIOM Code Style

```axiom
// Theorem naming: descriptive
theorem list_append_associative: forall l1: List(T), l2: List(T), l3: List(T),
  (l1 ++ l2) ++ l3 == l1 ++ (l2 ++ l3)

// Proof organization: clear steps
{
    intro l1, l2, l3
    induction l1 {
        case []: simp,
        case h::t: {
            rw [append_cons]
            rw [ih]
        }
    }
}
```

### General Guidelines

✅ **DO**
- Write clear, descriptive variable names
- Use type annotations in public APIs
- Add comments for non-obvious logic
- Keep functions small and focused
- Handle errors explicitly
- Write idiomatic language code

❌ **DON'T**
- Use single-letter variables (except loops: `i`, `j`, `k`)
- Ignore compiler warnings
- Create overly complex abstractions
- Skip error handling
- Write code without tests
- Copy-paste code (refactor instead)

---

## Documentation

### Documentation Standards

All contributions should include:

1. **Code comments** - Explain WHY, not WHAT
2. **Function documentation** - Purpose and parameters
3. **Examples** - Show common usage
4. **Edge cases** - Document special behaviors

### Documentation Format

```titan
// Main feature description
fun sort_array(mut arr: Vec<i32>) -> Vec<i32> {
    // Uses quicksort for O(n log n) average case
    // Handles already-sorted arrays efficiently
    
    // ... implementation ...
}

// Module documentation
module graphics {
    // GPU-accelerated 2D/3D rendering
    // Supports: Vulkan, Metal, DirectX
    // Performance: 60+ FPS
}
```

### Markdown Documentation

For documentation files:

```markdown
# Feature Title

Brief description of the feature.

## Overview

Detailed explanation of what this feature does and why it's useful.

## Quick Start

Simple example to get started quickly.

## API Reference

Complete API documentation with parameters and return values.

## Examples

Real-world usage examples.

## Best Practices

Tips for using this feature effectively.

## Next Steps

Links to related documentation.
```

### Document Locations

| Type | Location |
|------|----------|
| Getting Started | `docs/` |
| Language Guides | `docs/` |
| Framework Guides | `docs/` |
| Platform Guides | `docs/` |
| API References | `docs/` |
| Examples | `examples/` |
| Tutorials | `docs/` |

---

## Testing

### Test Organization

```titanity
#[test]
fun test_addition() {
    assert_eq!(2 + 2, 4)
}

#[test]
fun test_string_concat() {
    let s = "hello".to_string()
    s.push_str(" world")
    assert_eq!(s, "hello world")
}

#[test]
#[should_panic(expected = "overflow")]
fun test_overflow_panic() {
    panic!("overflow")
}
```

### Test Coverage

- Aim for **>80% code coverage**
- Test happy paths and error cases
- Include edge cases
- Use descriptive test names
- Keep tests focused and independent

### Running Tests

```bash
# Run all tests
omnisystem test

# Run specific test
omnisystem test test_addition

# Run with backtrace
RUST_BACKTRACE=1 omnisystem test

# Run benchmarks
omnisystem bench

# Check code coverage
omnisystem tarpaulin
```

### Benchmark Tests

```titan
use criterion::{black_box, criterion_group, criterion_main, Criterion}

fun fibonacci(n: u32) -> u32 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fun bench_fib(c: &mut Criterion) {
    c.bench_function("fib 10", |b| b.iter(|| fibonacci(black_box(10))))
}

criterion_group!(benches, bench_fib)
criterion_main!(benches)
```

---

## Submitting Changes

### Commit Messages

Follow this format:

```
type: brief description (under 70 chars)

Longer explanation of the change. Explain WHAT changed and WHY.
Include any relevant context or motivation.

References: Closes #123, Related to #456
```

### Commit Message Types

- **feat:** New feature
- **fix:** Bug fix
- **docs:** Documentation changes
- **style:** Formatting, linting
- **refactor:** Code restructuring
- **perf:** Performance improvements
- **test:** Test additions/modifications
- **chore:** Build, dependencies, etc.

### Example Commits

```
feat: Add GPU acceleration for matrix operations

Implemented CUDA kernel for dense matrix multiplication.
Provides 50x speedup on NVIDIA GPUs compared to CPU.

Benchmarks:
- 1000x1000 matrix: 2ms GPU vs 100ms CPU
- 5000x5000 matrix: 50ms GPU vs 2500ms CPU

Closes #234
```

```
fix: Correct memory leak in tensor deallocation

Previously, GPU memory wasn't properly released when
tensors went out of scope. Now uses RAII pattern.

Fixes #123
```

### Push Changes

```bash
# Keep your branch updated
git fetch upstream
git rebase upstream/main

# Push to your fork
git push origin feature/your-feature-name

# If you need to force push (after rebase only)
git push -f origin feature/your-feature-name
```

---

## Submitting a Pull Request

### Create Pull Request

1. **Go to GitHub** - Your fork on GitHub
2. **Click "New Pull Request"**
3. **Select base:** `omnisystem/omnisystem` main
4. **Select compare:** your feature branch
5. **Fill in the template** (see below)
6. **Submit pull request**

### PR Template

```markdown
## Description
Brief description of what this PR does.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation
- [ ] Performance improvement
- [ ] Refactoring

## Changes Made
- Detailed list of changes
- What was modified
- Why it was changed

## Testing
- [ ] Added unit tests
- [ ] All tests passing
- [ ] Tested locally
- [ ] No regressions

## Documentation
- [ ] Updated relevant docs
- [ ] Added code comments
- [ ] Updated examples

## Checklist
- [ ] Code follows style guidelines
- [ ] No new compiler warnings
- [ ] Self-review completed
- [ ] Comments added for complex logic
- [ ] Tests added/updated
- [ ] Documentation updated

## Related Issues
Closes #123
Related to #456
```

---

## Review Process

### Code Review Guidelines

Your PR will be reviewed by maintainers. The process:

1. **Automated Checks** - Linting, tests, coverage
2. **Maintainer Review** - Code quality, architecture
3. **Feedback** - Requested changes or approval
4. **Iteration** - Address feedback, push updates
5. **Merge** - Once approved and tests pass

### What We Look For

✅ **Code Quality**
- Follows style guidelines
- Has appropriate tests
- No compiler warnings
- Clear commit messages

✅ **Functionality**
- Solves the stated problem
- Handles edge cases
- No regressions
- Backward compatible

✅ **Performance**
- No unnecessary allocations
- Efficient algorithms
- Benchmarks if relevant
- No performance regressions

✅ **Documentation**
- Code comments for complex logic
- API documentation
- Examples if relevant
- Tests serve as documentation

### Getting Help

If review feedback is confusing:
- Ask for clarification in the PR
- Comment on specific lines
- Discuss in Discord/forums
- Tag the reviewer

---

## Community

### Communication Channels

- **GitHub Issues** - Bug reports and feature requests
- **GitHub Discussions** - Questions and discussions
- **Discord** - Real-time chat (https://discord.gg/omnisystem)
- **Forums** - Long-form discussions
- **Email** - contribute@omnisystem.io

### Getting Recognition

Contributors are recognized in:
- **CONTRIBUTORS.md** - All contributors listed
- **Release notes** - Major contributions highlighted
- **GitHub sponsors** - Option to receive donations
- **Community spotlight** - Featured contributors

### Contributor Levels

```
Contributor      - 1+ merged PR
Regular          - 10+ merged PRs
Maintainer       - High-quality contributions + review
Core Team        - Long-term leadership
```

---

## Contribution Ideas

### High-Impact Areas

1. **Language Implementation** - Core language features
2. **Framework Optimization** - Performance improvements
3. **Platform Enhancement** - New features for editors
4. **Documentation** - Guides, tutorials, examples
5. **Examples** - Real-world usage demonstrations
6. **Testing** - Test coverage improvements
7. **Tooling** - Build system, debugger, profiler
8. **Community** - Discussions, mentoring, advocacy

### Good First Issues

Look for issues tagged:
- `good-first-issue` - Suitable for new contributors
- `help-wanted` - Need community input
- `documentation` - Documentation improvements
- `beginner-friendly` - Easy to understand

---

## Frequently Asked Questions

### Q: How long does review take?
**A:** Typically 3-7 days for initial review, 1-2 days for iteration.

### Q: What if my PR is rejected?
**A:** Not rejection—feedback for improvement. Work with reviewers to address concerns.

### Q: Can I contribute to documentation?
**A:** Absolutely! Documentation is a high-value contribution.

### Q: Do I need permission to start working?
**A:** Claim an issue first by commenting. Prevents duplicate work.

### Q: Is there a code of conduct?
**A:** Yes. Be respectful, inclusive, and professional. See [Code of Conduct](CODE_OF_CONDUCT.md).

### Q: How do I report a security issue?
**A:** Email security@omnisystem.io. Do NOT open public issues.

---

## Resources

### Learning Resources

- [OMNISYSTEM_ECOSYSTEM_COMPLETE.md](OMNISYSTEM_ECOSYSTEM_COMPLETE.md) - Overview
- [INSTALLATION.md](INSTALLATION.md) - Setup guide
- [BUILD_SYSTEM_GUIDE.md](BUILD_SYSTEM_GUIDE.md) - Building from source
- [API References](docs/) - Complete API docs

### Development Tools

- [VS Code Extension](https://marketplace.visualstudio.com/items?itemName=omnisystem.omnisystem)
- [JetBrains Plugin](https://plugins.jetbrains.com/plugin/omnisystem)
- [Omnisystem CLI](docs/BUILD_SYSTEM_GUIDE.md)

### Community

- [Discord Server](https://discord.gg/omnisystem)
- [Discussion Forum](https://github.com/omnisystem/omnisystem/discussions)
- [Issue Tracker](https://github.com/omnisystem/omnisystem/issues)

---

## License

By contributing to Omnisystem, you agree that your contributions will be licensed under the same license as the project. See [LICENSE](LICENSE) for details.

---

## Thank You!

Thank you for contributing to Omnisystem! Your work makes this platform better for everyone. We appreciate:

- **Code contributions** - Features and fixes
- **Documentation** - Guides and examples
- **Bug reports** - Helping us improve quality
- **Ideas** - Feature requests and suggestions
- **Community** - Helping other users

**Happy coding!** 🚀

---

**Questions?** Open an issue or reach out on [Discord](https://discord.gg/omnisystem).

**Last Updated:** 2026-06-15
