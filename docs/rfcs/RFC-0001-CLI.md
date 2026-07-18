# RFC-0001: chromiumctl CLI

**Status:** Implemented (shipped in 0.2.0)
**Date:** 2026-06-27  
**Author:** Amu Hlongwane  

## Problem

chromiumctl is currently a library-only crate. DevTools Protocol automation scenarios often need CLI access for:
- CI/CD pipelines (headless browser testing)
- Manual debugging and inspection
- Integration with shell scripts and non-Rust tooling
- Decoupled test execution (no Rust dependency in test runner)

## Proposed Solution

Add a `chromiumctl-cli` binary that exposes core DevTools capabilities via command-line interface.

## Design

### Entry Point
```bash
chromiumctl [COMMAND] [OPTIONS]
```

### Commands

#### `launch`
Launch a headless browser instance and keep it alive.
```bash
chromiumctl launch --url https://example.com \
  --headless \
  --debug-port 9222 \
  --width 1920 \
  --height 1080
```

#### `eval`
Evaluate JavaScript in a running browser session.
```bash
chromiumctl eval --port 9222 \
  --script "document.title" \
  --output json
```

#### `screenshot`
Capture page screenshot.
```bash
chromiumctl screenshot --port 9222 \
  --output screenshot.png \
  --format png \
  --full-page
```

#### `navigate`
Navigate to URL in running session.
```bash
chromiumctl navigate --port 9222 \
  --url https://example.com
```

#### `wait`
Wait for condition (selector, text, navigation).
```bash
chromiumctl wait --port 9222 \
  --selector ".loaded" \
  --timeout 10s
```

#### `click`
Click element on page.
```bash
chromiumctl click --port 9222 \
  --selector "button.submit"
```

#### `input`
Type text into input field.
```bash
chromiumctl input --port 9222 \
  --selector "input#search" \
  --text "hello"
```

#### `get-dom`
Export current DOM as JSON.
```bash
chromiumctl get-dom --port 9222 \
  --output dom.json
```

#### `metrics`
Get performance metrics.
```bash
chromiumctl metrics --port 9222 \
  --output metrics.json
```

### Output Formats
- `--output` flag supports: `json`, `yaml`, `text` (default: text)
- `--verbose` / `-v` for detailed logging

### Exit Codes
- `0` — success
- `1` — command execution failed
- `2` — invalid arguments
- `3` — timeout
- `4` — browser connection failed

## Implementation

1. Add `[[bin]]` section to Cargo.toml
2. Create `src/bin/chromiumctl-cli/main.rs` with clap-based arg parsing
3. Implement subcommand handlers wrapping existing `CdpClient` methods
4. Add integration tests verifying CLI against real browser
5. Update README with CLI usage examples

## Benefits

- ✓ CI/CD integration without Rust dependency
- ✓ Shell script automation
- ✓ Debugging and exploration
- ✓ Decoupled test runners (Python, Node.js, Go can now use chromiumctl)
- ✓ Backward compatible (library API unchanged)

## Risks

- Maintenance burden: CLI must keep pace with library updates
- Mitigation: CLI is thin wrapper around public API, auto-tested

## Alternatives

1. Separate CLI crate — adds maintenance complexity
2. JavaScript bindings — ties to Node ecosystem
3. Status quo — users write Rust for all automation

## Questions

- Should we support config files (.chromiumctl.yaml)?
- Should CLI auto-discover browser instances, or require `--port`?
- Should we support browser launching via system PATH or require explicit binary path?
