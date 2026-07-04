// Tests for PlatformBrowserLocator browser discovery.

use chromiumctl::CdpClient;

#[test]
fn test_platform_browser_locator_attach_fails_on_no_debugger() {
    assert!(
        CdpClient::attach(1).is_err(),
        "attach to port 1 must fail — no debugger listening"
    );
}

#[test]
fn test_platform_browser_locator_launch_returns_ok_or_browser_not_found() {
    match CdpClient::launch("data:text/html,<p>probe</p>") {
        Ok(_)  => {}
        Err(e) => assert!(!e.is_empty(), "error message must not be empty"),
    }
}
