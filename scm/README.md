# Source

This is the Cargo workspace for BrowserCtl — see the [root README](../README.md) for what/why/how. This file documents the crate layout and the full API/CLI surface; for the design rationale and the development workflow, see:

- [Architecture](../docs/3-design/architecture.md)
- [Developer guide](../docs/4-development/developer_guide.md)

## Crate layout

```
scm/
├── Cargo.toml       workspace manifest (members: cdp-client, bin)
├── config/          deny.toml (license/advisory policy)
├── cdp-client/       lib crate "cdp-client" (Rust name `cdp_client`)
│   ├── main/src/     library source
│   ├── examples/     launch — minimal launch+evaluate example
│   ├── test-support/ fake-adb-for-tests — stand-in `adb` binary for Android e2e tests
│   └── tests/src/    e2e tests exercising only the library
└── bin/              bin crate "browsectl", installs the `browse` command
    ├── main/src/      CLI source
    └── tests/src/     cli_e2e_test — the one suite that needs CARGO_BIN_EXE_browse
```

Each e2e test lives with whichever crate owns the `CARGO_BIN_EXE_*` binary it depends on — `CARGO_BIN_EXE_<name>` is only set within a binary target's own package, not for a crate that merely depends on it.

## Library (`cdp-client`)

```toml
[dependencies]
cdp-client = "0.4"
```

Requires a Chromium-based browser installed on the machine. Set `CHROME_PATH` to override auto-discovery.

