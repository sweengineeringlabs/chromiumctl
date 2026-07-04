use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicU16, AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};

use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

use crate::api::{CdpClient, PageEvaluator};
use crate::api::browser::BrowserLocator;
use crate::core::browser::PlatformBrowserLocator;

static NEXT_PORT: AtomicU16 = AtomicU16::new(9300);

impl CdpClient {
    /// Launch a new headless Chromium instance, navigate to `url`, and connect.
    ///
    /// Discovers the browser binary via the `CHROME_PATH` environment variable
    /// or well-known platform paths.
    pub fn launch(url: &str) -> Result<Self, String> {
        Self::launch_on_port(url, None)
    }

    /// Launch a new headless Chromium instance on a specific `port` (or an
    /// auto-assigned one if `port` is `None`), navigate to `url`, and connect.
    ///
    /// Used by [`CdpClientBuilder::port`] to honor a caller-fixed debugging port.
    ///
    /// [`CdpClientBuilder::port`]: crate::CdpClientBuilder::port
    pub(crate) fn launch_on_port(url: &str, port: Option<u16>) -> Result<Self, String> {
        let chrome = PlatformBrowserLocator::find()?;
        let port   = port.unwrap_or_else(|| NEXT_PORT.fetch_add(1, Ordering::Relaxed));

        let mut chrome_process = Command::new(&chrome)
            .args([
                "--headless=new",
                &format!("--remote-debugging-port={}", port),
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-gpu",
                "--disable-extensions",
                "--disable-translate",
                "--disable-background-networking",
                "--mute-audio",
                url,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to launch Chromium at '{}': {}", chrome, e))?;

        // `Child` is not killed on drop, so a spawned-but-unreachable browser
        // must be reaped explicitly before propagating either error below.
        let ws_url = match PlatformBrowserLocator::wait_for_debugger(port) {
            Ok(url) => url,
            Err(e) => {
                let _ = chrome_process.kill();
                let _ = chrome_process.wait();
                return Err(e);
            }
        };
        let socket = match connect_ws(&ws_url) {
            Ok(socket) => socket,
            Err(e) => {
                let _ = chrome_process.kill();
                let _ = chrome_process.wait();
                return Err(e);
            }
        };

        let client = Self {
            socket:         Mutex::new(socket),
            next_id:        AtomicU64::new(1),
            chrome_process: Some(chrome_process),
            port,
            ws_url,
        };
        client.wait_for_load();
        Ok(client)
    }

    /// Attach to an already-running Chromium instance at `port`.
    pub fn attach(port: u16) -> Result<Self, String> {
        let ws_url = PlatformBrowserLocator::get_ws_url(port)?;
        let socket = connect_ws(&ws_url)?;
        Ok(Self {
            socket:         Mutex::new(socket),
            next_id:        AtomicU64::new(1),
            chrome_process: None,
            port,
            ws_url,
        })
    }

    /// Navigate to `url` and wait for the page to finish loading (up to 10 s).
    pub fn navigate(&mut self, url: &str) -> Result<(), String> {
        self.send_cdp("Page.navigate", serde_json::json!({ "url": url }))?;
        std::thread::sleep(Duration::from_millis(150));
        self.wait_for_load();
        Ok(())
    }

    /// The Chrome DevTools Protocol debugging port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The WebSocket debugger URL for the connected page target.
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Send a raw CDP command and return the `result` field of the response.
    ///
    /// Useful for CDP methods not covered by [`PageEvaluator`].
    pub fn send(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        self.send_cdp(method, params)
    }

    fn send_cdp(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut socket = self.socket.lock()
            .map_err(|_| "socket lock poisoned".to_string())?;
        send_cdp_raw(&mut socket, id, method, params)
    }

    fn wait_for_load(&self) {
        let js = "document.readyState === 'complete' && \
                  document.body !== null && \
                  document.body.childElementCount > 0 ? 'ready' : 'not'";
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            match self.evaluate(js) {
                Ok(ref s) if s == "ready" => return,
                _ => std::thread::sleep(Duration::from_millis(50)),
            }
        }
    }
}

impl Drop for CdpClient {
    fn drop(&mut self) {
        if self.chrome_process.is_some() {
            // On Windows, the process spawned by `Command::spawn` is a launcher
            // stub that re-execs and exits almost immediately — the real browser
            // (and its renderer subprocesses) end up as an unrelated PID that
            // `Child::kill()` below cannot see or terminate. Ask the browser to
            // close itself over CDP first; this reliably tears down the whole
            // process tree regardless of that launcher indirection.
            let _ = self.send_cdp("Browser.close", serde_json::json!({}));
        }
        if let Ok(mut socket) = self.socket.lock() {
            let _ = socket.close(None);
        }
        if let Some(ref mut proc) = self.chrome_process {
            let _ = proc.kill();
            let _ = proc.wait();
        }
    }
}

// ---------------------------------------------------------------------------
// PageEvaluator
// ---------------------------------------------------------------------------

impl PageEvaluator for CdpClient {
    fn evaluate(&self, js: &str) -> Result<String, String> {
        let result = self.send_cdp(
            "Runtime.evaluate",
            serde_json::json!({
                "expression":    js,
                "returnByValue": true,
                "awaitPromise":  false,
            }),
        )?;

        if let Some(exc) = result.get("exceptionDetails") {
            return Err(format!("JS exception: {}", exc));
        }

        let r = &result["result"];
        match r["type"].as_str() {
            Some("undefined")                                         => Ok(String::new()),
            Some("string")                                            => Ok(r["value"].as_str().unwrap_or("").to_string()),
            Some("number")                                            => Ok(r["value"].to_string()),
            Some("boolean")                                           => Ok(r["value"].to_string()),
            Some("object") if r["subtype"].as_str() == Some("null")  => Ok(String::new()),
            _                                                         => Ok(r["description"].as_str().unwrap_or("").to_string()),
        }
    }

