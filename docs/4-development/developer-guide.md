# Developer Guide

## Prerequisites

| Requirement | Version |
|-------------|---------|
| Rust | 1.75+ (`rustup update stable`) |
| Browser | Chrome, Edge, or Brave (for integration tests) |
| curl | Any recent version (used by browser-discovery code) |

## Build

```sh
cargo build
```

No code generation, no proc-macros beyond `serde`. Cold build takes ~10 seconds.

## Tests

```sh
# Unit and offline tests (fast, no browser required)
cargo test --lib

# All integration and e2e tests (browser must be installed)
cargo test -- --ignored --test-threads=1
```

Unit tests live in `#[cfg(test)]` blocks inside source files. Integration tests are in `tests/` and use the `_e2e_test.rs` suffix.

To target a specific browser:

```sh
CHROME_PATH=/opt/chromium/chrome cargo test -- --ignored --test-threads=1
```

## Adding a new CDP method

Most CDP methods are thin wrappers around `client.send()`. To expose one as a typed helper:

1. Add the method to the `PageEvaluator` trait in `src/api/traits/page_evaluator.rs` as a default implementation that calls `self.evaluate(js)`.
2. If the method needs a new return type (e.g. a parsed struct), define it in `src/api/types/`.
3. If the method cannot be expressed as a JS expression (e.g. `Network.enable`), implement it directly on `CdpClient` in `src/client.rs` via `self.send_cdp(method, params)`.

Example — wrapping `Page.getNavigationHistory`:

```rust
// In src/api/traits/page_evaluator.rs
fn get_navigation_history(&self) -> Result<serde_json::Value, String>;

// In src/client.rs (CdpClient impl)
fn get_navigation_history(&self) -> Result<serde_json::Value, String> {
    self.send("Page.getNavigationHistory", serde_json::json!({}))
}
```

## Environment variables

| Variable | Effect |
|----------|--------|
| `CHROME_PATH` | Override browser binary path used by `PlatformBrowserLocator::find()` |

## Project structure walkthrough

```
src/client.rs               CdpClient impl: launch, attach, navigate, send,
                            WebSocket helpers, PageEvaluator impl

src/api/
  types/cdp/cdp_client.rs   Struct definition (fields are pub(crate))
  types/cdp/cdp_client_builder.rs  Builder
  types/rect.rs             Rect data type
  traits/page_evaluator.rs  PageEvaluator trait with default method impls
  traits/validator.rs       Validator SPI trait
  browser/browser_locator.rs  BrowserLocator trait
  spi/browser_session.rs    BrowserSession SPI trait

src/core/browser/
  platform_browser_locator.rs  find(), get_ws_url(), wait_for_debugger()

src/saf/mod.rs              Public constants: DEFAULT_DEBUG_PORT, viewport presets

tests/
  client_e2e_test.rs        CdpClient lifecycle (launch, attach, navigate, send)
  page_evaluator_e2e_test.rs  PageEvaluator methods
  rect_e2e_test.rs          Rect helpers (offline)
  cdp_client_builder_e2e_test.rs  Builder
  validator_e2e_test.rs     Validator trait contract
  browser_locator_e2e_test.rs  Browser discovery
  cdp_client_e2e_test.rs    CdpClient API surface
  browser_session_e2e_test.rs  BrowserSession contract
  platform_browser_locator_e2e_test.rs  Platform discovery smoke tests
```

## Running the structural audit

```sh
arch audit .
```

This runs `struct-engine` and checks 168 SEA architecture rules. The one persistent failure is rule 71 (`.git` is flagged as an unexpected root entry — a tool bug).

## Commit style

```
type(scope): description

feat(client): add Page.printToPDF wrapper
fix(discovery): fall back to which on Linux when path check fails
test(page_evaluator): add e2e coverage for get_pseudo_style
```

Types: `feat`, `fix`, `test`, `refactor`, `docs`, `chore`.
