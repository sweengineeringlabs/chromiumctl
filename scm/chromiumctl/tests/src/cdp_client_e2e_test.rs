// E2e tests for CdpClient — exercises launch, attach, navigate, send, port, ws_url.
//
// All tests require a Chromium-based browser. Run with:
//   cargo test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used)]

use chromiumctl::{CdpClient, PageEvaluator};

fn fixture_url() -> String {
    "data:text/html,<html><body><p id=x>hello</p></body></html>".into()
}

/// @covers: launch
#[test]
#[ignore]
fn test_cdp_client_launch_connects_to_page() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert!(!c.ws_url().is_empty());
}

/// @covers: port
#[test]
#[ignore]
fn test_cdp_client_port_returns_nonzero() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert!(c.port() > 0);
}

/// @covers: ws_url
#[test]
#[ignore]
fn test_cdp_client_ws_url_starts_with_ws() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    assert!(c.ws_url().starts_with("ws://"));
}

/// @covers: attach
#[test]
#[ignore]
fn test_cdp_client_attach_reuses_session() {
    let c1 = CdpClient::launch(&fixture_url()).unwrap();
    let c2 = CdpClient::attach(c1.port()).unwrap();
    assert_eq!(c2.evaluate("1+1").unwrap(), "2");
}

/// @covers: navigate
#[test]
#[ignore]
fn test_cdp_client_navigate_loads_new_content() {
    let mut c = CdpClient::launch(&fixture_url()).unwrap();
    c.navigate("data:text/html,<p id=y>world</p>").unwrap();
    let found = c.evaluate("document.getElementById('y') !== null ? 'yes' : 'no'").unwrap();
    assert_eq!(found, "yes");
}

/// @covers: send
#[test]
#[ignore]
fn test_cdp_client_send_returns_valid_result() {
    let c = CdpClient::launch(&fixture_url()).unwrap();
    let result = c.send(
        "Runtime.evaluate",
        serde_json::json!({ "expression": "6*7", "returnByValue": true }),
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap()["result"]["value"], 42);
}
