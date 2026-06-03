# Changelog

## [Unreleased]

## [0.1.0] — 2026-06-03

### Added
- `CdpClient::launch` — launch headless Chromium and connect via CDP WebSocket
- `CdpClient::attach` — attach to an already-running Chromium instance
- `CdpClient::navigate` — navigate to a URL and wait for page load
- `CdpClient::send` — send a raw CDP command
- `PageEvaluator` trait — evaluate JS, read computed CSS, get bounding rects, control viewport
- `Rect` — bounding rectangle with `overlaps` and `contains` helpers
