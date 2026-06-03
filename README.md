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

## Further reading

- [Architecture](docs/3-design/architecture.md) — layer diagram, CDP message flow, threading model
- [Developer guide](docs/4-development/developer-guide.md) — build, test, adding new CDP methods

## License

MIT
