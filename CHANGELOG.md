# Changelog

## [Unreleased]

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
