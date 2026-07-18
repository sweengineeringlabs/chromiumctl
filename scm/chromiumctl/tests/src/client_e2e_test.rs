// Live-browser e2e tests — exercise tungstenite WebSocket transport via CdpClient.
//
// Requires a Chromium-based browser (Chrome, Edge, Brave) to be installed,
// or `CHROME_PATH` set to the binary.
//
// Run with:
//   cargo test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used, clippy::expect_used)]

use browsectl::{CdpClient, PageEvaluator};
use tungstenite as _; // tungstenite WebSocket transport is exercised by every CdpClient call below

/// Inline HTML fixture as a data: URL — no file-system access required.
fn fixture_url() -> String {
    let html = r#"<!DOCTYPE html>
<html>
<head>
<style>
.btn { display: inline-flex; padding: 8px 16px; border-radius: 6px; font-weight: 500; cursor: pointer; border: none; }
.btn--primary { background-color: %232563eb; color: white; }
.btn--danger  { background-color: %23dc2626; color: white; }
.card { background: white; border-radius: 8px; padding: 16px; }
.hidden { display: none; }
</style>
</head>
<body>
<button class="btn btn--primary" id="primary-btn">Primary</button>
<button class="btn btn--danger"  id="danger-btn">Danger</button>
<div class="card" id="card"><div class="card__header">Title</div></div>
<div class="hidden" id="hidden-el">Hidden</div>
</body>
</html>"#;
    format!("data:text/html,{}", html)
}

// ---------------------------------------------------------------------------
// evaluate
// ---------------------------------------------------------------------------

/// @covers: launch
#[test]
#[ignore]
fn test_evaluate_string() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("'hello'").unwrap(), "hello");
}

#[test]
#[ignore]
fn test_evaluate_number() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("2 + 2").unwrap(), "4");
}

#[test]
#[ignore]
fn test_evaluate_boolean() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("1 === 1").unwrap(), "true");
}

#[test]
#[ignore]
fn test_evaluate_undefined_returns_empty() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.evaluate("void(0)").unwrap(), "");
}

#[test]
#[ignore]
fn test_evaluate_dom_element_found() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let r = c.evaluate("document.querySelector('.btn--primary') !== null ? 'found' : 'missing'").unwrap();
    assert_eq!(r, "found");
}

// ---------------------------------------------------------------------------
// get_computed_style
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_computed_style_reads_display() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.get_computed_style(".btn", "display").unwrap(), "inline-flex");
}

#[test]
#[ignore]
fn test_computed_style_hidden_element_is_none() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert_eq!(c.get_computed_style(".hidden", "display").unwrap(), "none");
}

#[test]
#[ignore]
fn test_computed_style_missing_element_returns_err() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let r = c.get_computed_style(".no-such-thing", "color");
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("not found"));
}

// ---------------------------------------------------------------------------
// get_bounding_rect
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_bounding_rect_visible_element_has_size() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let r = c.get_bounding_rect(".btn--primary").unwrap();
    assert!(r.width > 0.0);
    assert!(r.height > 0.0);
}

#[test]
#[ignore]
fn test_bounding_rect_missing_element_returns_err() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert!(c.get_bounding_rect(".no-such-thing").is_err());
}

// ---------------------------------------------------------------------------
// set_viewport_width / get_viewport_size
// ---------------------------------------------------------------------------

/// Core regression test: verifies Emulation.setDeviceMetricsOverride actually
/// changes the viewport (not just a fake JS property).
#[test]
#[ignore]
fn test_set_viewport_width_changes_actual_viewport() {
    let c = CdpClient::launch(&fixture_url()).unwrap();

    c.set_viewport_width(375).unwrap();
    assert_eq!(c.get_viewport_size().unwrap().0, 375);

    c.set_viewport_width(1280).unwrap();
    assert_eq!(c.get_viewport_size().unwrap().0, 1280);
}

