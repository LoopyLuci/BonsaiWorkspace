# Omnisystem Local CI/CD Pipeline

**Everything runs locally. Zero GitHub dependency.**

This is the complete replacement for GitHub Actions workflows. All CI/CD operations execute on your device with full control and no external dependencies.

## 📋 Overview

- **Build**: Compile all 1,039+ crates with stable + nightly toolchain
- **Lint**: Code formatting (rustfmt) + linting (Clippy)
- **Test**: 4,156+ unit & integration tests with 4 parallel workers
- **Security**: Cargo audit + dependency scanning
- **Coverage**: Code coverage analysis with Tarpaulin
- **Docs**: Generate documentation for all crates
- **Deploy**: Staging and production deployments with safety checks
- **Rollback**: Emergency rollback to previous stable version

## 🚀 Quick Start

### Run Default Pipeline (build, lint, test, security, coverage, docs)
```powershell
.\Run-CI.ps1
```

### Run Specific Stages
```powershell
# Build only
.\Run-CI.ps1 -BuildOnly

# Test only
.\Run-CI.ps1 -TestOnly

# Lint only
.\Run-CI.ps1 -LintOnly

# Security audit only
.\Run-CI.ps1 -SecurityOnly

# Coverage analysis only
.\Run-CI.ps1 -CoverageOnly

# Generate documentation
.\Run-CI.ps1 -DocsOnly
```

### Run Multiple Stages
```powershell
# Custom combination
.\Run-CI.ps1 -Stage "build,lint,test"

# Full pipeline including staging deployment
.\Run-CI.ps1 -Full

# Fast incremental build + test
.\Run-CI.ps1 -Fast -Stage "build,test"
```

### Deployment
```powershell
# Deploy to staging
.\Run-CI.ps1 -DeployStagingOnly

# Deploy to production (requires manual approval)
.\Run-CI.ps1 -DeployProdOnly
```

## 📊 Stage Details

### 1. BUILD (omnisystem-ci.ti:ci_stage_build)
- Compiles workspace with stable toolchain
- Compiles workspace with nightly toolchain
- Builds all test binaries
- **Time**: ~5-15 minutes depending on system
- **Output**: `./Omnisystem/target/release/`

### 2. LINT (omnisystem-ci.ti:ci_stage_lint)
- Checks code formatting with `cargo fmt`
- Runs Clippy linter with `-D warnings`
- Reports style issues
- **Time**: ~2-5 minutes
- **Output**: Terminal report

### 3. TEST (omnisystem-ci.ti:ci_stage_test)
- Runs 4,156+ unit tests
- Runs integration tests
- Runs database tests
- Parallel execution (4 threads unit, 2 threads integration)
- **Time**: ~10-30 minutes depending on system
- **Output**: Test results + coverage

### 4. SECURITY (omnisystem-ci.ti:ci_stage_security)
- Runs `cargo audit` for vulnerability scanning
- Checks dependency tree
- Reports security issues
- **Time**: ~1-2 minutes
- **Output**: Vulnerability report

### 5. COVERAGE (omnisystem-ci.ti:ci_stage_coverage)
- Uses Tarpaulin for code coverage analysis
- Generates HTML + XML reports
- Shows coverage percentage per module
- **Time**: ~10-20 minutes
- **Output**: `./Omnisystem/tarpaulin-report.html`

### 6. DOCS (omnisystem-ci.ti:ci_stage_docs)
- Generates Rustdoc for all 1,039+ crates
- Includes private items
- Creates searchable documentation
- **Time**: ~5-10 minutes
- **Output**: `./Omnisystem/target/doc/index.html`

### 7. DEPLOY-STAGING (deployment.ti:deploy_to_staging)
- Builds release binaries
- Runs smoke tests
- Verifies service health
- **Time**: ~5-10 minutes
- **Output**: Staging ready for testing

### 8. DEPLOY-PROD (deployment.ti:deploy_to_production)
- Pre-deployment verification (all checks must pass)
- Requires manual approval to proceed
- Tracks deployment status
- **Time**: Varies (manual approval required)
- **Output**: Production deployment status

## 🔧 Architecture

### Files

```
Omnisystem/
├── ci-cd/
│   ├── omnisystem-ci.ti          # Main orchestrator (8 stages)
│   ├── deployment.ti              # Deployment + rollback logic
│   └── README.md                  # This file
├── ../Run-CI.ps1                  # PowerShell orchestrator
└── scripts/
    ├── smoke_tests.sh             # Staging verification
    ├── deploy_prod.sh             # Production deployment
    └── rollback_production.sh      # Emergency rollback
```

### Components

1. **Run-CI.ps1**: PowerShell script that orchestrates everything
   - Parses command-line arguments
   - Executes stages in sequence
   - Tracks timing and results
   - Provides colored output

