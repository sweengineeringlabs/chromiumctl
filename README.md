# chromiumctl

Minimal Chromium DevTools Protocol client for Rust. Synchronous, zero-async, no runtime dependency. Works with Chrome, Edge, Brave, Arc, and Vivaldi.

## Install

```toml
[dependencies]
chromiumctl = "0.1"
```

Requires a Chromium-based browser installed on the machine. Set `CHROME_PATH` to override auto-discovery.

## Usage

```rust
use chromiumctl::{CdpClient, PageEvaluator};

// Launch headless Chrome and navigate to a URL
let mut client = CdpClient::launch("https://example.com")?;

// Evaluate JavaScript — returns a String
let title = client.evaluate("document.title")?;

// Read computed CSS
let color = client.get_computed_style("h1", "color")?;

// Get element bounding rect
let rect = client.get_bounding_rect(".hero")?;
println!("width={} height={}", rect.width, rect.height);

// Resize viewport (uses Emulation.setDeviceMetricsOverride — actually changes it)
client.set_viewport_width(375)?;

// Navigate and wait for load
client.navigate("https://example.com/about")?;

// Send any raw CDP command
let result = client.send("Runtime.evaluate", serde_json::json!({
    "expression": "performance.now()",
    "returnByValue": true,
}))?;
```

### Attach to a running browser

```rust
// Start Chrome with: --remote-debugging-port=9222
let client = CdpClient::attach(9222)?;
```

### Builder

```rust
let client = CdpClientBuilder::new("https://example.com")
    .chrome_bin("/opt/chromium/chrome")
    .port(9300)
    .launch()?;
```

## API surface

| Item | What it does |
|------|-------------|
| `CdpClient::launch(url)` | Launch headless browser, navigate to `url`, connect |
| `CdpClient::attach(port)` | Attach to an existing debugger on `port` |
| `client.navigate(url)` | Navigate and wait for page load (10 s timeout) |
| `client.send(method, params)` | Raw CDP command, returns `result` JSON |
| `client.port()` | The remote-debugging port |
| `client.ws_url()` | The WebSocket debugger URL |
| `PageEvaluator::evaluate(js)` | Run JS, return string |
| `PageEvaluator::get_computed_style(sel, prop)` | Computed CSS value |
| `PageEvaluator::get_pseudo_style(sel, pseudo, prop)` | Pseudo-element CSS |
| `PageEvaluator::get_bounding_rect(sel)` | `Rect { x, y, width, height }` |
| `PageEvaluator::set_viewport_width(px)` | Resize viewport |
| `PageEvaluator::get_viewport_size()` | `(width, height)` in pixels |

## CLI

`chromiumctl-cli` exposes the library over the command line for shell scripts, CI, and non-Rust callers. Build it with:

```sh
cargo build --release --bin chromiumctl-cli
```

### `launch` — start a browser and detach

Spawns headless Chromium, connects, then detaches: the browser keeps running after the CLI process exits, so later commands can `--port` into it.

```sh
chromiumctl-cli launch --url https://example.com --port 9222 --width 1920 --height 1080
```

### Commands that attach to a running session with `--port`

```sh
chromiumctl-cli eval       --port 9222 --script "document.title" --output json
chromiumctl-cli navigate   --port 9222 --url https://example.com/about
chromiumctl-cli wait       --port 9222 --selector ".loaded" --timeout 10
chromiumctl-cli click      --port 9222 --selector "button.submit"
chromiumctl-cli input      --port 9222 --selector "input#search" --text "hello"
chromiumctl-cli screenshot --port 9222 --output page.png --full-page
chromiumctl-cli get-dom    --port 9222 --output dom.json
chromiumctl-cli metrics    --port 9222 --output metrics.json
```

`eval --output` selects the stdout format (`text`, `json`, `yaml` — default `text`). For `screenshot`/`get-dom`/`metrics`, `--output <FILE>` is a destination path; omit it on `get-dom`/`metrics` to print JSON to stdout instead.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | success |
| 1 | command executed but failed (JS exception, element not found) |
| 2 | invalid or missing arguments |
| 3 | operation timed out (`wait`) |
| 4 | could not connect to (or launch) the browser |

### Known limitations

- Chromium is always launched headless (`--headless=new`) — the `--headless` flag is accepted for RFC-0001 compatibility but has no effect; there is currently no way to launch headed via this library.
- `eval --output yaml` emits a single `result: <value>` line, not general YAML serialization — it's only ever used to render one string field.
- On Windows, if a launched browser never becomes reachable (e.g. `wait_for_debugger` times out), the spawned process may be left running: Chrome's own launcher process re-execs and exits almost immediately, so the `Child` handle `CdpClient` holds doesn't correspond to the real, long-lived browser process, and `Child::kill()` on it is a no-op. Normal launch → use → drop is unaffected — `Drop` closes the browser over CDP itself (`Browser.close`) rather than relying on that handle — this only affects the rare case where the browser never came up in the first place.

## Further reading

- [Architecture](docs/3-design/architecture.md) — layer diagram, CDP message flow, threading model
- [Developer guide](docs/4-development/developer-guide.md) — build, test, adding new CDP methods

## License

MIT
