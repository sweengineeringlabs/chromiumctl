pub mod launch;
pub mod eval;
pub mod screenshot;
pub mod navigate;
pub mod wait;
pub mod click;
pub mod input;
pub mod get_dom;
pub mod metrics;
pub mod mock;
pub mod reap;
pub mod set_files;
pub mod stop;

/// Errors surfaced by CLI subcommands, mapped to RFC-0001's exit codes.
#[derive(Debug)]
pub enum CliError {
    /// Exit code 1 — the command ran but the browser-side action failed
    /// (JS exception, element not found, bad CDP response).
    ExecutionFailed(String),
    /// Exit code 2 — invalid or missing command-line arguments.
    InvalidArgs(String),
    /// Exit code 3 — the operation did not complete within its timeout.
    Timeout(String),
    /// Exit code 4 — could not connect to (or launch) the browser's debugger.
    ConnectionFailed(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::ExecutionFailed(_) => 1,
            CliError::InvalidArgs(_) => 2,
            CliError::Timeout(_) => 3,
            CliError::ConnectionFailed(_) => 4,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            CliError::ExecutionFailed(m) => m,
            CliError::InvalidArgs(m) => m,
            CliError::Timeout(m) => m,
            CliError::ConnectionFailed(m) => m,
        };
        write!(f, "{}", msg)
    }
}

/// Return the value following flag `args[i]`, or an `InvalidArgs` error.
pub fn expect_value(args: &[String], i: usize, flag: &str) -> Result<String, CliError> {
    args.get(i)
        .cloned()
        .ok_or_else(|| CliError::InvalidArgs(format!("{} requires a value", flag)))
}

/// Parse the value following flag `args[i]` into `T`, or an `InvalidArgs` error.
pub fn parse_value<T: std::str::FromStr>(args: &[String], i: usize, flag: &str) -> Result<T, CliError> {
    expect_value(args, i, flag)?
        .parse()
        .map_err(|_| CliError::InvalidArgs(format!("invalid value for {}", flag)))
}

/// Attach to a running session via `--port` (default 9222) or, with the
/// `android` feature, `--package` for a debuggable Android WebView.
/// `port` and `package` are mutually exclusive; validated by the caller.
pub fn attach(port: Option<u16>, package: Option<&str>) -> Result<cdp_client::CdpClient, CliError> {
    if let Some(pkg) = package {
        return attach_android(pkg);
    }
    cdp_client::CdpClient::attach(port.unwrap_or(9222)).map_err(CliError::ConnectionFailed)
}

#[cfg(feature = "android")]
fn attach_android(package: &str) -> Result<cdp_client::CdpClient, CliError> {
    cdp_client::CdpClient::attach_android(package).map_err(CliError::ConnectionFailed)
}

#[cfg(not(feature = "android"))]
fn attach_android(_package: &str) -> Result<cdp_client::CdpClient, CliError> {
    Err(CliError::InvalidArgs(
        "--package requires building browse with `--features android`".to_string(),
    ))
}

/// Reject `--port`/`--package` given together; returns the effective port
/// (only meaningful when `package` is `None`).
pub fn validate_connect_args(port: Option<u16>, package: &Option<String>) -> Result<(), CliError> {
    if port.is_some() && package.is_some() {
        return Err(CliError::InvalidArgs(
            "--port and --package cannot be used together".to_string(),
        ));
    }
    Ok(())
}
