// E2e tests for PageEvaluator trait methods exercised via CdpClient.
//
// All tests require a Chromium-based browser. Run with:
//   cargo test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used, clippy::expect_used)]

use browsectl::{CdpClient, PageEvaluator};

fn fixture_url() -> &'static str {
    "data:text/html,<html><head><style>.box{width:100px;height:50px;background:red}</style></head>\
     <body><div class='box' id='b'>text</div></body></html>"
}

/// A page with an *open* shadow root containing an element that plain
/// `document.querySelector` cannot see — only `deepQuerySelector` can.
fn shadow_fixture_url() -> &'static str {
    r#"data:text/html,<html><body>
        <div id="host"></div>
        <script>
            var host = document.getElementById('host');
            var root = host.attachShadow({mode: 'open'});
            root.innerHTML = '<div class="shadow-box" style="width:120px;height:60px;display:block">in shadow</div>';
        </script>
    </body></html>"#
}

/// A page with an element whose only matching selector requires an
/// attribute-value selector containing a literal `'` — the exact shape
/// that broke `format!("...'{}'...", selector)` before it was fixed to use
/// `js_string_literal`.
fn apostrophe_selector_fixture_url() -> &'static str {
    r#"data:text/html,<html><body>
        <div data-x="a" style="width:80px;height:40px;display:block">apostrophe target</div>
    </body></html>"#
}

#[test]
#[ignore]
fn test_page_evaluator_evaluate_returns_string() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    assert_eq!(c.evaluate("'ok'").unwrap(), "ok");
}

#[test]
#[ignore]
fn test_page_evaluator_get_computed_style_reads_property() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    let bg = c.get_computed_style(".box", "background-color").unwrap();
    assert!(!bg.is_empty());
}

#[test]
#[ignore]
fn test_page_evaluator_get_computed_style_missing_selector_returns_err() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    assert!(c.get_computed_style(".no-such", "color").is_err());
}

#[test]
#[ignore]
fn test_page_evaluator_get_bounding_rect_has_nonzero_dimensions() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    let r = c.get_bounding_rect(".box").unwrap();
    assert!(r.width > 0.0 && r.height > 0.0);
}

#[test]
#[ignore]
fn test_page_evaluator_set_viewport_width_changes_width() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    c.set_viewport_width(480).unwrap();
    assert_eq!(c.get_viewport_size().unwrap().0, 480);
}

/// @covers: evaluate
#[test]
#[ignore]
fn test_page_evaluator_evaluate_awaits_delayed_promise() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    let result = c
        .evaluate(
            "new Promise(function(resolve) { setTimeout(function() { resolve('resolved-value'); }, 200); })",
        )
        .unwrap();
    assert_eq!(result, "resolved-value", "evaluate must await the promise and return its resolved value");
}

/// @covers: evaluate
#[test]
#[ignore]
fn test_page_evaluator_evaluate_rejected_promise_returns_err() {
    let c = CdpClient::launch(fixture_url()).unwrap();
    let err = c
        .evaluate("Promise.reject(new Error('rejected-value'))")
        .expect_err("a rejected promise must surface as an error, not a silent empty result");
    assert!(err.contains("rejected-value"), "error must include the rejection reason, got: {}", err);
}

// ---------------------------------------------------------------------------
// shadow DOM piercing + selector escaping (issue #11)
// ---------------------------------------------------------------------------

/// @covers: get_computed_style
#[test]
#[ignore]
fn test_get_computed_style_finds_element_inside_open_shadow_root() {
    let c = CdpClient::launch(shadow_fixture_url()).unwrap();
    // A plain `document.querySelector` cannot see into the shadow root at
    // all — confirms the fixture actually exercises shadow piercing, not a
    // light-DOM element that would pass even without the fix.
    assert_eq!(
        c.evaluate("document.querySelector('.shadow-box') === null ? 'blind' : 'sees-it'").unwrap(),
        "blind",
        "fixture setup: plain querySelector must NOT see into the shadow root"
    );

    let display = c.get_computed_style(".shadow-box", "display").unwrap();
    assert_eq!(display, "block", "get_computed_style must pierce into the open shadow root");
}

/// @covers: get_bounding_rect
#[test]
#[ignore]
fn test_get_bounding_rect_finds_element_inside_open_shadow_root() {
    let c = CdpClient::launch(shadow_fixture_url()).unwrap();
    let r = c.get_bounding_rect(".shadow-box").unwrap();
    assert!(r.width > 0.0 && r.height > 0.0, "must resolve real dimensions for a shadow-rooted element");
}

/// @covers: get_pseudo_style
#[test]
#[ignore]
fn test_get_pseudo_style_finds_element_inside_open_shadow_root() {
    let c = CdpClient::launch(shadow_fixture_url()).unwrap();
    // `::first-letter` always resolves to *some* display value for a block
    // element — this only proves the element itself was found via piercing.
    let display = c.get_pseudo_style(".shadow-box", "::first-letter", "display").unwrap();
    assert!(!display.is_empty());
}

/// @covers: get_computed_style
#[test]
#[ignore]
fn test_get_computed_style_selector_with_embedded_apostrophe_does_not_break() {
    let c = CdpClient::launch(apostrophe_selector_fixture_url()).unwrap();
    // `[data-x='a']` — the selector string itself contains a literal `'`,
    // which broke the old `format!("...'{}'...", selector)` interpolation.
    let display = c
        .get_computed_style("[data-x='a']", "display")
        .expect("a selector containing a literal ' must not break the generated JS");
    assert_eq!(display, "block");
}

/// @covers: get_bounding_rect
#[test]
#[ignore]
fn test_get_bounding_rect_selector_with_embedded_apostrophe_does_not_break() {
    let c = CdpClient::launch(apostrophe_selector_fixture_url()).unwrap();
    let r = c
        .get_bounding_rect("[data-x='a']")
        .expect("a selector containing a literal ' must not break the generated JS");
    assert!(r.width > 0.0 && r.height > 0.0);
}

/// @covers: get_pseudo_style
#[test]
#[ignore]
fn test_get_pseudo_style_selector_with_embedded_apostrophe_does_not_break() {
    let c = CdpClient::launch(apostrophe_selector_fixture_url()).unwrap();
    let display = c
        .get_pseudo_style("[data-x='a']", "::first-letter", "display")
        .expect("a selector containing a literal ' must not break the generated JS");
    assert!(!display.is_empty());
}
