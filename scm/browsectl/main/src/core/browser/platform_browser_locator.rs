use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::api::browser::BrowserLocator;

/// Platform-native implementation of [`BrowserLocator`].
pub(crate) struct PlatformBrowserLocator;

impl BrowserLocator for PlatformBrowserLocator {
    fn find() -> Result<String, String> {
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
            return Err(format!(
                "CHROME_PATH is set to '{}' but that path does not exist",
                path,
            ));
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
}

impl PlatformBrowserLocator {
    /// Poll the debugger endpoint until a WebSocket URL is available (up to 10 s).
    pub(crate) fn wait_for_debugger(port: u16) -> Result<String, String> {
        Self::wait_with_timeout(port, Duration::from_secs(10))
    }

    fn wait_with_timeout(port: u16, timeout: Duration) -> Result<String, String> {
        let deadline = Instant::now() + timeout;
        let mut last_err = String::from("timeout waiting for Chromium debugger");
        while Instant::now() < deadline {
            match Self::get_ws_url(port) {
                Ok(url) => return Ok(url),
                Err(e)  => last_err = e,
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        Err(last_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_for_debugger_fails_when_no_browser_on_port() {
        let result = PlatformBrowserLocator::wait_with_timeout(
            19996, Duration::from_secs(1)
        );
        assert!(result.is_err(), "should fail when no browser is listening");
    }
}
