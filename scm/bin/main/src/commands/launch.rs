use browsectl::CdpClientBuilder;

use crate::core::os_process::ProcessLocator;
use crate::core::session::{now_unix_secs, SessionRecord, SessionStore};

use super::{expect_value, parse_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut url: Option<String> = None;
    let mut port: u16 = 9222;
    let mut width: u32 = 1920;
    let mut height: u32 = 1080;
    let mut reap_stale = false;

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
            "--reap-stale" => reap_stale = true,
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let url = url.ok_or_else(|| CliError::InvalidArgs("--url is required".to_string()))?;

    if reap_stale {
        // Caller-liveness only (no --max-age): bounds worst-case leak growth
        // without touching sessions whose caller might just be slow.
        let reaped = super::reap::reap_sessions(false, None);
        super::reap::print_outcomes(&reaped, false);
    }

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

    // Record this session so `reap` can find and close it if the caller
    // dies before `stop` is called. The "caller" tracked here is *our own
    // parent* process, not this `launch` invocation itself — `launch`
    // always exits right after this point (that's the whole reason the
    // browser gets detached), so its own PID would already be dead by the
    // time anything could check it. The process actually expected to stay
    // alive and eventually call `stop` is whatever spawned `launch`.
    // Best-effort: the browser is already up and usable, so a bookkeeping
    // failure here must not fail `launch`.
    let own_pid = std::process::id();
    let caller_pid = ProcessLocator::parent_pid(own_pid).unwrap_or(own_pid);
    let record = SessionRecord {
        port: client.port(),
        launched_at: now_unix_secs(),
        caller_pid,
        caller_start_time: ProcessLocator::start_time_fingerprint(caller_pid),
    };
    if let Err(e) = SessionStore::write(&record) {
        eprintln!("Warning: failed to record session for reap: {}", e);
    }

    // Detach: skip Drop so the spawned Chromium process outlives this CLI
    // invocation instead of being killed when `client` goes out of scope.
    std::mem::forget(client);

    Ok(())
}
