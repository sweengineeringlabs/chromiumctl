# Developer Guide

## Prerequisites

| Requirement | Version |
|-------------|---------|
| Rust | 1.97+ (`rustup update stable`) |
| Browser | Chrome, Edge, or Brave (for integration tests) |
| curl | Any recent version (used by browser-discovery code) |

## Build

The workspace manifest lives at `scm/Cargo.toml` — run Cargo from `scm/`:

```sh
cd scm
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

Unit tests live in `#[cfg(test)]` blocks inside source files. Integration tests use the `_e2e_test.rs` suffix and live with whichever crate owns the binary/library surface they exercise: `scm/browsectl/tests/` for everything that only needs the library (including the `android`-gated `adb_locator_e2e_test`, since it drives `attach_android` directly, not the CLI), and `scm/bin/tests/cli_e2e_test.rs` for the one suite that spawns the `browse` binary (it needs `CARGO_BIN_EXE_browse`, which is only set within `browse`'s own package).

To target a specific browser:

```sh
CHROME_PATH=/opt/chromium/chrome cargo test -- --ignored --test-threads=1
```

## Adding a new CDP method

Most CDP methods are thin wrappers around `client.send()`. To expose one as a typed helper:

1. Add the method to the `PageEvaluator` trait in `scm/browsectl/main/src/api/traits/page_evaluator.rs` as a default implementation that calls `self.evaluate(js)`.
2. If the method needs a new return type (e.g. a parsed struct), define it in `scm/browsectl/main/src/api/types/`.
3. If the method cannot be expressed as a JS expression (e.g. `Network.enable`), implement it directly on `CdpClient` in `scm/browsectl/main/src/client.rs` via `self.send_cdp(method, params)`.

Example — wrapping `Page.getNavigationHistory`:

```rust
// In scm/browsectl/main/src/api/traits/page_evaluator.rs
fn get_navigation_history(&self) -> Result<serde_json::Value, String>;

// In scm/browsectl/main/src/client.rs (CdpClient impl)
fn get_navigation_history(&self) -> Result<serde_json::Value, String> {
    self.send("Page.getNavigationHistory", serde_json::json!({}))
}
```

## Environment variables

| Variable | Effect |
|----------|--------|
| `CHROME_PATH` | Override browser binary path used by `PlatformBrowserLocator::find()`. Must exist on disk — a nonexistent path is an error, not a fallback. |
| `BROWSECTL_SESSION_DIR` | Override where the CLI's session records (written by `launch`, read by `stop`/`reap`) live. Defaults to `<tmp>/browsectl/sessions`. Tests use this to avoid touching a real machine's session state. |

## Project structure walkthrough

`src/` is the leaf folder holding actual source for library/binary/example targets (`main/`, `examples/<name>/`) — those need an explicit `[[bin]]`/`[[example]]` entry in `Cargo.toml` since the path doesn't match Cargo's auto-discovery convention. `tests/` is the one exception: test files sit directly under `tests/*.rs` (no `src/` nesting, no explicit `[[test]]` entries) so Cargo auto-discovers them — this also matters for `arch`'s test-coverage checks (`all_methods_tested`, `test_covers_annotation`), which key off Cargo's own test-target discovery.

Two workspace members, both published: `browsectl` (the library) and `bin` (package `browsectl-bin`, the CLI — installs `browse`). `browsectl-bin` depends on `browsectl`, so it publishes second, after `browsectl` is live on crates.io. Each e2e test lives with whichever crate owns the `CARGO_BIN_EXE_*`/library surface it needs — see [`scm/README.md`](../../scm/README.md) for the top-level layout.

