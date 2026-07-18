/// Default CDP remote-debugging port range start.
pub const DEFAULT_DEBUG_PORT: u16 = 9300;

/// Viewport width for mobile simulation (375 px, iPhone-class device).
pub const MOBILE_VIEWPORT_WIDTH: u32 = 375;

/// Viewport width for standard desktop simulation (1280 px).
pub const DESKTOP_VIEWPORT_WIDTH: u32 = 1280;

/// Timeout for browser launch and debugger readiness, in milliseconds.
pub const BROWSER_LAUNCH_TIMEOUT_MS: u64 = 10_000;

/// Default interval between debugger readiness poll attempts, in milliseconds.
pub const DEBUGGER_POLL_INTERVAL_MS: u64 = 200;

/// Maximum number of concurrent CDP sessions recommended per process.
pub const MAX_CONCURRENT_SESSIONS: usize = 16;
