use cdp_client::PageEvaluator;

use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut selector: Option<String> = None;
    let mut text: Option<String> = None;

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
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let selector = selector.ok_or_else(|| CliError::InvalidArgs("--selector is required".to_string()))?;
    let text = text.ok_or_else(|| CliError::InvalidArgs("--text is required".to_string()))?;

    let client = attach(port, package.as_deref())?;

    let selector_json = serde_json::to_string(&selector).map_err(|e| CliError::InvalidArgs(e.to_string()))?;
    let focused = client
        .evaluate(&format!(
            "(function() {{ \
                {deep_query_selector} \
                var el = __chromiumctl_deepQuerySelector(document, {selector}); \
                if (!el) return 'no'; \
                el.focus(); \
                return 'yes'; \
            }})()",
            deep_query_selector = cdp_client::deep_query_selector_js(),
            selector = selector_json,
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
