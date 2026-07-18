// Tests verifying BrowserSession interface contracts.
//
// BrowserSession is the SPI trait — tested via CdpClient (the default implementor).
#![allow(clippy::unwrap_used)]

use cdp_client::{CdpClient, PageEvaluator};

#[test]
#[ignore]
fn test_browser_session_evaluate_returns_result() {
    let c = CdpClient::launch("data:text/html,<p>test</p>").unwrap();
    assert_eq!(c.evaluate("1+1").unwrap(), "2");
}

#[test]
#[ignore]
fn test_browser_session_port_is_nonzero() {
    let c = CdpClient::launch("data:text/html,<p>test</p>").unwrap();
    assert!(c.port() > 0);
}

#[test]
#[ignore]
fn test_browser_session_ws_url_is_websocket() {
    let c = CdpClient::launch("data:text/html,<p>test</p>").unwrap();
    assert!(c.ws_url().starts_with("ws://"));
}
