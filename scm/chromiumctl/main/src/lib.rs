//! `chromiumctl` — minimal Chromium DevTools Protocol client.
//!
//! Works with any Chromium-based browser: Chrome, Edge, Brave, Arc, Vivaldi.
//!
//! ## Quick start
//!
//! ```no_run
//! use chromiumctl::{CdpClient, PageEvaluator};
//!
//! // Launch headless Chrome and connect
//! let mut client = CdpClient::launch("https://example.com").unwrap();
//!
//! // Evaluate JavaScript
//! let title = client.evaluate("document.title").unwrap();
//!
//! // Read computed CSS
//! let color = client.get_computed_style("h1", "color").unwrap();
//!
//! // Resize the viewport (actually changes it — uses Emulation.setDeviceMetricsOverride)
//! client.set_viewport_width(375).unwrap();
//!
//! // Navigate to a new page
//! client.navigate("https://example.com/other").unwrap();
//! ```

mod api;
mod client;
mod core;
mod saf;

pub use api::{
    BrowserLocator, BrowserSession, CdpClient, CdpClientBuilder, PageEvaluator,
    PlatformBrowserLocator, Rect, Validator,
};
pub use saf::*;
