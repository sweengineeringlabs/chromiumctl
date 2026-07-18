use crate::api::PageEvaluator;
use serde_json::Value;

/// A CDP session backend that can be substituted for the default tungstenite transport.
///
/// Implement this trait to back `CdpClient`-style operations with a different transport
/// (e.g., a remote proxy, an in-process recorder, or a test double).
pub trait BrowserSession: PageEvaluator {
    /// The Chrome DevTools Protocol remote-debugging port this session is bound to.
    fn port(&self) -> u16;

    /// The WebSocket debugger URL for the connected page target.
    fn ws_url(&self) -> &str;

    /// Send a raw CDP command and return the `result` field of the response.
    fn send_command(&self, method: &str, params: Value) -> Result<Value, String>;
}
