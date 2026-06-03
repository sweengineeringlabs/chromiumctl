pub mod browser;
pub mod spi;
pub mod traits;
pub mod types;

pub use traits::{PageEvaluator, Validator};
pub use types::{CdpClient, CdpClientBuilder, Rect};
