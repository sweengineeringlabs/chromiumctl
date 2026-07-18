use std::net::TcpStream;
use std::process::Child;
use std::sync::{atomic::AtomicU64, Mutex};

use tungstenite::{stream::MaybeTlsStream, WebSocket};

/// A connected Chromium DevTools Protocol session.
///
/// Launches (or attaches to) a Chromium-based browser and holds a persistent
/// WebSocket connection to a page target. All CDP communication is synchronous
/// over that single connection.
///
/// ## Example
///
/// ```no_run
/// use cdp_client::{CdpClient, PageEvaluator};
///
/// let client = CdpClient::launch("https://example.com").unwrap();
/// let title  = client.evaluate("document.title").unwrap();
/// println!("{}", title);
/// ```
pub struct CdpClient {
    pub(crate) socket:         Mutex<WebSocket<MaybeTlsStream<TcpStream>>>,
    pub(crate) next_id:        AtomicU64,
    pub(crate) chrome_process: Option<Child>,
    pub(crate) port:           u16,
    pub(crate) ws_url:         String,
    /// `Some((adb_path, local_port))` when this client owns an `adb forward`
    /// created by `attach_android` — torn down on `Drop`.
    #[cfg(feature = "android")]
    pub(crate) adb_forward: Option<(String, u16)>,
}
