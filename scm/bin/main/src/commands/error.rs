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
