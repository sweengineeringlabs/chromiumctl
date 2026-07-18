use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut url: Option<String> = None;

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
            "--url" => {
                i += 1;
                url = Some(expect_value(args, i, "--url")?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let url = url.ok_or_else(|| CliError::InvalidArgs("--url is required".to_string()))?;

    let mut client = attach(port, package.as_deref())?;
    client.navigate(&url).map_err(CliError::ExecutionFailed)?;

    println!("Navigated to {}", url);
    Ok(())
}
