use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut output = "screenshot.png".to_string();
    let mut format = "png".to_string();
    let mut full_page = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = Some(parse_value(args, i, "--port")?);
            }
            "--package" => {
                i += 1;
                package = Some(expect_value(args, i, "--package")?);
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
    validate_connect_args(port, &package)?;

    let client = attach(port, package.as_deref())?;

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
