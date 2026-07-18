pub mod browser;
pub mod js;
pub mod spi;
pub mod traits;
pub mod types;

pub use browser::{BrowserLocator, PlatformBrowserLocator};
pub use js::{deep_query_selector_js, js_string_literal};
pub use spi::BrowserSession;
pub use traits::{PageEvaluator, Validator};
pub use types::{CdpClient, CdpClientBuilder, Rect};
