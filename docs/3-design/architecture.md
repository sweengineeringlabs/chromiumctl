# Architecture

## Overview

chromiumctl is a synchronous CDP client. Every operation — launching a browser, evaluating JavaScript, reading CSS — maps to one or more CDP messages sent over a persistent WebSocket connection. There is no async runtime; callers block on each command.

```
Your code
    │
    ▼
CdpClient  ──── WebSocket (tungstenite) ──── Chrome DevTools Protocol
    │                                               │
    │                                        Chrome / Edge / Brave
    └── PageEvaluator (trait)
         evaluate / get_computed_style / get_bounding_rect / ...
```

## Layers

The crate follows SEA (Service → Engine → Adapter) layering:

```
src/
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
    chrome_process: Option<Child>          // Some → we launched it; Drop kills it
    port:           u16
    ws_url:         String
}
```

On `Drop`, the WebSocket is closed and the child process is killed if owned.

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
