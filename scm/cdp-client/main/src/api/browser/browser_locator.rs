/// Locates and connects to a Chromium-based browser on the host platform.
pub trait BrowserLocator {
    /// Locate a Chromium-based browser binary.
    fn find() -> Result<String, String>;

    /// Return the WebSocket debugger URL for the first page target on `port`.
    fn get_ws_url(port: u16) -> Result<String, String>;
}
