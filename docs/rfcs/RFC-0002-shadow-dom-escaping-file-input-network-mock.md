# RFC-0002: Shadow DOM support, consistent selector escaping, file-input command, network interception

**Status:** Implemented (shipped in 0.4.0)
**Date:** 2026-07-16
**Author:** Amu Hlongwane

## Problem

Dogfooding `chromiumctl`/`chromiumctl-cli` against a real Shadow-DOM-heavy app (justjs, `sweengineeringlabs/justjs`) surfaced four gaps, all confirmed by reading the source and by live reproduction, not just CLI trial-and-error:

1. **Shadow DOM blindness.** `PageEvaluator`'s default methods (`get_bounding_rect`, `get_computed_style`, `get_pseudo_style` in `main/src/api/traits/page_evaluator.rs`) resolve selectors via plain `document.querySelector(selector)`. This cannot reach into any shadow root — open or closed. Every `click`, `wait`, `get_computed_style`-backed command, and (transitively) every `csslense` lens is blind to anything rendered inside a Shadow DOM custom element. Confirmed live: `document.querySelector('.toggle-btn')` against a real shadow-rooted `<view-toggle>` button returns null, forcing every interaction in this session to fall back to hand-written `eval` + `element.shadowRoot.querySelector(...)`.

2. **Inconsistent selector escaping.** `bin/chromiumctl-cli/main/src/commands/input.rs` properly escapes its selector via `serde_json::to_string(&selector)` before interpolating into the evaluated JS. But `PageEvaluator`'s default trait methods (used by `click.rs` directly, and inherited by every `csslense` lens that doesn't wrap with its own escaping) build the JS via raw `format!("...'{}'...", selector)` — unescaped. A selector containing a literal `'` breaks the generated JS, or worse, is a JS-injection vector if the selector ever comes from untrusted input.

3. **No file-input command.** None of `click`/`eval`/`get-dom`/`input`/`launch`/`metrics`/`navigate`/`screenshot`/`stop`/`wait` can set `<input type="file">.files`. Every screenshot-attach flow tested this session required a hand-written `eval` synthesizing a `File`/`DataTransfer` and dispatching a `change` event manually.

4. **No network interception/mocking.** No `Fetch.*` or `Network.enable` CDP domain is used anywhere in the codebase. Confirmed live: a form submission with fake AWS credentials reached the real, unmediated `sts.amazonaws.com`-equivalent endpoint and returned a genuine `InvalidClientTokenId` error — there is no way today to intercept that call and fake a response instead (e.g. to test a *successful*-connect code path without real credentials).

## Proposed Solution

### 1. Shadow-piercing selector resolution

Add a shared JS helper (used by every `PageEvaluator` default method) that recursively descends through open shadow roots, e.g.:

```js
function deepQuerySelector(root, selector) {
  const direct = root.querySelector(selector);
  if (direct) return direct;
  const hosts = root.querySelectorAll('*');
  for (const host of hosts) {
    if (host.shadowRoot) {
      const found = deepQuerySelector(host.shadowRoot, selector);
      if (found) return found;
    }
  }
  return null;
}
```

Swap `document.querySelector('{}')` for `deepQuerySelector(document, '{}')` in `get_bounding_rect`, `get_computed_style`, `get_pseudo_style`, and in `commands/click.rs`/`commands/input.rs`/`commands/wait.rs`. Closed shadow roots remain unreachable (by design — CDP itself can't see them without `DOM.getFlattenedDocument` tricks that carry their own cost); scope this RFC to open roots, which is what justjs and most real component libraries use.

### 2. Consistent escaping

Extract `input.rs`'s `serde_json::to_string(&selector)` pattern into a shared helper (e.g. `fn js_string_literal(s: &str) -> String`) and use it in every command/trait method that interpolates a selector into JS. No command should hand-roll `format!("'{}'", selector)` again.

### 3. `set-files` command

```bash
chromiumctl set-files --port 9222 --selector "#file-input" --files "./screenshot.png,./doc.pdf"
```
Reads each file from disk, base64-encodes it, and runs the same `File`/`DataTransfer`/`dispatchEvent('change')` sequence this session's `eval` workaround used — but as a first-class, tested command instead of ad-hoc JS every caller has to reinvent.

### 4. Network interception (opt-in)

```bash
chromiumctl mock --port 9222 --url-pattern "*sts.amazonaws.com*" --status 200 --body '{"fake":"response"}'
```
Backed by `Fetch.enable` + `Fetch.fulfillRequest`. Off by default (real calls remain real, matching this session's finding that real-network behavior is itself valuable signal) — only intercepts patterns explicitly registered via `mock`.

## Implementation

1. Add `deep_query_selector.js` (or inline JS constant) shared by all `PageEvaluator` default methods; update `click.rs`/`input.rs`/`wait.rs` to use it too.
2. Add `js_string_literal()` helper; replace every raw selector interpolation site.
3. Add `commands/set_files.rs` + `SetFiles` handling in `PageEvaluator`/`CdpClient`.
4. Add `commands/mock.rs`, wire `Fetch.enable`/`Fetch.fulfillRequest` through `CdpClient`.
5. Add e2e tests for all four against a real Shadow DOM + real file-input + real network fixture page (`tests/src/`, matching the existing `*_e2e_test.rs` convention).
6. Update `docs/rfcs/RFC-0001-CLI.md`'s command list and this crate's top-level docs once shipped.

## Benefits

- ✓ Every `csslense` lens gains Shadow DOM reach for free (no changes needed in csslense itself, since it goes through `PageEvaluator`)
- ✓ Closes a real, reproduced JS-injection-shaped bug in unescaped selector interpolation
- ✓ Screenshot/file-attach testing no longer requires hand-written CDP JS in every caller
- ✓ Opt-in network mocking enables testing success paths that require real, hard-to-obtain credentials

## Risks

- Shadow-piercing `querySelectorAll('*')` walks are O(n) per call on large DOMs — acceptable for test/CLI use, worth a perf note in docs, not a blocker.
- `mock` introduces a new CDP domain (`Fetch`) with its own lifecycle (must resume/continue non-matched requests) — real implementation complexity, should ship after (1)-(3), not bundled in the same release.
- Closed shadow roots remain unreachable — should be documented as an explicit limitation, not silently unsupported.

## Alternatives

1. Push Shadow DOM piercing into `csslense` only, leaving `chromiumctl` as-is — rejected, since `click`/`wait`/raw `eval`-adjacent commands need it too, and every other consumer of `PageEvaluator` would have to reinvent the same fix.
2. Require closed-shadow support via `DOM.getFlattenedDocument` — deferred, real cost/complexity not justified by anything observed in this dogfooding pass (justjs uses open roots throughout).

## Questions

- Should shadow-piercing be the default, or an opt-in flag (`--pierce-shadow`) for backward compatibility with callers relying on light-DOM-only matching today?
- Should `mock` support pattern-based dynamic responses (e.g. templated by request body), or is exact URL + fixed response enough for v1?
- Does `set-files` need multi-file-input support (`<input multiple>`), or is single-file sufficient for now?
