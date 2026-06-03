#!/usr/bin/env bash
set -euo pipefail

echo "chromiumctl bootstrap"
echo "Requires: Rust 1.75+, a Chromium-based browser for integration tests"

cargo build
cargo test --lib
echo "Bootstrap complete. Run 'cargo test -- --ignored --test-threads=1' for live-browser tests."
