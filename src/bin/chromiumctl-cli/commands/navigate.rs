use chromiumctl::CdpClient;

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: u16 = 9222;
    let mut url: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = parse_value(args, i, "--port")?;
            }
            "--url" => {
                i += 1;
                url = Some(expect_value(args, i, "--url")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let url = url.ok_or_else(|| CliError::InvalidArgs("--url is required".to_string()))?;

    let mut client = CdpClient::attach(port).map_err(CliError::ConnectionFailed)?;
    client.navigate(&url).map_err(CliError::ExecutionFailed)?;

    println!("Navigated to {}", url);
    Ok(())
}
