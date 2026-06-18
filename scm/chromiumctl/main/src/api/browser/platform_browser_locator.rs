/// The result of browser binary discovery on the current platform.
pub struct PlatformBrowserLocator {
    /// Absolute path to the discovered browser binary.
    pub path: String,
}

impl PlatformBrowserLocator {
    /// Wrap a discovered browser binary path.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}
