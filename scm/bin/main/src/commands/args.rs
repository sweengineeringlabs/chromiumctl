use super::CliError;

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
