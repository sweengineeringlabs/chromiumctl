// E2e tests for the BrowserLocator trait (exercised via CdpClient).
//
// Requires a Chromium-based browser. Run with:
//   cargo test -- --ignored --test-threads=1

#![allow(clippy::unwrap_used, clippy::expect_used)]

use browsectl::CdpClient;

/// Verifies browser discovery by attempting a launch — uses BrowserLocator internally.
#[test]
#[ignore]
fn test_browser_locator_find_locates_installed_browser() {
    let client = CdpClient::launch("data:text/html,<p>probe</p>")
        .expect("BrowserLocator::find must succeed when a browser is installed");
    assert!(client.port() > 0, "a located browser must be reachable on a real debug port");
    assert!(
        client.ws_url().starts_with("ws://"),
        "a located browser must yield a real WebSocket debugger URL, got: {}",
        client.ws_url()
    );
}