```
scm/
├── Cargo.toml                  Workspace root (members: browsectl, bin)
├── Cargo.lock
├── deny.toml                   cargo-deny config (cargo deny check --config deny.toml)
│
├── browsectl/                  Package "browsectl" — the published library
│   ├── Cargo.toml
│   ├── main/src/
│   │   ├── lib.rs              Public surface — re-exports from api/ and saf/
│   │   ├── client.rs           CdpClient impl: launch, attach, attach_android, navigate,
│   │   │                       send, WebSocket helpers, PageEvaluator impl
│   │   ├── api/
│   │   │   ├── types/cdp/
│   │   │   │   ├── cdp_client.rs          Struct definition (fields pub(crate))
│   │   │   │   └── cdp_client_builder.rs  Builder
│   │   │   ├── types/rect.rs              Rect data type
│   │   │   ├── traits/page_evaluator.rs   PageEvaluator trait + default impls
│   │   │   ├── traits/validator.rs        Validator SPI trait
│   │   │   ├── browser/browser_locator.rs BrowserLocator trait
│   │   │   ├── spi/browser_session.rs     BrowserSession SPI trait
│   │   │   └── js.rs                      deep_query_selector_js, js_string_literal
│   │   ├── core/browser/
│   │   │   └── platform_browser_locator.rs  find(), get_ws_url(), wait_for_debugger()
│   │   ├── core/android/       (feature `android`)
│   │   │   └── adb_locator.rs  AdbLocator: find adb, enumerate/match WebView sockets, forward
│   │   └── saf/mod.rs          Public constants: DEFAULT_DEBUG_PORT, viewport presets
│   ├── examples/launch/main/src/
│   │   └── main.rs             Minimal usage example ([[example]] name = "launch")
│   ├── test-support/fake-adb-for-tests/main/src/
│   │   └── main.rs             adb stand-in for adb_locator_e2e_test.rs — a [[bin]] target
│   │                           (env!("CARGO_BIN_EXE_...") only works for [[bin]], not
│   │                           [[example]]); lives here because it's what adb_locator_e2e_test
│   │                           (also in this crate) needs at CARGO_BIN_EXE_fake-adb-for-tests
│   └── tests/                  Auto-discovered by Cargo — no [[test]] entries in Cargo.toml
│       ├── client_e2e_test.rs               CdpClient lifecycle
│       ├── page_evaluator_e2e_test.rs        PageEvaluator methods
│       ├── rect_e2e_test.rs                  Rect helpers (offline)
│       ├── cdp_client_builder_e2e_test.rs    Builder
│       ├── validator_e2e_test.rs             Validator trait contract
│       ├── browser_locator_e2e_test.rs       Browser discovery
│       ├── cdp_client_e2e_test.rs            CdpClient API surface
│       ├── browser_session_e2e_test.rs       BrowserSession contract
│       ├── platform_browser_locator_e2e_test.rs  Platform discovery smoke tests
│       └── adb_locator_e2e_test.rs           attach_android (feature `android`)
│
└── bin/                        Package "browsectl-bin", published — builds binary `browse`
    ├── Cargo.toml               Depends on browsectl (version-pinned path dep, required to publish)
    ├── main/src/
    │   ├── main.rs              browse binary: pure arg dispatch, no logic
    │   ├── help.rs               print_help, print_version (static usage/version text)
    │   ├── session.rs           SessionStore: launch/stop/reap record tracking
    │   ├── os_process.rs        Caller-liveness check (tasklist/PowerShell, ps)
    │   └── commands/
    │       ├── mod.rs           Only `pub mod`/`mod`/`pub use` — no logic
    │       ├── error.rs         CliError + Display + exit_code
    │       ├── args.rs          expect_value, parse_value, validate_connect_args
    │       ├── connection.rs    attach, attach_android
    │       └── {launch,eval,...}.rs   One module per subcommand
    └── tests/                   Auto-discovered by Cargo — no [[test]] entries in Cargo.toml
        └── cli_e2e_test.rs      Every browse subcommand, end to end
```

## Commit style

```
type(scope): description

feat(client): add Page.printToPDF wrapper
fix(discovery): fall back to which on Linux when path check fails
test(page_evaluator): add e2e coverage for get_pseudo_style
```

Types: `feat`, `fix`, `test`, `refactor`, `docs`, `chore`.
