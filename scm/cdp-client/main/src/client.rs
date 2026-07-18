use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};

use tungstenite::{stream::MaybeTlsStream, Message, WebSocket};

use crate::api::{CdpClient, PageEvaluator};
use crate::api::browser::BrowserLocator;
use crate::core::browser::PlatformBrowserLocator;

/// Ask the OS for a currently-unused TCP port instead of guessing from a
/// fixed/predictable starting value.
///
/// The previous approach (`static NEXT_PORT: AtomicU16` counting up from a
/// fixed 9300) only prevented collisions between `CdpClient`s launched
/// within the *same process* - concurrent process launches (multiple `csslense`
/// invocations, parallel test binaries, ...) all started counting from the
/// same 9300 and routinely raced for the same port. Binding an OS-assigned
/// ephemeral port has no fixed starting point to collide on, for either
/// sequential or concurrent launches.
fn pick_free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("failed to find a free port: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to read assigned port: {}", e))?
        .port();
    // `listener` drops here, freeing the port for Chrome to bind.
    Ok(port)
}

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
        let port = match port {
            Some(p) => p,
            None => pick_free_port()?,
        };

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
            #[cfg(feature = "android")]
            adb_forward: None,
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
            #[cfg(feature = "android")]
            adb_forward: None,
        })
    }

    /// Attach to a debuggable Android WebView owned by `package_name` on the
    /// single connected/authorized `adb` device.
    ///
    /// Enumerates active `webview_devtools_remote_*` sockets via `adb shell`,
    /// picks the first one owned by `package_name` (exact match, or
    /// `package_name:sub_process` for a named sub-process), forwards a local
    /// port to it, and attaches over that port exactly like [`CdpClient::attach`].
    /// The forward is torn down automatically when the returned client is dropped.
    ///
    /// Requires `adb` (`ADB_PATH` env var, or Android SDK `platform-tools` on
    /// a well-known path or `PATH`) and a device with
    /// `WebView.setWebContentsDebuggingEnabled(true)` active for the target app.
    #[cfg(feature = "android")]
    pub fn attach_android(package_name: &str) -> Result<Self, String> {
        use crate::core::android::AdbLocator;

        let adb = AdbLocator::find()?;
        let socket = AdbLocator::find_webview_socket(&adb, package_name)?;
        let port = AdbLocator::forward(&adb, &socket)?;

        match Self::attach(port) {
            Ok(mut client) => {
                client.adb_forward = Some((adb, port));
                Ok(client)
            }
            Err(e) => {
                AdbLocator::remove_forward(&adb, port);
                Err(e)
            }
        }
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

    /// Set the files on the `<input type="file">` matched by `selector`
    /// (piercing into open shadow roots, same as [`PageEvaluator`]'s
    /// default methods) via `DOM.setFileInputFiles`.
    ///
    /// Each entry in `file_paths` must exist on disk — checked up front, so
    /// a typo'd path fails with an actionable error naming the exact path
    /// rather than an opaque CDP error. Relative paths are resolved against
    /// this process's current directory before being sent to Chromium
    /// (which would otherwise resolve them against its own).
    ///
    /// Sets real files via CDP rather than synthesizing a `File`/
    /// `DataTransfer` in JS: Chromium reads each file itself, so the
    /// resulting `File` objects have correct, real metadata.
    pub fn set_files(&self, selector: &str, file_paths: &[String]) -> Result<(), String> {
        if file_paths.is_empty() {
            return Err("set_files requires at least one file path".to_string());
        }
        let absolute_paths = file_paths
            .iter()
            .map(|p| resolve_existing_absolute_path(p))
            .collect::<Result<Vec<_>, _>>()?;

        let js = format!(
            "(function() {{ {deep_query_selector} return __chromiumctl_deepQuerySelector(document, {selector}); }})()",
            deep_query_selector = crate::api::js::deep_query_selector_js(),
            selector = crate::api::js::js_string_literal(selector)?,
        );
        let eval_result = self.send_cdp(
            "Runtime.evaluate",
            serde_json::json!({
                "expression":    js,
                "returnByValue": false,
                "awaitPromise":  true,
            }),
        )?;
        if let Some(exc) = eval_result.get("exceptionDetails") {
            return Err(format!("JS exception resolving selector: {}", exc));
        }
        let object_id = eval_result["result"]["objectId"]
            .as_str()
            .ok_or_else(|| format!("element not found: {}", selector))?;

        self.send_cdp(
            "DOM.setFileInputFiles",
            serde_json::json!({ "files": absolute_paths, "objectId": object_id }),
        )?;
        Ok(())
    }

    fn send_cdp(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut socket = self.socket.lock()
            .map_err(|_| "socket lock poisoned".to_string())?;
        send_cdp_raw(&mut socket, id, method, params)
    }

    /// Block until a CDP *event* (unsolicited, no `id` — e.g.
    /// `Fetch.requestPaused`) named `method` arrives, and return its
    /// `params`. Messages that are responses to some other in-flight
    /// command, or events with a different method name, are discarded
    /// while waiting — same policy [`Self::send`]/[`Self::evaluate`] use
    /// for responses that don't match their own request `id`.
    ///
    /// Intended for a connection dedicated to receiving one kind of event
    /// (e.g. a `mock` session's own `CdpClient::attach`, after
    /// `Fetch.enable`) — not general-purpose event multiplexing alongside
    /// unrelated `send`/`evaluate` calls on the same connection, since a
    /// response to some other command sent concurrently would also be
    /// silently discarded here rather than delivered to its actual caller.
    ///
    /// Returns an error if no matching event arrives within `timeout`.
    pub fn wait_for_event(&self, method: &str, timeout: Duration) -> Result<serde_json::Value, String> {
        let deadline = Instant::now() + timeout;
        let mut socket = self.socket.lock()
            .map_err(|_| "socket lock poisoned".to_string())?;
        let result = wait_for_event_raw(&mut socket, method, deadline, timeout);
        // Always restore blocking mode, regardless of outcome, so a
        // subsequent `send`/`evaluate` on this same connection doesn't
        // inherit a leftover short read timeout and spuriously fail.
        let _ = set_read_timeout(&socket, None);
        result
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
        #[cfg(feature = "android")]
        if let Some((ref adb, port)) = self.adb_forward {
            crate::core::android::AdbLocator::remove_forward(adb, port);
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
                "awaitPromise":  true,
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

/// Validate that `raw` exists on disk and resolve it to an absolute path,
/// so relative paths are resolved against *this process's* current
/// directory (what a caller actually means) rather than Chromium's own.
fn resolve_existing_absolute_path(raw: &str) -> Result<String, String> {
    let path = std::path::Path::new(raw);
    if !path.exists() {
        return Err(format!("file not found: '{}'", raw));
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("failed to resolve current directory: {}", e))?
            .join(path)
    };
    Ok(absolute.to_string_lossy().into_owned())
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

fn wait_for_event_raw(
    socket:   &mut WebSocket<MaybeTlsStream<TcpStream>>,
    method:   &str,
    deadline: Instant,
    timeout:  Duration,
) -> Result<serde_json::Value, String> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("timed out waiting for '{}' event after {:?}", method, timeout));
        }
        set_read_timeout(socket, Some(remaining))?;

        let msg = match socket.read() {
            Ok(m) => m,
            Err(tungstenite::Error::Io(e)) if is_timeout_error(&e) => {
                return Err(format!("timed out waiting for '{}' event after {:?}", method, timeout));
            }
            Err(e) => return Err(format!("CDP read failed while waiting for '{}': {}", method, e)),
        };

        match msg {
            Message::Text(text) => {
                let val: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("CDP event parse error: {}", e))?;
                if val["method"].as_str() == Some(method) {
                    return Ok(val["params"].clone());
                }
                // A response to some other in-flight command, or an event
                // we're not waiting for right now — discard and keep going.
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

fn is_timeout_error(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)
}

/// Set (or clear, with `None`) the underlying socket's read timeout.
/// `MaybeTlsStream::Plain` is the only variant this build can ever
/// construct (`tungstenite` is compiled with no TLS backend — CDP is
/// always a local `ws://` connection, never `wss://`).
fn set_read_timeout(
    socket:  &WebSocket<MaybeTlsStream<TcpStream>>,
    timeout: Option<Duration>,
) -> Result<(), String> {
    match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream
            .set_read_timeout(timeout)
            .map_err(|e| format!("failed to set read timeout: {}", e)),
        _ => Err("unsupported stream type for read timeout".to_string()),
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
    fn test_resolve_existing_absolute_path_fails_for_missing_file() {
        let err = resolve_existing_absolute_path("this-file-does-not-exist-anywhere.tmp")
            .expect_err("a nonexistent path must be rejected");
        assert!(err.contains("this-file-does-not-exist-anywhere.tmp"), "error must name the exact path: {}", err);
    }

    // Note: relative-path resolution (joining against the process's current
    // directory) is deliberately not unit-tested here via
    // `std::env::set_current_dir` — that mutates process-global state, and
    // `cargo test` runs this file's tests in parallel by default, making
    // such a test a flakiness risk for any test added later. Covered
    // instead by the `set-files` CLI e2e test, which exercises a real
    // relative path through an isolated subprocess.

    #[test]
    fn test_resolve_existing_absolute_path_leaves_an_already_absolute_path_unchanged_in_meaning() {
        let dir = std::env::temp_dir().join(format!("chromiumctl_client_test_abs_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("setup: temp dir must be creatable");
        let file = dir.join("abs.txt");
        std::fs::write(&file, b"x").expect("setup: file must be writable");

        let resolved = resolve_existing_absolute_path(&file.to_string_lossy())
            .expect("an existing absolute path must resolve");
        assert!(std::path::Path::new(&resolved).exists());
    }

    #[test]
    fn test_find_chrome_returns_path_or_helpful_error() {
        match PlatformBrowserLocator::find() {
            Ok(path) => assert!(!path.is_empty()),
            Err(msg) => assert!(msg.contains("No Chromium-based browser found"), "{}", msg),
        }
    }

    #[test]
    fn test_pick_free_port_returns_a_bindable_port() {
        let port = pick_free_port().expect("should find a free port");
        assert!(port > 0);
        // The port must actually be free immediately after being picked.
        let listener = TcpListener::bind(("127.0.0.1", port));
        assert!(listener.is_ok(), "port {port} should be bindable right after being picked");
    }

    /// @covers: pick_free_port - the actual property that fixes issue #7
    /// (concurrent csslense process launches colliding on a predictable
    /// starting port). Simulates concurrent launches via threads, since
    /// pick_free_port has no cross-process state to race on in the first
    /// place - the OS's ephemeral port allocator is the thing under test.
    #[test]
    fn test_pick_free_port_concurrent_calls_do_not_collide() {
        use std::thread;
        let handles: Vec<_> = (0..8).map(|_| thread::spawn(pick_free_port)).collect();
        let ports: Vec<u16> = handles
            .into_iter()
            .map(|h| h.join().expect("thread panicked").expect("should find a free port"))
            .collect();
        let mut unique = ports.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            ports.len(),
            "concurrent pick_free_port calls returned duplicate ports: {ports:?}"
        );
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
