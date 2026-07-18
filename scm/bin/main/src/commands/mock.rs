use std::time::Duration;

use super::{attach, expect_value, parse_value, validate_connect_args, CliError};

/// How long to wait for the next matching request before giving up. Not
/// truly unbounded: each wait is capped, even though the overall command is
/// designed to keep looping (like a dev server) until the user interrupts
/// it (Ctrl-C) or a full hour passes with no activity at all.
const REQUEST_WAIT_TIMEOUT: Duration = Duration::from_secs(3600);

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut port: Option<u16> = None;
    let mut package: Option<String> = None;
    let mut url_pattern: Option<String> = None;
    let mut status: u16 = 200;
    let mut body = String::new();

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
            "--url-pattern" => {
                i += 1;
                url_pattern = Some(expect_value(args, i, "--url-pattern")?);
            }
            "--status" => {
                i += 1;
                status = parse_value(args, i, "--status")?;
            }
            "--body" => {
                i += 1;
                body = expect_value(args, i, "--body")?;
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }
    validate_connect_args(port, &package)?;

    let url_pattern =
        url_pattern.ok_or_else(|| CliError::InvalidArgs("--url-pattern is required".to_string()))?;

    let client = attach(port, package.as_deref())?;

    // Chromium only pauses requests matching this pattern — everything else
    // proceeds completely untouched, with no interception overhead. No
    // manual pattern matching or explicit "continue the non-matches" logic
    // needed on our side.
    client
        .send(
            "Fetch.enable",
            serde_json::json!({ "patterns": [{ "urlPattern": url_pattern }] }),
        )
        .map_err(CliError::ExecutionFailed)?;

    println!(
        "Mocking requests matching '{}' with status {} ({} byte body).",
        url_pattern,
        status,
        body.len()
    );
    println!("Blocking until interrupted (Ctrl-C) or {:?} of inactivity.", REQUEST_WAIT_TIMEOUT);

    let body_b64 = data_encoding::BASE64.encode(body.as_bytes());

    loop {
        let event = client
            .wait_for_event("Fetch.requestPaused", REQUEST_WAIT_TIMEOUT)
            .map_err(CliError::ExecutionFailed)?;

        let request_id = event["requestId"].as_str().ok_or_else(|| {
            CliError::ExecutionFailed("Fetch.requestPaused event missing requestId".to_string())
        })?;
        let url = event["request"]["url"].as_str().unwrap_or("<unknown>");

        client
            .send(
                "Fetch.fulfillRequest",
                serde_json::json!({
                    "requestId": request_id,
                    "responseCode": status,
                    "body": body_b64,
                    // A fulfilled response goes through the same CORS
                    // pipeline as a real one, so a cross-origin fetch (the
                    // motivating case — mocking a third-party API like
                    // sts.amazonaws.com) still fails with a generic
                    // "Failed to fetch" without this, even though the CDP
                    // call itself succeeds. Permissive by default since a
                    // mock is explicitly opt-in, page-scoped, and the
                    // caller already chose to fake this exact response.
                    "responseHeaders": [
                        { "name": "Access-Control-Allow-Origin", "value": "*" },
                    ],
                }),
            )
            .map_err(CliError::ExecutionFailed)?;

        println!("Mocked {} -> {}", url, status);
    }
}
