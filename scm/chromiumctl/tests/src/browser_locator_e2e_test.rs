// E2e tests for the BrowserLocator trait (exercised via CdpClient).
//
// Requires a Chromium-based browser. Run with:
//   cargo test -- --ignored --test-threads=1

use chromiumctl::CdpClient;

/// Verifies browser discovery by attempting a launch — uses BrowserLocator internally.
#[test]
#[ignore]
fn test_browser_locator_find_locates_installed_browser() {
    let result = CdpClient::launch("data:text/html,<p>probe</p>");
    assert!(result.is_ok(), "BrowserLocator::find must succeed when a browser is installed");
}
