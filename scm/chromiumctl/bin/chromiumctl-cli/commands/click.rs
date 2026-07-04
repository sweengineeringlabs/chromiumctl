use chromiumctl::{CdpClient, PageEvaluator};

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: u16 = 9222;
    let mut selector: Option<String> = None;

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
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let selector = selector.ok_or_else(|| CliError::InvalidArgs("--selector is required".to_string()))?;

    let client = CdpClient::attach(port).map_err(CliError::ConnectionFailed)?;

    let rect = client
        .get_bounding_rect(&selector)
        .map_err(CliError::ExecutionFailed)?;
    let x = rect.x + rect.width / 2.0;
    let y = rect.y + rect.height / 2.0;

    client
        .send(
            "Input.dispatchMouseEvent",
            serde_json::json!({ "type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1 }),
        )
        .map_err(CliError::ExecutionFailed)?;
    client
        .send(
            "Input.dispatchMouseEvent",
            serde_json::json!({ "type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1 }),
        )
        .map_err(CliError::ExecutionFailed)?;

    println!("Clicked {} at ({:.0}, {:.0})", selector, x, y);
    Ok(())
}
