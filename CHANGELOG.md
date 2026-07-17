# Changelog

## [Unreleased]

## [0.4.1] — 2026-07-17

### Testing
- Closed remaining acceptance-criteria gaps for #11/#12/#13: `get_pseudo_style` apostrophe-escaping regression, `input`/`wait --selector` escaping re-verified after shadow-piercing was wired in, multi-file `set-files` against `<input multiple>`, and `mock`'s "no added latency" / "unchanged when never invoked" behavior — no functional changes

## [0.4.0] — 2026-07-17

### Added
- Selector resolution (`get_computed_style`, `get_pseudo_style`, `get_bounding_rect`, and everything built on them — `click`, `input`, `wait --selector`) now pierces into open shadow roots; closed shadow roots remain unreachable and are documented as a known limitation (RFC-0002, #11)
- `chromiumctl-cli set-files --selector <SEL> --files <PATHS>` — sets real files on an `<input type="file">` via CDP's `DOM.setFileInputFiles`, no base64/`File`/`DataTransfer` synthesis needed; `change` fires natively (RFC-0002, #12)
- `chromiumctl-cli mock --url-pattern <PAT> --status <CODE> --body <TEXT>` — fakes a response for matching requests via the `Fetch` domain, off by default and fully opt-in; blocks in the foreground until interrupted (RFC-0002, #13)
- `CdpClient::wait_for_event(method, timeout)` — new public primitive for receiving unsolicited CDP events (e.g. `Fetch.requestPaused`), which the existing transport had no way to do

### Fixed
- `PageEvaluator`'s default methods (`get_computed_style`, `get_pseudo_style`, `get_bounding_rect`) previously interpolated selectors into JS unescaped — a selector containing a literal `'` broke the generated JS. Now uses a shared, safe escaping helper (RFC-0002, #11)

## [0.3.0] — 2026-07-17

### Added
- `chromiumctl-cli reap [--dry-run] [--max-age <DUR>]` — closes and cleans up sessions whose `launch` caller has died or gone stale, recovering headless Chromium instances that would otherwise leak permanently (RFC-0003, #10)
- `launch --reap-stale` — opportunistically reaps other dead-caller sessions before starting a new one
- `launch` now writes a session record (port, launch time, caller PID, caller start-time fingerprint) before detaching; `stop` deletes it on successful close. Session directory defaults to `<tmp>/chromiumctl/sessions`, overridable via `CHROMIUMCTL_SESSION_DIR`

## [0.2.3] — 2026-07-12

### Fixed
- `CdpClient::launch`'s auto-assigned port now comes from an OS-assigned ephemeral port instead of a fixed-starting counter, which could collide when separate processes launched concurrently (#9)

## [0.2.2] — 2026-07-12

### Fixed
- `eval` now awaits a returned `Promise` and prints its resolved value instead of silently exiting with no output (`returnByValue` can't serialize an un-awaited `Promise`, so the result came back empty) (#8)

### Documentation
- Note in "Known limitations" that `--package` sessions can show a stale `prefers-color-scheme` relative to the device's live OS setting, since the attached WebView renderer — not chromiumctl — owns that state (#6)
- Note in "Known limitations" that `screenshot --package` only captures the WebView's own render surface, not native Activity chrome or system dialogs outside it (#5)
- Note in "Known limitations" that an `eval`-dispatched `.click()` doesn't carry trusted user-gesture activation, so gesture-gated APIs like file choosers silently no-op (#7)

## [0.2.1] — 2026-07-09

### Fixed
- `eval --output json` now preserves the JS expression's real type (boolean, number) instead of stringifying every result (#4)

## [0.2.0] — 2026-07-09

### Added
- `chromiumctl-cli` — CLI exposing the library over the command line: `launch`, `eval`, `screenshot`, `navigate`, `wait`, `click`, `input`, `get-dom`, `metrics`
- `chromiumctl-cli stop` — safely terminate exactly the browser session at `--port`/`--package` via `Browser.close`, without killing unrelated Chrome processes (#3)
- `CdpClient::attach_android` (feature `android`) — attach to a debuggable Android WebView over `adb`, forwarding a local port to the matching `webview_devtools_remote_*` socket (#2)
- `chromiumctl-cli --package <PKG>` — every session-attaching subcommand accepts `--package` as an alternative to `--port`, attaching to Android WebViews via `adb` (feature `android`)

### Changed
- Browser teardown now goes through CDP `Browser.close` instead of `Child::kill`, so `Drop`/`stop` reliably end the real browser process even when the OS-level `Child` handle refers to a launcher stub (observed on Windows)

## [0.1.0] — 2026-06-03

### Added
- `CdpClient::launch` — launch headless Chromium and connect via CDP WebSocket
- `CdpClient::attach` — attach to an already-running Chromium instance
- `CdpClient::navigate` — navigate to a URL and wait for page load
- `CdpClient::send` — send a raw CDP command
- `PageEvaluator` trait — evaluate JS, read computed CSS, get bounding rects, control viewport
- `Rect` — bounding rectangle with `overlaps` and `contains` helpers
