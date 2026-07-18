use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut output: Option<String> = None;

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
                output = Some(expect_value(args, i, "--output")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let client = attach(port, package.as_deref())?;

    client
        .send("Performance.enable", serde_json::json!({}))
        .map_err(CliError::ExecutionFailed)?;
    let metrics = client
        .send("Performance.getMetrics", serde_json::json!({}))
        .map_err(CliError::ExecutionFailed)?;

    let json = serde_json::to_string_pretty(&metrics)
        .map_err(|e| CliError::ExecutionFailed(format!("failed to serialize metrics: {}", e)))?;

    match output {
        Some(path) => {
            std::fs::write(&path, &json)
                .map_err(|e| CliError::ExecutionFailed(format!("failed to write '{}': {}", path, e)))?;
            println!("Metrics exported to {}", path);
        }
        None => println!("{}", json),
    }

    Ok(())
}
