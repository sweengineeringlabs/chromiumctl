// E2e tests for CdpClientBuilder.
//
// Requires a Chromium-based browser. Run with:
//   cargo test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromiumctl::{CdpClientBuilder, PageEvaluator};

#[test]
#[ignore]
fn test_cdp_client_builder_launch_connects_to_url() {
    let c = CdpClientBuilder::new("data:text/html,<p id=x>ok</p>")
        .launch()
        .expect("builder launch must succeed");
    let found = c.evaluate("document.getElementById('x') !== null ? 'yes' : 'no'").unwrap();
    assert_eq!(found, "yes");
}

#[test]
#[ignore]
fn test_cdp_client_builder_port_is_honored_on_launch() {
    let c = CdpClientBuilder::new("data:text/html,<p>test</p>")
        .port(9399)
        .launch()
        .expect("builder launch must succeed");
    assert_eq!(c.port(), 9399, "builder's fixed port must be used, not an auto-assigned one");
}

#[test]
fn test_cdp_client_builder_chrome_bin_override_is_accepted() {
    let b = CdpClientBuilder::new("about:blank").chrome_bin("/nonexistent/chrome");
    // Only validates the builder accepts the override — actual launch would fail.
    let result = b.launch();
    assert!(result.is_err(), "launch with a bad chrome_bin path must fail");
}