#[test]
#[ignore]
fn test_viewport_width_affects_media_queries() {
    let c = CdpClient::launch(&fixture_url()).unwrap();

    c.evaluate(r#"
        var s = document.createElement('style');
        s.textContent = '.mq { color: red; } @media (min-width: 768px) { .mq { color: blue; } }';
        document.head.appendChild(s);
        var el = document.createElement('div');
        el.className = 'mq';
        document.body.appendChild(el);
    "#).unwrap();

    c.set_viewport_width(375).unwrap();
    let _ = c.evaluate("document.body.offsetHeight");
    let narrow = c.get_computed_style(".mq", "color").unwrap();

    c.set_viewport_width(1024).unwrap();
    let _ = c.evaluate("document.body.offsetHeight");
    let wide = c.get_computed_style(".mq", "color").unwrap();

    assert_ne!(narrow, wide, "color should differ across breakpoint: narrow={narrow} wide={wide}");
}

// ---------------------------------------------------------------------------
// navigate
// ---------------------------------------------------------------------------

/// @covers: navigate
#[test]
#[ignore]
fn test_navigate_changes_page_content() {
    let mut c = CdpClient::launch(&fixture_url()).unwrap();
    c.navigate("data:text/html,<h1 id=marker>navigated</h1>").unwrap();
    let found = c.evaluate("document.getElementById('marker') !== null ? 'yes' : 'no'").unwrap();
    assert_eq!(found, "yes");
}

// ---------------------------------------------------------------------------
// send / attach / port / ws_url
// ---------------------------------------------------------------------------

/// @covers: send
#[test]
#[ignore]
fn test_send_raw_cdp_returns_result() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let result = c.send(
        "Runtime.evaluate",
        serde_json::json!({ "expression": "40+2", "returnByValue": true }),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["result"]["value"], 42);
}

/// @covers: attach
/// @covers: port
#[test]
#[ignore]
fn test_attach_to_existing_browser() {
    let c1 = CdpClient::launch(&fixture_url()).unwrap();
    let c2 = CdpClient::attach(c1.port()).unwrap();
    assert_eq!(c2.evaluate("1 + 1").unwrap(), "2");
}

/// @covers: ws_url
#[test]
#[ignore]
fn test_ws_url_is_websocket_url() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert!(c.ws_url().starts_with("ws://"), "ws_url must start with ws://");
}

// ---------------------------------------------------------------------------
// wait_for_event (issue #11: mock's Fetch.requestPaused interception loop
// depends on this; tested independently here per that issue's AC)
// ---------------------------------------------------------------------------

/// @covers: wait_for_event
#[test]
#[ignore]
fn test_wait_for_event_returns_params_of_a_real_event() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    c.send("Page.enable", serde_json::json!({})).unwrap();

    // `navigate` itself already waits for readiness via polling `evaluate`,
    // so trigger the navigation directly over CDP instead, to leave the
    // real `Page.loadEventFired` event unconsumed for `wait_for_event` to
    // actually receive.
    c.send("Page.navigate", serde_json::json!({ "url": "data:text/html,<h1>done</h1>" })).unwrap();

    let params = c
        .wait_for_event("Page.loadEventFired", std::time::Duration::from_secs(10))
        .expect("a real Page.loadEventFired event must be received");
    assert!(params.is_object(), "event params must be a JSON object, got: {}", params);
}

/// @covers: wait_for_event
#[test]
#[ignore]
fn test_wait_for_event_times_out_when_event_never_arrives() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let err = c
        .wait_for_event("Totally.FakeEventThatNeverFires", std::time::Duration::from_millis(500))
        .expect_err("must time out, not hang forever, when the event never arrives");
    assert!(err.contains("timed out"), "error must say it timed out, got: {}", err);
}

/// @covers: wait_for_event
#[test]
#[ignore]
fn test_wait_for_event_timeout_does_not_break_subsequent_send_calls() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let _ = c.wait_for_event("Totally.FakeEventThatNeverFires", std::time::Duration::from_millis(300));

    // Regression check for the read-timeout restore: a timed-out
    // `wait_for_event` must not leave the socket's read timeout set to a
    // short duration, which would make this ordinary blocking call
    // spuriously fail if `evaluate` ever took longer than that leftover
    // window.
    assert_eq!(c.evaluate("1 + 1").unwrap(), "2");
}
