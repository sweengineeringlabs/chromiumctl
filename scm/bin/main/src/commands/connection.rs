use super::CliError;

/// Attach to a running session via `--port` (default 9222) or, with the
/// `android` feature, `--package` for a debuggable Android WebView.
/// `port` and `package` are mutually exclusive; validated by the caller.
pub fn attach(port: Option<u16>, package: Option<&str>) -> Result<browsectl::CdpClient, CliError> {
    if let Some(pkg) = package {
        return attach_android(pkg);
    }
    browsectl::CdpClient::attach(port.unwrap_or(9222)).map_err(CliError::ConnectionFailed)
}

#[cfg(feature = "android")]
fn attach_android(package: &str) -> Result<browsectl::CdpClient, CliError> {
    browsectl::CdpClient::attach_android(package).map_err(CliError::ConnectionFailed)
}

#[cfg(not(feature = "android"))]
fn attach_android(_package: &str) -> Result<browsectl::CdpClient, CliError> {
    Err(CliError::InvalidArgs(
        "--package requires building browse with `--features android`".to_string(),
    ))
}
