# BrowserCtl

A minimal Chromium DevTools Protocol (CDP) client for Rust, and the `browse` CLI built on it.

## What

BrowserCtl is two crates:

- **`browsectl`** — a synchronous, zero-async-runtime CDP client library, published to crates.io. Launch or attach to a Chromium-based browser (Chrome, Edge, Brave, Arc, Vivaldi), evaluate JavaScript, read computed CSS, resize the viewport, navigate — over a plain WebSocket, no `tokio` required.
- **`browsectl-bin`** (installs the `browse` command) — a CLI exposing the library over the command line for shell scripts, CI, and non-Rust callers: launch/attach, eval, screenshot, click, input, file-input injection, network mocking, orphaned-session cleanup.

## Why

Most CDP tooling is either a full browser-automation framework (Puppeteer/Playwright-scale, with an async runtime and code-generated bindings for the entire protocol) or a thin wrapper that still pulls in that same weight. BrowserCtl takes the opposite approach: hand-written calls for exactly the CDP methods its consumers need, nothing generated, nothing speculative. "Minimal" describes that restraint — not a limited feature set. See [`docs/3-design/architecture.md`](docs/3-design/architecture.md) for the full rationale.

## When / How

Use `browsectl` as a Rust dependency when you need programmatic control of a real browser — screenshot testing, scraping behind JS rendering, driving a page in an integration test.

```toml
[dependencies]
browsectl = "0.5"
```

```rust
use browsectl::{CdpClient, PageEvaluator};

let mut client = CdpClient::launch("https://example.com")?;
let title = client.evaluate("document.title")?;
```

The `browse` CLI wraps the same library for shell scripts, CI, and non-Rust callers:

```sh
cargo install browsectl-bin

browse launch --url https://example.com --port 9222
browse eval --port 9222 --script "document.title"
```

Requires a Chromium-based browser installed on the machine (set `CHROME_PATH` to override auto-discovery).

## Further reading

Full crate layout, the complete API surface, the CLI command reference, exit codes, and known limitations live in [`scm/README.md`](scm/README.md) — the source-code documentation, which in turn maps to:

- [Architecture](docs/3-design/architecture.md) — layer diagram, CDP message flow, threading model, why "minimal"
- [Developer guide](docs/4-development/developer_guide.md) — build, test, adding new CDP methods

## License

MIT

---

A [Software Engineering Labs](https://swelabs.io) (SWE Labs) project.
