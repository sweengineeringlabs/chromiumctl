use chromiumctl::CdpClient;

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: u16 = 9222;
    let mut output: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = parse_value(args, i, "--port")?;
            }
            "--output" => {
                i += 1;
                output = Some(expect_value(args, i, "--output")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let client = CdpClient::attach(port).map_err(CliError::ConnectionFailed)?;

    let dom = client
        .send("DOM.getDocument", serde_json::json!({ "depth": -1, "pierce": true }))
        .map_err(CliError::ExecutionFailed)?;

    let json = serde_json::to_string_pretty(&dom)
        .map_err(|e| CliError::ExecutionFailed(format!("failed to serialize DOM: {}", e)))?;

    match output {
        Some(path) => {
            std::fs::write(&path, &json)
                .map_err(|e| CliError::ExecutionFailed(format!("failed to write '{}': {}", path, e)))?;
            println!("DOM exported to {}", path);
        }
        None => println!("{}", json),
    }

    Ok(())
}
