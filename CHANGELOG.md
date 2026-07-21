# Changelog

## [Unreleased]

## [0.5.2] — 2026-07-21

### Fixed
- `browsectl-bin`'s `no_orphan_src_files` fix (moving loose `src/` files into `core/`) activated previously-dormant `application_type = "binary"` structural checks; fixed all newly-surfaced findings:
  - Free functions in `os_process.rs`/`session_store.rs` converted to associated functions on `ProcessLocator`/`SessionStore`; `help.rs`'s `print_help`/`print_version` wrapped in a new `Help` unit struct
  - `core/session/{record,store}.rs` flattened to `core/session_record.rs` + `core/session_store.rs`
  - `CliError` demoted from `pub` to `pub(crate)` — a binary crate has no external consumers for it to be a public API to
  - Two invalid `@covers: SessionRecord` doc comments removed (the annotation only accepts function names, not struct names)

### Added
- `tests/help_int_test.rs` — dedicated coverage for `core/help.rs`'s CLI-visible output (moved out of `cli_e2e_test.rs`)
- Inline serde round-trip tests for `SessionRecord`

### Known limitations
- `structure_matches_app_type`/`api_impl_public_tests_external` (both want a full trait-contract `api/` layer for the binary crate) and a circular conflict between `shared_prefix_grouping` and `api_layer_mirrors_core_domains` for `session_record.rs`/`session_store.rs` remain open — not fixed, see commit `fbec796` for details

## [0.5.1] — 2026-07-20

### Fixed
- Removed unused `data-encoding` dependency from `browsectl`'s library manifest
- `adb_locator.rs`'s hardcoded `HOME` env-var reads replaced with `dirs::home_dir()` (new optional dependency, gated behind the `android` feature)
- 5 `unsafe` blocks in `adb_locator_e2e_test.rs` given proper `// SAFETY:` comments
- Strengthened 4 test assertions that only checked `is_ok()`/`is_some()` with no payload or boundary check
- Fixed a malformed `@covers` annotation with trailing prose breaking annotation parsing

### Added
- Real e2e test coverage for `CdpClient::set_files` (previously untested at the library level; only the CLI wrapper had coverage)

### Changed
- Test layout flattened from `tests/src/*.rs` + explicit `[[test]]` Cargo.toml entries to Cargo's auto-discovered `tests/*.rs` convention, on both `browsectl` and `browsectl-bin`
- `commands/mod.rs` split into `error.rs`/`args.rs`/`connection.rs` — `mod.rs` is now pure declarations and re-exports
- `deny.toml` moved from `scm/config/deny.toml` to `scm/deny.toml`
- Internal CLI module files renamed to noun form: `get_dom.rs` → `dom_snapshot.rs`, `set_files.rs` → `file_selection.rs` (the `get-dom`/`set-files` CLI subcommands themselves are unchanged)

## [0.5.0] — 2026-07-18

### Changed
- Workspace restructured from a single combined crate into two published packages: `browsectl` (library) and `browsectl-bin` (CLI, installs `browse`) (#14)
- Repository renamed `chromiumctl` → `browsectl` on GitHub; remote and crate metadata updated to match (#15)
- MSRV bumped to 1.97 (current latest stable)
- `CHROMIUMCTL_SESSION_DIR` → `BROWSECTL_SESSION_DIR`; matching temp-file/dir prefixes renamed to `browsectl_*`

### Added
- `browse version`/`--version` — the CLI previously had no way to report its own version

### Documentation
- Root `README.md` rewritten as intro + W3; new `scm/README.md` carries the full API/CLI reference; `architecture.md` and `developer_guide.md` (renamed from `developer-guide.md`) updated for the new crate structure (#16)
- RFC-0001/0002/0003 marked Implemented with the version that shipped them

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