2. **omnisystem-ci.ti**: TITAN module with all CI/CD logic
   - `ci_config_create()`: Configuration management
   - `ci_stage_build()`: Build orchestration
   - `ci_stage_lint()`: Code quality checks
   - `ci_stage_test()`: Test execution
   - `ci_stage_security()`: Security audits
   - `ci_stage_coverage()`: Coverage analysis
   - `ci_stage_docs()`: Documentation generation
   - `ci_run_complete_pipeline()`: Master orchestrator

3. **deployment.ti**: AETHER module with deployment logic
   - `deploy_to_staging()`: Staging deployment
   - `deploy_to_production()`: Production with approval
   - `deploy_rollback_production()`: Emergency rollback
   - `deploy_performance_test()`: Load testing

## 📈 Configuration

Edit `Run-CI.ps1` to customize:

```powershell
# Timeout settings
$testTimeoutSeconds = 3600      # 1 hour

# Parallel workers
$maxParallelJobs = 4            # Adjust for your CPU

# Failure behavior
$failFast = $true               # Stop on first failure

# Environment variables
$env:RUST_BACKTRACE = 1
$env:CARGO_TERM_COLOR = "always"
```

## ✅ Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All stages passed |
| 1 | Build or test failure |
| 2 | Lint/format failure |
| 3 | Security issue found |
| 4 | Coverage below threshold |
| 5 | Deployment approval rejected |

## 📝 Example Workflows

### Before Pushing Code
```powershell
# Verify everything locally
.\Run-CI.ps1 -Full
```

### During Development
```powershell
# Quick feedback loop
.\Run-CI.ps1 -Fast -Stage "build,test"
```

### Before Release
```powershell
# Complete verification
.\Run-CI.ps1 -Full
.\Run-CI.ps1 -DeployStagingOnly
# Manual testing in staging...
.\Run-CI.ps1 -DeployProdOnly
```

### Debugging Test Failures
```powershell
# Run specific test stage with verbose output
.\Run-CI.ps1 -TestOnly
```

## 🔒 Safety Features

### Pre-Deployment Checks
Before deploying to production, the system verifies:
- ✅ All CI stages passed
- ✅ Test coverage > 80%
- ✅ No critical security issues
- ✅ Staging environment healthy

### Manual Approval Required
Production deployment requires typing `DEPLOY TO PRODUCTION` to confirm.

### Automatic Rollback
If production deployment fails, rollback is available via:
```powershell
# Emergency rollback to previous version
.\scripts\rollback_production.sh
```

## 🚫 GitHub Actions - DISABLED

All `.github/workflows/` files have been removed:
- ❌ ci.yml (Rust CI)
- ❌ deploy.yml (Multi-service deployment)
- ❌ omnisystem-build.yml (Omnisystem build)

**Everything now runs locally on your device.**

No more waiting for GitHub to run builds. No more secrets in GitHub. No more external dependencies.

## 📊 Performance

Typical execution times (on modern hardware):
- Build: 5-15 minutes
- Lint: 2-5 minutes
- Test (4,156+ tests): 10-30 minutes
- Security: 1-2 minutes
- Coverage: 10-20 minutes
- Docs: 5-10 minutes
- **Total (Full Pipeline): ~35-80 minutes**

For faster feedback during development, use `-Fast`:
```powershell
.\Run-CI.ps1 -Fast -Stage "build,test"  # ~10-15 minutes
```

## 🆘 Troubleshooting

### Build fails
```powershell
# Clean build
cd Omnisystem
cargo clean
cargo build --workspace --release
```

### Tests fail
```powershell
# Run with backtrace
$env:RUST_BACKTRACE = "full"
.\Run-CI.ps1 -TestOnly
```

### Clippy issues
```powershell
# Auto-fix clippy warnings
cd Omnisystem
cargo clippy --fix --allow-dirty --allow-staged
```

### Coverage unavailable
```powershell
# Install tarpaulin
cargo install cargo-tarpaulin
.\Run-CI.ps1 -CoverageOnly
```

## 📚 Integration with Omnisystem

This CI/CD system uses native Omnisystem components:

- **TITAN**: Orchestration logic, shell command execution, state management
- **AETHER**: Deployment coordination, distributed deployment logic
- **SYLVA**: Potential analytics on build/test metrics
- **AXIOM**: Future formal verification of deployment safety

## 🔄 Extending the Pipeline

To add new stages, edit `omnisystem-ci.ti`:

```titan
pub fn ci_stage_custom_check(config: String) -> String {
    println("🔍 Running custom check...");
    let result = json_create_object();
    let r1 = json_object_set(result, "custom", "value");
    omnisystem_type_wrap(r1, "omnisystem/custom_results")
}
```

Then update `ci_run_complete_pipeline()` to include it.

## 📞 Support

For issues or questions about the local CI/CD system:
1. Check the troubleshooting section above
2. Review the stage output for error messages
3. Enable RUST_BACKTRACE for more details
4. Check individual stage logs in terminal output

---

**Last Updated**: 2026-06-16  
**Version**: 1.0 - Local Only  
**Status**: Production Ready
