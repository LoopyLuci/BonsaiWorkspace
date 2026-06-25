# Contributing to UOSC & Omnisystem

**Guidelines for contributing to the next generation of sovereign operating systems.**

Welcome! We're excited to have contributors. This guide covers both UOSC (kernel) and Omnisystem (OS services) repositories.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Before You Start](#before-you-start)
3. [Development Workflow](#development-workflow)
4. [Code Style](#code-style)
5. [Testing](#testing)
6. [Formal Verification](#formal-verification)
7. [Commit Messages](#commit-messages)
8. [Pull Requests](#pull-requests)
9. [Documentation](#documentation)
10. [Getting Help](#getting-help)

---

## Code of Conduct

All contributors must abide by the **Contributor Covenant Code of Conduct**:

- **Be respectful** – Treat all participants with dignity and respect
- **Be inclusive** – Welcome people from all backgrounds and experience levels
- **Be constructive** – Provide thoughtful feedback and suggestions
- **Report issues** – Contact maintainers privately if you witness violations

Violations can be reported to: [conduct@bonsai-ai.org](mailto:conduct@bonsai-ai.org)

---

## Before You Start

### Discuss Large Changes

For significant features, refactoring, or architectural changes:

1. **Open an issue** describing the change
2. **Tag maintainers** (`@maintainers`) for feedback
3. **Wait for approval** before starting implementation
4. **Discuss alternatives** – there may be better approaches

### Check Existing Issues

- Search for related issues first
- Comment on existing issues rather than opening duplicates
- Vote with 👍 on issues you'd like to see fixed

### Development Setup

```bash
# Clone repository
git clone https://github.com/your-org/uosc
# OR
git clone https://github.com/your-org/omnisystem

# Create feature branch
git checkout -b feature/my-feature

# Install development dependencies
make deps

# Verify setup
make check
```

---

## Development Workflow

### 1. Create a Feature Branch

```bash
# Branch naming: feature/*, bugfix/*, docs/*, refactor/*
git checkout -b feature/my-awesome-feature

# OR for bug fixes:
git checkout -b bugfix/issue-123

# OR for documentation:
git checkout -b docs/api-reference
```

### 2. Make Changes

- Write code following the style guide (below)
- Write tests for new functionality
- Update documentation
- Run tests locally to verify

### 3. Commit Changes

See [Commit Messages](#commit-messages) section for format.

```bash
git add src/my_changes.ti
git commit -m "feat: add new capability type

Detailed description of the change..."
```

### 4. Push and Create Pull Request

```bash
git push origin feature/my-awesome-feature

# Then go to GitHub and create a PR
```

### 5. Respond to Reviews

- Address feedback promptly
- Ask questions if feedback is unclear
- Suggest alternative approaches if you disagree
- Mark conversations as resolved once addressed

### 6. Merge

Once approved and CI passes, a maintainer will merge your PR.

---

## Code Style

### Titan (Systems Language)

**Naming**:
- Variables: `snake_case`
- Functions: `snake_case`
- Types: `PascalCase`
- Constants: `SCREAMING_SNAKE_CASE`

```titan
const MAX_BUFFER_SIZE = 1024;

type CapabilityToken = struct {
    token_id: string,
    permissions: Permissions,
};

fn process_capability(cap: CapabilityToken) -> Result<(), Error> {
    // ...
}
```

**Formatting**:
- 4-space indentation
- Run `titan fmt` before committing
- Max line length: 100 characters

**Documentation**:
```titan
/// Allocate a new memory region with the given size.
/// 
/// # Arguments
/// * `size` - Size in bytes
/// 
/// # Returns
/// A CapabilityToken for the allocated region, or an error.
fn alloc_memory(size: usize) -> Result<CapabilityToken, AllocError>
```

### Sylva (Scripting Language)

**Naming**:
- Variables: `snake_case`
- Functions: `snake_case`
- Classes: `PascalCase`

```sylva
class DataProcessor:
    def process_file(self, filename: String) -> String:
        # Implementation
        return result

processor = DataProcessor()
result = processor.process_file("data.txt")
```

**Formatting**:
- 2-space indentation
- Run `sylva fmt` before committing
- Max line length: 100 characters

### Aether (Actor Language)

**Naming**:
- Actors: `PascalCase`
- Messages: `snake_case` or CamelCase
- Variables: `snake_case`

```aether
actor FileStore {
    var files: map<string, bytes>
    
    on save(path: string, data: bytes) {
        files[path] = data
    }
    
    on read(path: string) -> bytes {
        return files.get(path) or empty_bytes
    }
}
```

**Formatting**:
- 4-space indentation
- Max line length: 100 characters

### Axiom (Proof Language)

**Naming**:
- Theorems: `descriptive_name`
- Lemmas: `lemma_name`
- Definitions: `definition_name`

```ax
theorem capability_isolation:
    ∀ cap1 cap2: Capability,
    distinct(cap1, cap2) → ¬(cap1 == cap2)

lemma derived_caps_valid:
    ∀ cap parent: Capability,
    derives_from(cap, parent) → valid(cap)
```

**Formatting**:
- Mathematical notation where appropriate
- Clear variable introduction
- Comments explaining non-obvious steps

### Rust (Build Tools, FFI)

Follow standard Rust conventions:

```bash
cargo fmt
cargo clippy
```

---

## Testing

### Writing Tests

**Unit Tests** (same file as implementation):

```titan
#[test]
fn test_capability_creation() {
    let cap = CapabilityToken::new("test", Permissions::READ);
    assert_eq!(cap.permissions, Permissions::READ);
}
```

**Integration Tests** (tests/ directory):

```titan
#[test]
fn test_full_ipc_workflow() {
    // Boot kernel, create processes, test IPC
    // Takes longer than unit tests
}
```

**Property-Based Tests** (using quickcheck):

```titan
#[quickcheck]
fn property_capability_revocation(cap: CapabilityToken) -> bool {
    kernel.issue(cap);
    kernel.revoke(&cap);
    !kernel.is_valid(&cap)
}
```

### Running Tests

```bash
# Unit tests only
make test-unit

# Integration tests
make test-integration

# All tests
make test

# Specific test
make test TEST=capability

# Verbose output
make test VERBOSE=1

# With coverage
make test COVERAGE=1
```

### Test Coverage

- Aim for **>80% coverage** on new code
- 100% coverage on security-critical code (capability system, scheduler)
- Use `make coverage` to generate coverage report

---

## Formal Verification

### When Verification is Required

You **must** add an Axiom proof if you modify:
- `kernel/capability.ti` – capability system
- `kernel/scheduler.ti` – scheduler algorithm
- `services/ai-shim/*` – AI safety
- `services/ums/*` – module integrity

You **should** add a proof if you add new:
- Security-critical functionality
- Synchronization primitives
- Resource allocation algorithm

### Writing Proofs

**Example**: Prove that revocation cascades correctly

```ax
theorem revoke_cascades:
    ∀ cap parent: Capability,
    derives_from(cap, parent) ∧ revoked(parent) 
    → revoked(cap)

proof:
    assume cap parent: Capability
    assume h1: derives_from(cap, parent)
    assume h2: revoked(parent)
    -- Apply revocation rule
    apply revocation_rule (h1, h2)
    -- Proof complete
qed
```

### Verifying Proofs

```bash
# Check specific proof
axiom verify proof/capability.ax

# Check all proofs
make verify

# Verbose output (shows proof steps)
make verify VERBOSE=1
```

---

## Commit Messages

Use **Conventional Commits** format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style (formatting, missing semicolons)
- `refactor`: Code refactoring without feature/bug change
- `perf`: Performance improvement
- `test`: Adding or updating tests
- `chore`: Dependency updates, tool configuration
- `proof`: Formal verification proofs

### Examples

```bash
# Feature
git commit -m "feat(capability): add capability revocation

- Implement sys_cap_revoke syscall
- Add revocation bit to capability table
- Prove cascading revocation in Axiom

Closes #123"

# Bug fix
git commit -m "fix(scheduler): handle deadline overflow

EDF scheduler was incorrectly handling tasks with large deadlines
due to integer overflow. Add saturation arithmetic to prevent overflow.

Fixes #456"

# Documentation
git commit -m "docs: add API reference for capability syscalls

Add detailed documentation for:
- sys_cap_create()
- sys_cap_revoke()
- sys_cap_delegate()

Includes examples and return codes."

# Proof
git commit -m "proof(capability): verify isolation theorem

Add Axiom proof that capability isolation is enforced:
- No capability can be forged
- No capability can be accessed without grant
- Revocation is complete

Refs #789"
```

### Guidelines

- Keep subject line under **50 characters**
- Use imperative mood ("add" not "added" or "adds")
- Capitalize first letter of subject
- Reference issues: `Fixes #123`, `Refs #456`
- Explain **why**, not just **what** (body)
- Wrap body at **72 characters**

---

## Pull Requests

### Before Submitting

- [ ] Tests pass locally: `make test`
- [ ] Code is formatted: `make fmt`
- [ ] No linting errors: `make lint`
- [ ] Documentation updated
- [ ] Commit messages follow Conventional Commits
- [ ] No merge conflicts
- [ ] CI will likely pass

### PR Description Template

```markdown
## Description
Brief description of changes.

## Motivation and Context
Why is this change needed? What problem does it solve?

## Testing
How was this tested? Include test results.

## Types of Changes
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation

## Checklist
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Formal verification (if needed)
- [ ] Code reviewed by me

## Related Issues
Closes #123
Refs #456
```

### Responding to Review

- **Don't take feedback personally** – it's about the code, not you
- **Ask questions** if feedback is unclear
- **Suggest alternatives** if you disagree (respectfully)
- **Make requested changes** in new commits (don't rewrite history)
- **Re-request review** when done

### Merge

Once approved:
- Maintainer will squash/rebase if necessary
- PR will be merged to `main`
- Your branch will be deleted

---

## Documentation

### Code Comments

- **Minimize comments** – code should be self-explanatory
- **Add comments for "why"** – not "what" (what should be clear from code)
- **Update comments** when code changes
- **Don't comment obvious code**

```titan
// Good: explains non-obvious behavior
// EDF requires admission control to guarantee deadlines.
// We use a conservative admission test to ensure feasibility.
fn admit_edf_task(deadline: i64) -> bool {
    // ...
}

// Bad: restates what code obviously does
// Calculate the sum
let sum = a + b;
```

### Documentation Files

- Update relevant docs in `docs/` directory
- Use Markdown format
- Add to table of contents if creating new file
- Maximum **100 characters** line length for readability

### API Documentation

For public APIs, include:

```titan
/// Brief description (one line).
///
/// Longer description if needed. Explain what the function does,
/// not how it does it.
///
/// # Arguments
/// * `param1` - Description
/// * `param2` - Description
///
/// # Returns
/// Description of return value.
///
/// # Errors
/// Description of possible errors.
///
/// # Examples
/// ```titan
/// let result = my_function(42);
/// assert_eq!(result, 84);
/// ```
pub fn my_function(param1: i32) -> Result<i64, MyError>
```

---

## Getting Help

### Questions?

- **GitHub Discussions** – for general questions and discussion
- **Chat** – Join #uosc or #omnisystem on Matrix (placeholder)
- **Email** – hello@bonsai-ai.org (placeholder)

### Found a Bug?

1. Check existing issues first
2. Open a new issue with:
   - Description of the bug
   - Steps to reproduce
   - Expected vs actual behavior
   - Environment (OS, Rust version, etc.)

### Have a Feature Idea?

1. Check existing issues first
2. Open a new issue with:
   - Description of the feature
   - Use cases / motivation
   - Proposed design (if you have one)
   - Potential challenges

---

## Licensing

By contributing, you agree that your contributions will be licensed under Apache 2.0 / MIT (dual license).

---

## Recognition

Contributors will be recognized in:
- `CONTRIBUTORS.md` file in repository
- GitHub repository contributors page
- Release notes (if significant contribution)

Thank you for contributing to the future of sovereign computing! 🚀

---

**Contributing Guide Version**: 1.0.0  
**Last Updated**: 2026-06-08  
**Repositories**: UOSC, Omnisystem

