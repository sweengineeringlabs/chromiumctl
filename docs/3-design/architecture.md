# Architecture

## Overview

`browsectl` is a synchronous CDP client library. Every operation — launching a browser, evaluating JavaScript, reading CSS — maps to one or more CDP messages sent over a persistent WebSocket connection. There is no async runtime; callers block on each command.

### Why "minimal"

`browsectl` implements a hand-written wrapper for exactly the CDP methods its consumers need (currently ~11, across `Page`, `Runtime`, `DOM`, `Input`, `Emulation`, `Performance`, `Fetch`, and `Browser`) — not code-generated bindings for the full protocol, which spans hundreds of methods across roughly 40 domains. "Minimal" describes that restraint on scope and dependencies (plain `tungstenite`, no async runtime, no `tokio`), not a limited feature set: the methods it does implement are enough to fully drive a browser — navigate, evaluate, screenshot, input, file-input injection, network mocking, performance metrics. New CDP surface is added when a real consumer needs it (see [Developer guide → Adding a new CDP method](../4-development/developer_guide.md#adding-a-new-cdp-method)), not speculatively.

The `browse` CLI (package `browse`, unpublished — see [`scm/README.md`](../../scm/README.md)) is a separate crate layered on top of the `browsectl` library — it adds session tracking (`launch`/`stop`/`reap`) and CLI-level concerns (arg parsing, output formatting) but performs no CDP calls of its own that aren't exposed through the library first.

## Use cases

- **Read back computed CSS from a live render** — colours, fonts, sizes, spacing, after the full CSS cascade and JavaScript have been applied. Static analysis cannot do this; only a running browser can.
- **Drive a page programmatically** — navigate, click, fill inputs, submit forms — from a Rust process without a human at a keyboard.
- **Visual regression in CI** — load a component, measure it, assert it matches expected dimensions or styles, fail the build if it does not.
- **Test responsive behaviour** — set the viewport to a mobile width, re-measure, verify the layout changed as expected.
- **Execute JavaScript in a real browser context** — run arbitrary JS and capture the result in Rust.
- **Headless screenshot / PDF pipeline** — trigger browser-native capture via raw CDP commands.

### I/O

```
┌──────────────────────────────────────────────────────┐
│  Rust caller                                         │
│                                                      │
│  IN:  URL or debug port                              │
│       JavaScript expression strings                  │
│       CSS selector + property name                   │
│       Viewport width (u32)                           │
│       Raw CDP method + serde_json::Value params      │
│                                                      │
│  OUT: String  (JS result, CSS value)                 │
│       Rect    (x, y, width, height)                  │
│       (u32, u32)  (viewport width × height)          │
│       serde_json::Value  (raw CDP result)            │
│       String  (error message)                        │
└───────────────────────┬──────────────────────────────┘
                        │
                        ▼
┌──────────────────────────────────────────────────────┐
│  browsectl                                           │
│                                                      │
│  CdpClient::launch(url)                              │
│    1. PlatformBrowserLocator::find()  → binary path  │
│    2. Command::new(binary).spawn()   → Child process │
│    3. poll /json HTTP (curl, 200 ms) → ws_url        │
│    4. tungstenite::connect(ws_url)   → WebSocket     │
│                                                      │
│  .evaluate / .get_computed_style /                   │
│  .get_bounding_rect / .set_viewport_width / .send    │
│    serialize → JSON CDP frame → socket.send()        │
│    socket.read() loop → match id → return result     │
└──────────┬───────────────────────────────────────────┘
           │  WebSocket (port 9300+)
           ▼
┌──────────────────────────────────────────────────────┐
│  Chromium-based browser (headless)                   │
│  Chrome / Edge / Brave                               │
│                                                      │
│  Chrome DevTools Protocol                            │
│  Runtime.evaluate                                    │
│  Emulation.setDeviceMetricsOverride                  │
│  Page.navigate                                       │
│  DOM.* (incl. setFileInputFiles)  /  Input.*         │
│  Fetch.* (opt-in mocking)  /  Performance.*           │
└──────────────────────────────────────────────────────┘
```

### Shadow DOM piercing

Every default `PageEvaluator` method that resolves a CSS selector (`get_computed_style`, `get_pseudo_style`, `get_bounding_rect`, and the CLI's `click`/`input`/`wait --selector`) embeds a shared recursive JS helper (`__browsectl_deepQuerySelector`, in `api/js.rs`) instead of a bare `document.querySelector`. It descends into every *open* shadow root along the way; closed shadow roots remain unreachable, since CDP itself can't see into them without `DOM.getFlattenedDocument`. This is the default for every selector-based method — callers don't opt in.

## Layers

The crate follows SEA (Service → Engine → Adapter) layering:

```
scm/browsectl/main/src/
├── lib.rs                  Public surface (re-exports from api/)
├── client.rs               CdpClient impl blocks + send_cdp_raw
│
├── api/                    L1 — public contracts (traits and types)
│   ├── types/cdp/          CdpClient struct, CdpClientBuilder
│   ├── types/rect.rs       Rect (bounding box data type)
│   ├── traits/             PageEvaluator, Validator
│   ├── browser/            BrowserLocator trait, PlatformBrowserLocator result type
│   └── spi/                BrowserSession SPI interface
│
├── core/                   L2 — implementations
│   ├── browser/            PlatformBrowserLocator (finds Chrome/Edge/Brave on disk)
│   ├── android/            AdbLocator (feature `android`: finds adb, enumerates WebView debug sockets)
│   └── spi/                SPI slot (reserved for alternative transports)
│
└── saf/                    L3 — facade constants (viewport presets, timeout defaults)
```

### Layer rules

- `api/` defines traits and types; no implementation logic.
- `core/` implements `api/` interfaces; does not import from `saf/`.
- `saf/` exports public-facing constants; delegates everything else to `api/`.
- `lib.rs` re-exports from `api/` only.

## Key types

### `CdpClient`

Owns the WebSocket socket (`tungstenite`), an atomic message-ID counter, and optionally the `Child` process handle for a browser it launched.

Field layout:

```
CdpClient {
    socket:         Mutex<WebSocket<...>>   // serialises concurrent sends
    next_id:        AtomicU64              // monotonic CDP message ID
    chrome_process: Option<Child>          // Some → we launched it
    port:           u16
    ws_url:         String
    adb_forward:    Option<(String, u16)>  // feature `android`: Some → we own an `adb forward`
}
```

On `Drop`, if this client launched the browser, it is asked to close itself over CDP (`Browser.close`) — not killed via the `Child` handle, which on Windows can point at an already-exited launcher stub rather than the real browser process. The WebSocket is then closed and `Child::kill()` is attempted as a fallback. If this client owns an `adb` port forward (via `attach_android`), that forward is also removed.

### `PageEvaluator` trait

All DOM-query methods are default implementations built on top of `evaluate`. The only methods an implementor must provide are `evaluate` and `set_viewport_width`.

### `CdpClientBuilder`

Fluent builder that sets `CHROME_PATH` before delegating to `CdpClient::launch`. Useful when the binary path is known at build time or changes per environment.

### `Rect`

Plain data struct (`x`, `y`, `width`, `height`) returned by `get_bounding_rect`. Provides `right()`, `bottom()`, `overlaps()`, and `contains()` helpers.

## CDP message flow

```
CdpClient::send_cdp(method, params)
    │
    ├─ fetch next id  (AtomicU64)
    ├─ lock socket    (Mutex)
    └─ send_cdp_raw(socket, id, method, params)
            │
            ├─ serialize → JSON { id, method, params }
            ├─ socket.send(Text frame)
            └─ read loop
                    ├─ Text  → parse JSON, check id matches
                    │          check for "error" key
                    │          return val["result"]
                    ├─ Ping  → send Pong, continue
                    └─ Close → return Err
```

All reads are synchronous; the loop discards events with mismatched IDs (CDP push events) until the matching response arrives.

## Browser discovery

`PlatformBrowserLocator::find()` probes:

1. `CHROME_PATH` environment variable (if set and exists).
2. Well-known install paths for Chrome, Edge, and Brave on the current platform.
3. `which <candidate>` on Linux/macOS as a fallback.

`wait_for_debugger` polls `http://localhost:{port}/json` via `curl` every 200 ms until a page target with a `webSocketDebuggerUrl` appears, or the 10-second deadline is reached.

## Threading model

`CdpClient` is `Send` (all fields are `Send`). Concurrent callers are serialised by the `Mutex<WebSocket>`. There is no background thread; reads happen only inside `send_cdp_raw` on the calling thread.
