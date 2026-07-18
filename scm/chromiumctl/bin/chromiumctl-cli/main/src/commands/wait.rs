use browsectl::PageEvaluator;
use std::time::{Duration, Instant};

use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut selector: Option<String> = None;
    let mut text: Option<String> = None;
    let mut navigation = false;
    let mut timeout_secs: u64 = 30;

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
            "--text" => {
                i += 1;
                text = Some(expect_value(args, i, "--text")?);
            }
            "--navigation" => navigation = true,
            "--timeout" => {
                i += 1;
                timeout_secs = parse_value(args, i, "--timeout")?;
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let condition_js = if let Some(sel) = &selector {
        format!(
            "(function() {{ {deep_query_selector} return __chromiumctl_deepQuerySelector(document, {selector}) !== null; }})()",
            deep_query_selector = browsectl::deep_query_selector_js(),
            selector = json_string(sel)?,
        )
    } else if let Some(txt) = &text {
        format!(
            "document.body !== null && document.body.innerText.includes({})",
            json_string(txt)?
        )
    } else if navigation {
        "document.readyState === 'complete'".to_string()
    } else {
        return Err(CliError::InvalidArgs(
            "--selector, --text, or --navigation is required".to_string(),
        ));
    };

    let client = attach(port, package.as_deref())?;

    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let found = client.evaluate(&condition_js).map_err(CliError::ExecutionFailed)?;
        if found == "true" {
            println!("Condition met.");
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(CliError::Timeout(format!(
                "condition not met within {}s",
                timeout_secs
            )));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// JSON-encode a string so it can be safely embedded as a JS string literal.
fn json_string(s: &str) -> Result<String, CliError> {
    serde_json::to_string(s).map_err(|e| CliError::InvalidArgs(e.to_string()))
}
