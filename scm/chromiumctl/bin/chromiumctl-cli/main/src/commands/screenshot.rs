use chromiumctl::CdpClient;

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: u16 = 9222;
    let mut output = "screenshot.png".to_string();
    let mut format = "png".to_string();
    let mut full_page = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = parse_value(args, i, "--port")?;
            }
            "--output" => {
                i += 1;
                output = expect_value(args, i, "--output")?;
            }
            "--format" => {
                i += 1;
                format = expect_value(args, i, "--format")?;
            }
            "--full-page" => full_page = true,
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let client = CdpClient::attach(port).map_err(CliError::ConnectionFailed)?;

    let result = client
        .send(
            "Page.captureScreenshot",
            serde_json::json!({
                "format": format,
                "captureBeyondViewport": full_page,
            }),
        )
        .map_err(CliError::ExecutionFailed)?;

    let base64_data = result["data"].as_str().ok_or_else(|| {
        CliError::ExecutionFailed("screenshot response missing 'data' field".to_string())
    })?;

    let bytes = data_encoding::BASE64
        .decode(base64_data.as_bytes())
        .map_err(|e| CliError::ExecutionFailed(format!("failed to decode screenshot data: {}", e)))?;

    std::fs::write(&output, &bytes)
        .map_err(|e| CliError::ExecutionFailed(format!("failed to write '{}': {}", output, e)))?;

    println!("Screenshot saved to {}", output);
    Ok(())
}
