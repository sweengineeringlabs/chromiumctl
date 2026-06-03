use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicU16, AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};

use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

use crate::evaluator::PageEvaluator;

static NEXT_PORT: AtomicU16 = AtomicU16::new(9300);

/// A connected Chromium DevTools Protocol session.
///
/// Launches (or attaches to) a Chromium-based browser and holds a persistent
/// WebSocket connection to a page target. All CDP communication is synchronous
/// over that single connection.
///
/// ## Example
///
/// ```no_run
/// use chromiumctl::{CdpClient, PageEvaluator};
///
/// let client = CdpClient::launch("https://example.com").unwrap();
/// let title  = client.evaluate("document.title").unwrap();
/// println!("{}", title);
/// ```
pub struct CdpClient {
    socket:         Mutex<WebSocket<MaybeTlsStream<TcpStream>>>,
    next_id:        AtomicU64,
    chrome_process: Option<Child>,
    port:           u16,
    ws_url:         String,
}

impl CdpClient {
    /// Launch a new headless Chromium instance, navigate to `url`, and connect.
    ///
    /// Discovers the browser binary via the `CHROME_PATH` environment variable
    /// or well-known platform paths.
    pub fn launch(url: &str) -> Result<Self, String> {
        let chrome = find_chrome()?;
        let port   = NEXT_PORT.fetch_add(1, Ordering::Relaxed);

        let chrome_process = Command::new(&chrome)
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

        let ws_url = wait_for_debugger(port)?;
        let socket = connect_ws(&ws_url)?;

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
        let ws_url = get_ws_url(port)?;
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

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

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
    let msg = serde_json::json!({ "id": id, "method": method, "params": params });
    socket
        .send(Message::Text(msg.to_string()))
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
// Chrome discovery
// ---------------------------------------------------------------------------

fn find_chrome() -> Result<String, String> {
    let candidates: Vec<&str> = if cfg!(windows) {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ]
    } else {
        vec![
            "google-chrome-stable",
            "google-chrome",
            "chromium-browser",
            "chromium",
            "microsoft-edge-stable",
            "brave-browser",
        ]
    };

    if let Ok(path) = std::env::var("CHROME_PATH") {
        if Path::new(&path).exists() {
            return Ok(path);
        }
    }

    for candidate in &candidates {
        if Path::new(candidate).exists() {
            return Ok(candidate.to_string());
        }
        if cfg!(not(windows)) {
            if let Ok(out) = Command::new("which").arg(candidate).output() {
                if out.status.success() {
                    return Ok(candidate.to_string());
                }
            }
        }
    }

    Err("No Chromium-based browser found. Install Chrome/Edge/Brave or set CHROME_PATH.".into())
}

// ---------------------------------------------------------------------------
// Debugger endpoint helpers
// ---------------------------------------------------------------------------

fn wait_for_debugger(port: u16) -> Result<String, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_err = String::from("timeout waiting for Chromium debugger");
    while Instant::now() < deadline {
        match get_ws_url(port) {
            Ok(url) => return Ok(url),
            Err(e)  => last_err = e,
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(last_err)
}

fn get_ws_url(port: u16) -> Result<String, String> {
    let url = format!("http://localhost:{}/json", port);
    let output = Command::new("curl")
        .args(["-s", "--max-time", "2", &url])
        .output()
        .map_err(|e| format!("curl failed: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "HTTP GET {} failed (exit {})",
            url,
            output.status.code().unwrap_or(-1)
        ));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let targets: Vec<serde_json::Value> = serde_json::from_str(&body)
        .map_err(|e| format!("failed to parse /json response: {}", e))?;

    targets
        .iter()
        .find(|t| t["type"].as_str() == Some("page"))
        .and_then(|t| t["webSocketDebuggerUrl"].as_str())
        .map(String::from)
        .ok_or_else(|| "no page target found in Chromium /json response".into())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_chrome_returns_path_or_helpful_error() {
        match find_chrome() {
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
        assert!(get_ws_url(19999).is_err());
    }
}
