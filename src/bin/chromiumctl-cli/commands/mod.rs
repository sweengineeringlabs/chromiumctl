pub mod launch;
pub mod eval;
pub mod screenshot;
pub mod navigate;
pub mod wait;
pub mod click;
pub mod input;
pub mod get_dom;
pub mod metrics;

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
