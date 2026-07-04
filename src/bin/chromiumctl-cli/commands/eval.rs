use chromiumctl::{CdpClient, PageEvaluator};

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: u16 = 9222;
    let mut script: Option<String> = None;
    let mut output_format = "text".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = parse_value(args, i, "--port")?;
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

    let script = script.ok_or_else(|| CliError::InvalidArgs("--script is required".to_string()))?;

    let client = CdpClient::attach(port).map_err(CliError::ConnectionFailed)?;
    let result = client.evaluate(&script).map_err(CliError::ExecutionFailed)?;

    match output_format.as_str() {
        "json" => println!("{}", serde_json::json!({ "result": result })),
        "yaml" => println!("result: {:?}", result),
        _ => println!("{}", result),
    }

    Ok(())
}