```rust
use cdp_client::{CdpClient, PageEvaluator};

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

### Android WebView (feature `android`)

```toml
[dependencies]
cdp-client = { version = "0.4", features = ["android"] }
```

```rust
// Requires `adb` (ADB_PATH env var, or Android SDK platform-tools on a
// well-known path or PATH) and a device/emulator with
// WebView.setWebContentsDebuggingEnabled(true) active for the given package.
let client = CdpClient::attach_android("com.example.app")?;
```

Enumerates active `webview_devtools_remote_*` debug sockets over `adb shell`, matches one against the package name, forwards a local port to it, and attaches like `CdpClient::attach`. The port forward is torn down automatically on drop.

### API surface

| Item | What it does |
|------|-------------|
| `CdpClient::launch(url)` | Launch headless browser, navigate to `url`, connect |
| `CdpClient::attach(port)` | Attach to an existing debugger on `port` |
| `CdpClient::attach_android(package)` | Attach to a debuggable Android WebView via `adb` (feature `android`) |
| `client.navigate(url)` | Navigate and wait for page load (10 s timeout) |
| `client.send(method, params)` | Raw CDP command, returns `result` JSON |
| `client.wait_for_event(method, timeout)` | Block for an unsolicited CDP event (e.g. `Fetch.requestPaused`), return its `params` |
| `client.set_files(sel, paths)` | Set `<input type="file">.files` via `DOM.setFileInputFiles` (real files, no synthesis) |
| `client.port()` | The remote-debugging port |
| `client.ws_url()` | The WebSocket debugger URL |
| `PageEvaluator::evaluate(js)` | Run JS, return string |
| `PageEvaluator::get_computed_style(sel, prop)` | Computed CSS value |
| `PageEvaluator::get_pseudo_style(sel, pseudo, prop)` | Pseudo-element CSS |
| `PageEvaluator::get_bounding_rect(sel)` | `Rect { x, y, width, height }` |
| `PageEvaluator::set_viewport_width(px)` | Resize viewport |
| `PageEvaluator::get_viewport_size()` | `(width, height)` in pixels |

## CLI (`browsectl`, installs `browse`)

Build it with (from `scm/`, where the workspace manifest lives):

```sh
cargo build --release --bin browse
```

or install it:

```sh
cargo install --path bin
```

### `launch` — start a browser and detach

Spawns headless Chromium, connects, then detaches: the browser keeps running after the CLI process exits, so later commands can `--port` into it.

```sh
browse launch --url https://example.com --port 9222 --width 1920 --height 1080
```

### `stop` — safely end a `launch`ed session

`launch` detaches on purpose (see above), so cleanup is a separate step. Kill it by image name (`taskkill /IM chrome.exe`, `pkill chrome`) and you'll take down every other Chrome window on the machine along with it. `stop` instead attaches to exactly the session at `--port`/`--package` and closes it over CDP (`Browser.close`), leaving everything else untouched:

```sh
browse stop --port 9222
```

### `reap` — clean up sessions whose caller never called `stop`

`launch` writes a small session record (port, launch time, and the PID of whatever process invoked `launch`) so a session can still be found and closed even if its caller crashes, gets killed, or times out before reaching `stop`. `reap` scans those records and closes/deletes the ones whose caller is no longer alive — a session whose caller is still running is left untouched:

```sh
browse reap --dry-run       # list what would be reaped, without acting
browse reap                 # close and clean up every orphaned session
browse reap --max-age 1h    # also reap sessions older than 1h, even with a live caller
```

`launch --reap-stale` opportunistically runs the caller-liveness sweep (no `--max-age`) before starting its own session, bounding worst-case growth without needing a standing watchdog process:

```sh
browse launch --url https://example.com --port 9223 --reap-stale
```

Session records live under `<tmp>/browsectl/sessions` by default; set `BROWSECTL_SESSION_DIR` to use a different location (e.g. to isolate session state per sandbox or CI job).

### `mock` — fake a network response for matching requests

Intercepts requests whose URL matches a glob pattern and fulfills them with a fake status/body instead of hitting the real network — e.g. to exercise a success code path against a third-party API without real credentials. Off by default: nothing is intercepted unless you run `mock`, and only requests matching `--url-pattern` are ever touched — everything else reaches its real destination exactly as if `mock` weren't running.

```sh
browse mock --port 9222 --url-pattern "*sts.amazonaws.com*" --status 200 --body '{"fake":"response"}'
```

Unlike every other subcommand, `mock` **blocks** — it keeps intercepting matching requests until you interrupt it (Ctrl-C) or an hour passes with no matching traffic at all. Run it in a separate terminal (or background job) while driving the rest of your session against the same `--port` from elsewhere. The fulfilled response always includes `Access-Control-Allow-Origin: *`, since a cross-origin request (the motivating case — mocking a third-party API) would otherwise fail CORS even though the response body itself is exactly what you asked for.

### Commands that attach to a running session with `--port`

```sh
browse eval       --port 9222 --script "document.title" --output json
browse navigate   --port 9222 --url https://example.com/about
browse wait       --port 9222 --selector ".loaded" --timeout 10
browse wait       --port 9222 --navigation --timeout 10  # or --text "some content"
browse click      --port 9222 --selector "button.submit"
browse input      --port 9222 --selector "input#search" --text "hello"
browse set-files  --port 9222 --selector "#file-input" --files "./a.png,./b.pdf"
browse screenshot --port 9222 --output page.png --full-page
browse get-dom    --port 9222 --output dom.json
browse metrics    --port 9222 --output metrics.json
browse version
```

`eval --output` selects the stdout format (`text`, `json`, `yaml` — default `text`). For `screenshot`/`get-dom`/`metrics`, `--output <FILE>` is a destination path; omit it on `get-dom`/`metrics` to print JSON to stdout instead. `set-files --files` is a comma-separated list of paths on disk (relative paths resolve against `browse`'s own current directory); each file is validated to exist before anything is sent to the browser, and Chromium reads the file itself over CDP (`DOM.setFileInputFiles`) — no base64 encoding, and the target `<input type="file">`'s `change` event fires natively, same as a real user picking a file.

### Attaching to Android instead of `--port` (feature `android`)

Every command above accepts `--package <PKG>` as an alternative to `--port` — attaches to a debuggable Android WebView via `adb` instead of a desktop debug port (see [Android WebView](#android-webview-feature-android) above for prerequisites). `--port` and `--package` are mutually exclusive.

```sh
cargo build --release --bin browse --features android
browse eval --package com.example.app --script "document.title"
```

Built without the `android` feature, `--package` is still recognized but returns a clear error telling you to rebuild with it, rather than an opaque "unknown option".

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
- Via the CLI, if the process that ran `browse launch` dies (crash, `kill`, CI cancellation, timeout) before ever calling `stop`, the browser it started is left running with nothing tracking it except a session record. Run `browse reap` (or `launch --reap-stale`) periodically — e.g. as a CI teardown step — to close and clean up those orphans; see the [`reap`](#reap--clean-up-sessions-whose-caller-never-called-stop) section above. `reap`'s caller-liveness check on Windows shells out to `tasklist`/PowerShell and on Unix to `ps` — no OS process API exists in `cdp-client` itself, so this is best-effort like the rest of the CLI's process handling.
- `--package` sessions can show a stale `prefers-color-scheme` (and possibly other media-query-driven rendering) relative to the device's actual live OS setting. `cdp-client` queries Blink fresh on every command — it holds no cached state of its own — so this traces back to the attached WebView's renderer, which typically only picks up `Configuration` changes (like a system dark-mode toggle) when its host app explicitly propagates them; a `--package` attach just observes whatever that renderer already has. If a media-query-dependent screenshot or `eval` result looks wrong, cross-check against a real `adb shell screencap` before assuming the page itself is broken.
- `screenshot --package` only captures the WebView's own rendered surface, not the full device screen — it's a `Page.captureScreenshot` of that page's compositor output, nothing more. Native Activity chrome (an `ActionBar`, a system dialog, a native file picker triggered by `onShowFileChooser`) is invisible to it, since those aren't part of the WebView's own render tree. For anything outside the page content, use `adb shell screencap` for a real full-device capture instead.
- Selector resolution (`get_computed_style`, `get_pseudo_style`, `get_bounding_rect`, and everything built on them — `click`, `input`, `wait --selector`) pierces into *open* shadow roots but not *closed* ones (`attachShadow({mode: 'closed'})`). CDP itself can't see into a closed shadow root without `DOM.getFlattenedDocument`, which this crate doesn't use. An element that only exists inside a closed shadow root will report as not found, the same as if it didn't exist at all.
- A synthetic `element.click()` dispatched via `eval` doesn't carry real user-gesture trust, so gesture-gated browser APIs — likely fullscreen and clipboard-write, confirmed for `<input type="file">`'s native file-picker dialog — silently no-op: no error, no exception, no event. This is inherent to how Chromium's `Runtime.evaluate`-injected execution is, by design, not treated as trusted user activation; it isn't something `cdp-client` can or should work around. For `<input type="file">` specifically, use `set-files` instead — it sets `.files` directly via CDP (`DOM.setFileInputFiles`), sidestepping the native file-picker dialog entirely rather than trying to trigger it. For other gesture-gated interactions, drive a real input event instead (e.g. `adb shell input tap` at the element's on-screen coordinates).
