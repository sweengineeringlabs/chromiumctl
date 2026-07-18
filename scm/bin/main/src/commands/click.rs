use cdp_client::PageEvaluator;

use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut selector: Option<String> = None;

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
            "--selector" => {
                i += 1;
                selector = Some(expect_value(args, i, "--selector")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let selector = selector.ok_or_else(|| CliError::InvalidArgs("--selector is required".to_string()))?;

    let client = attach(port, package.as_deref())?;

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
