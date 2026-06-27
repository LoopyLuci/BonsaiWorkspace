# Contributing to UOSC

Thank you for your interest in contributing to the Universal Operating System Core (UOSC)!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/UOSC.git`
3. Create a feature branch: `git checkout -b feature/your-feature`

## Development Guidelines

### Code Style
- Follow the existing code style in the repository
- Ensure all files are properly formatted
- Use meaningful variable and function names

### Kernel Subsystems
UOSC consists of 9 core subsystems:
- **Boot** - System bootstrap and initialization
- **Memory** - Memory management and allocation
- **Scheduler** - Process scheduling and execution
- **IPC** - Inter-process communication
- **Sanctum** - Hardware isolation and security
- **Hypercall** - System call interface
- **Console** - System output and debugging
- **Timer** - Timing and interrupts
- **Proofs** - Formal verification and axioms

### Making Changes
1. Make your changes in the appropriate subsystem directory
2. Ensure your code maintains formal verification properties
3. Update documentation in `docs/` as needed
4. Test your changes thoroughly

### Commit Messages
Use clear, descriptive commit messages:
```
feat: Add new memory management feature
fix: Correct scheduler timing issue
docs: Update IPC documentation
```

## Submitting Changes

1. Push your changes to your fork
2. Create a Pull Request with a clear description
3. Reference any related issues
4. Ensure all tests pass and documentation is updated

## Testing

Before submitting a PR:
- Run any existing tests
- Verify formal proofs still hold
- Test on relevant architectures

## Questions?

Feel free to open an issue for questions or discussion.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
