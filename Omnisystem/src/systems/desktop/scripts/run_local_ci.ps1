# Run local CI: builds workspace, signs artifacts and uploads to CAS if provided via env
# Usage:
#   $env:OMNISYSTEM_CAS_DB = "C:\path\to\cas.db"; $env:OMNISYSTEM_CAS_BLOB_DIR = "C:\path\to\blobs"; cargo run -p omnisystem-ci --bin ci_local_runner

param()

Write-Output "Running local Omnisystem CI"

# You can run the runner directly via cargo
Write-Output "To run: cargo run -p omnisystem-ci --bin ci_local_runner"

# If you want this script to run the runner for you, uncomment below:
# cargo run -p omnisystem-ci --bin ci_local_runner