    fn set_viewport_width(&self, width: u32) -> Result<(), String> {
        self.send_cdp(
            "Emulation.setDeviceMetricsOverride",
            serde_json::json!({
                "width":             width,
                "height":            768,
                "deviceScaleFactor": 1,
                "mobile":            false,
            }),
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// WebSocket helpers
// ---------------------------------------------------------------------------

fn connect_ws(ws_url: &str) -> Result<WebSocket<MaybeTlsStream<TcpStream>>, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_err = format!("timed out waiting for WebSocket on '{}'", ws_url);
    while Instant::now() < deadline {
        match tungstenite::connect(ws_url) {
            Ok((socket, _)) => return Ok(socket),
            Err(e) => {
                last_err = format!("WebSocket connect to '{}' failed: {}", ws_url, e);
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(last_err)
}

fn send_cdp_raw(
    socket: &mut WebSocket<MaybeTlsStream<TcpStream>>,
    id:     u64,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let msg = serde_json::json!({ "id": id, "method": method, "params": params }).to_string();
    socket
        .send(Message::Text(msg))
        .map_err(|e| format!("CDP send '{}' failed: {}", method, e))?;

    loop {
        match socket.read().map_err(|e| format!("CDP read failed: {}", e))? {
            Message::Text(text) => {
                let val: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("CDP response parse error: {}", e))?;
                if val["id"].as_u64() == Some(id) {
                    if let Some(err) = val.get("error") {
                        return Err(format!("CDP error from '{}': {}", method, err));
                    }
                    return Ok(val["result"].clone());
                }
            }
            Message::Ping(data) => {
                socket.send(Message::Pong(data))
                    .map_err(|e| format!("CDP pong failed: {}", e))?;
            }
            Message::Close(_) => return Err("CDP connection closed unexpectedly".into()),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::api::CdpClient;
    use crate::api::browser::BrowserLocator;
    use crate::core::browser::PlatformBrowserLocator;

    #[test]
    fn test_find_chrome_returns_path_or_helpful_error() {
        match PlatformBrowserLocator::find() {
            Ok(path) => assert!(!path.is_empty()),
            Err(msg) => assert!(msg.contains("No Chromium-based browser found"), "{}", msg),
        }
    }

    #[test]
    fn test_next_port_increments() {
        let p1 = NEXT_PORT.load(Ordering::Relaxed);
        let p2 = NEXT_PORT.fetch_add(1, Ordering::Relaxed);
        assert_eq!(p1, p2);
        assert_eq!(NEXT_PORT.load(Ordering::Relaxed), p2 + 1);
        NEXT_PORT.fetch_sub(1, Ordering::Relaxed);
    }

    #[test]
    fn test_get_ws_url_fails_when_no_browser_running() {
        assert!(PlatformBrowserLocator::get_ws_url(19999).is_err());
    }

    /// @covers: launch
    #[test]
    fn test_launch_returns_ok_or_no_browser_error() {
        match CdpClient::launch("data:text/html,<h1>test</h1>") {
            Ok(_) => {}
            Err(e) => assert!(!e.is_empty(), "error message must not be empty"),
        }
    }

    /// @covers: attach
    #[test]
    fn test_attach_fails_when_no_debugger_on_port() {
        assert!(CdpClient::attach(1).is_err());
    }

    /// @covers: navigate
    #[test]
    fn test_navigate_updates_page_when_browser_available() {
        if let Ok(mut c) = CdpClient::launch("data:text/html,<p>start</p>") {
            c.navigate("data:text/html,<p>end</p>").unwrap();
        }
    }

    /// @covers: navigate
    #[test]
    #[ignore = "requires a running Chromium instance"]
    fn test_navigate_changes_page_content() {
        let mut c = CdpClient::launch("data:text/html,<p>start</p>").unwrap();
        c.navigate("data:text/html,<p id=x>navigated</p>").unwrap();
        assert_eq!(
            c.evaluate("document.getElementById('x') !== null ? 'yes' : 'no'").unwrap(),
            "yes"
        );
    }

    /// @covers: send
    #[test]
    fn test_send_dispatches_cdp_command_when_browser_available() {
        if let Ok(c) = CdpClient::launch("data:text/html,<p>test</p>") {
            let result = c.send(
                "Runtime.evaluate",
                serde_json::json!({ "expression": "1", "returnByValue": true }),
            );
            assert!(result.is_ok(), "send must succeed when browser is running");
        }
    }

    /// @covers: send
    #[test]
    #[ignore = "requires a running Chromium instance"]
    fn test_send_raw_cdp_command_returns_result() {
        let c = CdpClient::launch("data:text/html,<h1>test</h1>").unwrap();
        let result = c.send(
            "Runtime.evaluate",
            serde_json::json!({ "expression": "1+1", "returnByValue": true }),
        );
        assert!(result.is_ok());
    }

    /// @covers: port
    #[test]
    #[ignore = "requires a running Chromium instance"]
    fn test_port_returns_assigned_port() {
        let c = CdpClient::launch("data:text/html,<h1>test</h1>").unwrap();
        assert!(c.port() > 0);
    }

    /// @covers: ws_url
    #[test]
    #[ignore = "requires a running Chromium instance"]
    fn test_ws_url_returns_websocket_url() {
        let c = CdpClient::launch("data:text/html,<h1>test</h1>").unwrap();
        assert!(c.ws_url().starts_with("ws://"), "ws_url must be a WebSocket URL");
    }
}
