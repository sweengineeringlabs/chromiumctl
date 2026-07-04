use chromiumctl::{CdpClient, PageEvaluator};

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: u16 = 9222;
    let mut selector: Option<String> = None;
    let mut text: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = parse_value(args, i, "--port")?;
            }
            "--selector" => {
                i += 1;
                selector = Some(expect_value(args, i, "--selector")?);
            }
            "--text" => {
                i += 1;
                text = Some(expect_value(args, i, "--text")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let selector = selector.ok_or_else(|| CliError::InvalidArgs("--selector is required".to_string()))?;
    let text = text.ok_or_else(|| CliError::InvalidArgs("--text is required".to_string()))?;

    let client = CdpClient::attach(port).map_err(CliError::ConnectionFailed)?;

    let selector_json = serde_json::to_string(&selector).map_err(|e| CliError::InvalidArgs(e.to_string()))?;
    let focused = client
        .evaluate(&format!(
            "(function() {{ \
                var el = document.querySelector({}); \
                if (!el) return 'no'; \
                el.focus(); \
                return 'yes'; \
            }})()",
            selector_json
        ))
        .map_err(CliError::ExecutionFailed)?;
    if focused != "yes" {
        return Err(CliError::ExecutionFailed(format!("element not found: {}", selector)));
    }

    client
        .send("Input.insertText", serde_json::json!({ "text": text }))
        .map_err(CliError::ExecutionFailed)?;

    println!("Typed into {}", selector);
    Ok(())
}
