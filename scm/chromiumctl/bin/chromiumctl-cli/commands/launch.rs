use chromiumctl::CdpClientBuilder;

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut url: Option<String> = None;
    let mut port: u16 = 9222;
    let mut width: u32 = 1920;
    let mut height: u32 = 1080;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                i += 1;
                url = Some(expect_value(args, i, "--url")?);
            }
            "--port" => {
                i += 1;
                port = parse_value(args, i, "--port")?;
            }
            "--headless" => {}
            "--width" => {
                i += 1;
                width = parse_value(args, i, "--width")?;
            }
            "--height" => {
                i += 1;
                height = parse_value(args, i, "--height")?;
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let url = url.ok_or_else(|| CliError::InvalidArgs("--url is required".to_string()))?;

    let client = CdpClientBuilder::new(&url)
        .port(port)
        .launch()
        .map_err(CliError::ConnectionFailed)?;

    client
        .send(
            "Emulation.setDeviceMetricsOverride",
            serde_json::json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": 1,
                "mobile": false,
            }),
        )
        .map_err(CliError::ExecutionFailed)?;

    println!("Browser launched.");
    println!("  URL: {}", url);
    println!("  Port: {}", client.port());
    println!("  Viewport: {}x{}", width, height);
    println!("  DevTools: {}", client.ws_url());
    println!(
        "\nUse --port {} with other commands to control this session.",
        client.port()
    );

    // Detach: skip Drop so the spawned Chromium process outlives this CLI
    // invocation instead of being killed when `client` goes out of scope.
    std::mem::forget(client);

    Ok(())
}
