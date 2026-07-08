# Contributing to Omnisystem

**Version**: 29.0.0  
**Status**: Open for Contributions  
**Last Updated**: 2026-06-16

---

## Welcome Contributors!

Omnisystem is a comprehensive, open software ecosystem. We welcome contributions from developers, testers, documentarians, and community members.

---

## Getting Started

### 1. Clone the Repository

```bash
git clone https://github.com/omnisystem/omnisystem.git
cd omnisystem
```

### 2. Understand the Architecture

**3-Layer Structure:**
- **Layer 1**: 7 Programming Languages (TITAN, SYLVA, AETHER, VERA, HELIX, NEXUS, AXIOM)
- **Layer 2**: Core Infrastructure (System Module, UOSC, Connectors)
- **Layer 3**: Applications (OmnisystemEcosystem, Web, Mobile, AI, Services)

Read: `OMNISYSTEM_ARCHITECTURE_3_LAYER.md`

### 3. Build the Project

```powershell
cd Omnisystem/scripts/build
.\Build-All.ps1
```

See: `BUILD_INSTRUCTIONS.md`

### 4. Read the Documentation

- [Architecture Guide](OMNISYSTEM_ARCHITECTURE_3_LAYER.md)
- [Build System](Omnisystem/scripts/build/README.md)
- [Layer 3 Integration](Omnisystem/LAYER3_INTEGRATION_COMPLETE.md)

---

## How to Contribute

### Code Contributions

#### Languages (Layer 1)

To improve or extend any of the 7 languages:

1. **Directory**: `Omnisystem/languages/{language}/`
2. **Files**: Core implementation and standard libraries
3. **Guidelines**:
   - Maintain type safety
   - Ensure memory safety
   - Follow language conventions
   - Update documentation
   - Add tests

#### System Module (Layer 2)

To improve core system services:

1. **Directory**: `Omnisystem/system/`
2. **Services**:
   - Launcher (Tauri-based window management)
   - Control Panel (system settings)
   - Installer (setup infrastructure)
   - Notifications (user alerts)
   - System Tray (desktop integration)
   - File Associations (OS integration)
   - Runtime (execution environment)

3. **Guidelines**:
   - Maintain service isolation
   - Use connector gateway for IPC
   - Document APIs clearly
   - Test cross-language integration

#### UOSC (Layer 2)

To improve the operating system core:

1. **Directory**: `Omnisystem/UOSC/`
2. **Components**:
   - Kernel (boot, process management, scheduling)
   - Device Drivers (block, network, graphics, input, audio, sensors)
   - Hypercalls (system call interface)
   - Proofs (formal verification)

3. **Guidelines**:
   - Maintain kernel stability
   - Prove critical components formally
   - Document kernel APIs
   - Test with multiple drivers

#### Applications (Layer 3)

To add new applications or improve existing ones:

1. **Directory**: `Omnisystem/applications/{app-name}/`
2. **Choose Language**:
   - Desktop GUI: VERA + HELIX
   - Web: VERA
   - Mobile: NEXUS
   - ML/Data: SYLVA
   - Backend: TITAN + AETHER

3. **Guidelines**:
   - Follow application structure
   - Register with OmnisystemEcosystem
   - Use system services (Layer 2)
   - Document capabilities
   - Include tests

### Documentation Contributions

Help improve Omnisystem documentation:

1. **Architecture Docs**: `docs/00-core/`
2. **Language Guides**: `docs/02-languages/`
3. **Frameworks**: `docs/03-frameworks/`
4. **Getting Started**: `docs/01-getting-started/`
5. **Operations**: `docs/09-operations/`

**Guidelines**:
- Use clear, technical language
- Include code examples
- Add diagrams where helpful
- Update table of contents
- Verify links work

### Bug Reports

Found a bug? Please report it:

1. **Check** existing issues first
2. **Describe** the problem clearly
3. **Provide** steps to reproduce
4. **Include** system information
5. **Attach** relevant code or logs

### Feature Requests

Have a feature idea?

1. **Check** existing issues/discussions
2. **Describe** the use case
3. **Explain** why it's needed
4. **Suggest** implementation approach
5. **Link** to relevant documentation

---

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/description
# or
git checkout -b fix/issue-number
```

### 2. Make Changes

- Follow code style
- Write tests
- Update documentation
- Commit with clear messages

### 3. Test Your Changes

```powershell
# Build everything
cd Omnisystem/scripts/build
.\Build-All.ps1

