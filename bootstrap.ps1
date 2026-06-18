Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

Write-Host "chromiumctl bootstrap"
Write-Host "Requires: Rust 1.75+, a Chromium-based browser for integration tests"

Push-Location scm
cargo build
cargo test --lib
Pop-Location
Write-Host "Bootstrap complete. Run 'cd scm && cargo test -- --ignored --test-threads=1' for live-browser tests."
