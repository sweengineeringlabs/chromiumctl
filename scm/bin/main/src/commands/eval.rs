use cdp_client::PageEvaluator;

use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut script: Option<String> = None;
    let mut output_format = "text".to_string();

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
            "--script" => {
                i += 1;
                script = Some(expect_value(args, i, "--script")?);
            }
            "--output" => {
                i += 1;
                output_format = expect_value(args, i, "--output")?;
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let script = script.ok_or_else(|| CliError::InvalidArgs("--script is required".to_string()))?;

    let client = attach(port, package.as_deref())?;
    let result = client.evaluate(&script).map_err(CliError::ExecutionFailed)?;

    match output_format.as_str() {
        "json" => {
            let value: serde_json::Value = serde_json::from_str(&result)
                .unwrap_or_else(|_| serde_json::Value::String(result.clone()));
            println!("{}", serde_json::json!({ "result": value }));
        }
        "yaml" => println!("result: {:?}", result),
        _ => println!("{}", result),
    }

    Ok(())
}