# Test your changes
.\Omnisystem.exe
```

### 4. Submit a Pull Request

1. Push your branch
2. Create PR with clear description
3. Link related issues
4. Request review from maintainers
5. Address review feedback

### 5. Merge & Deploy

Once approved:
- Squash and merge (for cleaner history)
- Update version number
- Update CHANGELOG.md
- Announce changes

---

## Code Style & Standards

### Language Standards

Each language follows its own conventions. See language-specific docs:
- [TITAN Guide](Omnisystem/languages/TITAN/README.md)
- [SYLVA Guide](Omnisystem/languages/SYLVA/README.md)
- [AETHER Guide](Omnisystem/languages/AETHER/README.md)
- [VERA Guide](Omnisystem/languages/VERA/README.md)
- [HELIX Guide](Omnisystem/languages/HELIX/README.md)
- [NEXUS Guide](Omnisystem/languages/NEXUS/README.md)
- [AXIOM Guide](Omnisystem/languages/AXIOM/README.md)

### General Guidelines

- **Type Safety**: Use type annotations everywhere
- **Memory Safety**: Avoid unsafe operations
- **Error Handling**: Handle all error paths
- **Documentation**: Document public APIs
- **Testing**: Write unit and integration tests
- **Performance**: Optimize critical paths
- **Security**: Follow security best practices

### Commit Messages

Use clear, conventional commit messages:

```
feat: Add feature description
fix: Fix bug description
docs: Update documentation
refactor: Refactor code for clarity
test: Add tests for feature
perf: Improve performance of X
```

---

## Project Structure

### Core Directories

```
Omnisystem/
├── languages/              (Layer 1: 7 programming languages)
├── system/                 (Layer 2a: Core system services)
├── UOSC/                   (Layer 2b: Operating system kernel)
├── bridges/                (Layer 2c: Cross-language connectors)
├── applications/           (Layer 3: All applications)
├── docs/                   (All documentation)
├── scripts/build/          (Build system)
└── launchers/              (Built executables)
```

---

## Testing

### Running Tests

```powershell
# Language tests
cargo test -p omnisystem-languages

# Integration tests
cargo test -p omnisystem-integration

# Build tests
.\Omnisystem\scripts\build\Build-All.ps1 -Test
```

### Writing Tests

1. **Unit Tests**: Test individual functions
2. **Integration Tests**: Test component interactions
3. **System Tests**: Test end-to-end workflows
4. **Performance Tests**: Test critical paths

**Guidelines**:
- Write tests for new code
- Maintain >80% code coverage
- Test error cases
- Test edge cases
- Keep tests fast

---

## Review Process

### What We Look For

✅ **Code Quality**
- Clear, maintainable code
- Proper error handling
- Type safety

✅ **Testing**
- Adequate test coverage
- Tests pass
- Edge cases covered

✅ **Documentation**
- Changes documented
- APIs described
- Examples provided

✅ **Design**
- Fits architecture
- No breaking changes
- Backward compatible

### Timeline

1. **Submission**: PR created and linked to issue
2. **Review**: 2-3 business days for initial review
3. **Feedback**: Address review comments
4. **Approval**: Approval from 2 maintainers
5. **Merge**: Merged to main branch

---

## Community Standards

### Be Respectful

- Welcome diverse perspectives
- Listen actively
- Respond with kindness
- Resolve conflicts constructively

### Be Inclusive

- Use inclusive language
- Make content accessible
- Welcome new contributors
- Celebrate contributions

### Be Professional

- Keep discussions technical
- Focus on ideas, not people
- Provide constructive feedback
- Follow code of conduct

---

## Resources

### Documentation
- [Architecture](OMNISYSTEM_ARCHITECTURE_3_LAYER.md)
- [Build System](Omnisystem/scripts/build/README.md)
- [Language Guides](Omnisystem/languages/)
- [API Reference](docs/03-frameworks/)

### Tools
- **Build**: PowerShell (scripts/build/)
- **Version Control**: Git
- **CI/CD**: GitHub Actions
- **Testing**: Rust test framework

### Communication
- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions
- **Email**: omnisystem@example.com

---

## Release Process

### Version Numbers

Follow semantic versioning: `MAJOR.MINOR.PATCH`

### Release Checklist

- [ ] All tests pass
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version numbers updated
- [ ] Build successful
- [ ] Tag created
- [ ] Release notes published

---

## Recognition

We recognize all contributions! Contributors will be:
- Listed in CONTRIBUTORS.md
- Mentioned in release notes
- Given appropriate credit
- Invited to the team (if ongoing)

---

## Questions?

- **Documentation**: See docs/ directory
- **Technical**: Create GitHub issue
- **Process**: Email maintainers
- **General**: Start a discussion

---

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming, inclusive environment for all contributors.

### Our Standards

- Be respectful and professional
- Provide constructive feedback
- Accept criticism gracefully
- Focus on what's best for the community

### Enforcement

Violations will be addressed by the maintainers. Serious violations may result in removal from the project.

---

**Welcome to Omnisystem!**

We look forward to your contributions and helping shape the future of this software ecosystem.

---

**Version**: 29.0.0  
**Last Updated**: 2026-06-16  
**Maintained By**: Omnisystem Team
