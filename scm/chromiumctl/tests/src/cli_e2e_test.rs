// E2e tests for the `chromiumctl-cli` binary.
//
// Tests that don't need a running browser (arg validation, connection
// failures) run by default. Tests that drive a real Chromium instance
// require a Chromium-based browser. Run with:
//   cargo test --test cli_e2e_test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used, clippy::expect_used)]

use chromiumctl::{CdpClient, CdpClientBuilder, PageEvaluator};
use std::process::Command;
use std::sync::atomic::{AtomicU16, Ordering};

static NEXT_TEST_PORT: AtomicU16 = AtomicU16::new(9400);

fn next_port() -> u16 {
    NEXT_TEST_PORT.fetch_add(1, Ordering::Relaxed)
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_chromiumctl-cli"))
}

fn unique_temp_file(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("chromiumctl_cli_test_{}_{}", std::process::id(), name))
}

// ---------------------------------------------------------------------------
// eval
// ---------------------------------------------------------------------------

/// @covers: eval
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_eval_prints_real_js_result() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<h1 id=x>Hello</h1>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args([
            "eval",
            "--port", &port.to_string(),
            "--script", "document.getElementById('x').textContent",
        ])
        .output()
        .expect("failed to run chromiumctl-cli eval");

    assert!(
        output.status.success(),
        "eval must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "Hello");

    drop(client);
}

/// @covers: eval
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_eval_returns_exit_1_on_js_exception() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>test</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["eval", "--port", &port.to_string(), "--script", "throw new Error('boom')"])
        .output()
        .expect("failed to run chromiumctl-cli eval");

    assert_eq!(output.status.code(), Some(1), "JS exceptions must exit 1 (execution failed)");
    drop(client);
}

/// @covers: eval
#[test]
fn test_eval_returns_exit_4_when_no_browser_listening() {
    let output = cli()
        .args(["eval", "--port", "19998", "--script", "1"])
        .output()
        .expect("failed to run chromiumctl-cli eval");
    assert_eq!(output.status.code(), Some(4), "unreachable debugger must exit 4 (connection failed)");
}

/// @covers: eval
#[test]
fn test_eval_returns_exit_2_when_script_missing() {
    let output = cli()
        .args(["eval", "--port", "9222"])
        .output()
        .expect("failed to run chromiumctl-cli eval");
    assert_eq!(output.status.code(), Some(2), "missing --script must exit 2 (invalid args)");
}

/// @covers: eval
#[test]
fn test_eval_returns_exit_2_when_port_and_package_both_given() {
    let output = cli()
        .args(["eval", "--port", "9222", "--package", "com.example.app", "--script", "1"])
        .output()
        .expect("failed to run chromiumctl-cli eval");
    assert_eq!(
        output.status.code(),
        Some(2),
        "--port and --package together must exit 2 (invalid args)"
    );
}

/// @covers: eval
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_eval_output_json_preserves_boolean_type() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>test</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["eval", "--port", &port.to_string(), "--script", "true", "--output", "json"])
        .output()
        .expect("failed to run chromiumctl-cli eval");

    assert!(output.status.success(), "eval must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(parsed["result"], serde_json::json!(true), "boolean result must be a native JSON boolean, not a string");

    drop(client);
}

/// @covers: eval
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_eval_output_json_wraps_non_json_string_as_string() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>test</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["eval", "--port", &port.to_string(), "--script", "'count:5'", "--output", "json"])
        .output()
        .expect("failed to run chromiumctl-cli eval");

    assert!(output.status.success(), "eval must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).expect("stdout must be valid JSON");
    assert_eq!(
        parsed["result"],
        serde_json::json!("count:5"),
        "a non-JSON-parseable string must still be wrapped as a JSON string"
    );

    drop(client);
}

/// @covers: eval
#[test]
#[cfg(not(feature = "android"))]
fn test_eval_package_gives_actionable_error_without_android_feature() {
    let output = cli()
        .args(["eval", "--package", "com.example.app", "--script", "1"])
        .output()
        .expect("failed to run chromiumctl-cli eval");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--features android"),
        "error must tell the caller how to fix it, got: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// navigate
// ---------------------------------------------------------------------------

/// @covers: navigate
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_navigate_changes_real_page_content() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>start</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args([
            "navigate",
            "--port", &port.to_string(),
            "--url", "data:text/html,<p id=x>navigated</p>",
        ])
        .output()
        .expect("failed to run chromiumctl-cli navigate");

    assert!(output.status.success(), "navigate must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    let found = client.evaluate("document.getElementById('x') !== null ? 'yes' : 'no'").unwrap();
    assert_eq!(found, "yes", "page content must reflect the real navigation");

    drop(client);
}

/// @covers: navigate
#[test]
fn test_navigate_returns_exit_2_when_url_missing() {
    let output = cli().args(["navigate", "--port", "9222"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

// ---------------------------------------------------------------------------
// wait
// ---------------------------------------------------------------------------

/// @covers: wait
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_wait_succeeds_when_selector_present() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<button id=btn>ok</button>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["wait", "--port", &port.to_string(), "--selector", "#btn", "--timeout", "5"])
        .output()
        .expect("failed to run chromiumctl-cli wait");

    assert!(output.status.success(), "wait must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    drop(client);
}

/// @covers: wait
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_wait_times_out_when_selector_absent() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>empty</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["wait", "--port", &port.to_string(), "--selector", "#does-not-exist", "--timeout", "1"])
        .output()
        .expect("failed to run chromiumctl-cli wait");

    assert_eq!(output.status.code(), Some(3), "an unmet condition must exit 3 (timeout)");
    drop(client);
}

/// @covers: wait
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_wait_succeeds_for_navigation_condition() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>loaded</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["wait", "--port", &port.to_string(), "--navigation", "--timeout", "5"])
        .output()
        .expect("failed to run chromiumctl-cli wait");

    assert!(
        output.status.success(),
        "wait --navigation must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(client);
}

/// @covers: wait
#[test]
fn test_wait_returns_exit_2_when_no_condition_given() {
    let output = cli().args(["wait", "--port", "9222"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "missing --selector/--text/--navigation must exit 2");
}

// ---------------------------------------------------------------------------
// click / input
// ---------------------------------------------------------------------------

/// @covers: click
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_click_dispatches_real_mouse_event() {
    let port = next_port();
    let client = CdpClientBuilder::new(
        "data:text/html,<button id=btn onclick=\"document.title='clicked'\">go</button>",
    )
    .port(port)
    .launch()
    .expect("setup: launch must succeed");

    let output = cli()
        .args(["click", "--port", &port.to_string(), "--selector", "#btn"])
        .output()
        .expect("failed to run chromiumctl-cli click");

    assert!(output.status.success(), "click must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(client.evaluate("document.title").unwrap(), "clicked");

    drop(client);
}

/// @covers: click
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_click_returns_exit_1_when_selector_not_found() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>empty</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["click", "--port", &port.to_string(), "--selector", "#does-not-exist"])
        .output()
        .expect("failed to run chromiumctl-cli click");

    assert_eq!(output.status.code(), Some(1));
    drop(client);
}

/// @covers: input
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_input_types_real_text_into_field() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<input id=box type=text>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["input", "--port", &port.to_string(), "--selector", "#box", "--text", "hello world"])
        .output()
        .expect("failed to run chromiumctl-cli input");

    assert!(output.status.success(), "input must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(client.evaluate("document.getElementById('box').value").unwrap(), "hello world");

    drop(client);
}

// ---------------------------------------------------------------------------
// screenshot
// ---------------------------------------------------------------------------

/// @covers: screenshot
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_screenshot_writes_real_png_file() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<h1 style='color:red'>shot</h1>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let out_path = unique_temp_file("shot.png");
    let output = cli()
        .args(["screenshot", "--port", &port.to_string(), "--output", out_path.to_str().unwrap()])
        .output()
        .expect("failed to run chromiumctl-cli screenshot");

    assert!(output.status.success(), "screenshot must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));

    let bytes = std::fs::read(&out_path).expect("screenshot file must exist");
    assert_eq!(&bytes[0..8], b"\x89PNG\r\n\x1a\n", "output file must be a real PNG");

    let _ = std::fs::remove_file(&out_path);
    drop(client);
}

// ---------------------------------------------------------------------------
// get-dom
// ---------------------------------------------------------------------------

/// @covers: get-dom
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_get_dom_exports_real_document_tree() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<h1 id=x>dom-test</h1>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let out_path = unique_temp_file("dom.json");
    let output = cli()
        .args(["get-dom", "--port", &port.to_string(), "--output", out_path.to_str().unwrap()])
        .output()
        .expect("failed to run chromiumctl-cli get-dom");

    assert!(output.status.success(), "get-dom must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));

    let raw = std::fs::read_to_string(&out_path).expect("DOM file must exist");
    let dom: serde_json::Value = serde_json::from_str(&raw).expect("DOM file must be valid JSON");
    assert_eq!(dom["root"]["nodeName"], "#document", "must be a real CDP DOM.getDocument tree");

    let _ = std::fs::remove_file(&out_path);
    drop(client);
}

// ---------------------------------------------------------------------------
// metrics
// ---------------------------------------------------------------------------

/// @covers: metrics
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_metrics_returns_real_performance_metrics() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>metrics</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let out_path = unique_temp_file("metrics.json");
    let output = cli()
        .args(["metrics", "--port", &port.to_string(), "--output", out_path.to_str().unwrap()])
        .output()
        .expect("failed to run chromiumctl-cli metrics");

    assert!(output.status.success(), "metrics must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));

    let raw = std::fs::read_to_string(&out_path).expect("metrics file must exist");
    let metrics: serde_json::Value = serde_json::from_str(&raw).expect("metrics file must be valid JSON");
    let has_timestamp = metrics["metrics"]
        .as_array()
        .expect("metrics field must be an array")
        .iter()
        .any(|m| m["name"] == "Timestamp");
    assert!(has_timestamp, "must contain a real CDP Performance.getMetrics 'Timestamp' entry");

    let _ = std::fs::remove_file(&out_path);
    drop(client);
}

// ---------------------------------------------------------------------------
// launch
// ---------------------------------------------------------------------------

/// @covers: launch
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_launch_starts_real_reachable_browser_and_survives_cli_exit() {
    let port = next_port();

    let output = cli()
        .args([
            "launch",
            "--url", "data:text/html,<h1 id=y>from-launch</h1>",
            "--port", &port.to_string(),
            "--width", "800",
            "--height", "600",
        ])
        .output()
        .expect("failed to run chromiumctl-cli launch");

    assert!(output.status.success(), "launch must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));

    // The CLI process has already exited; the browser must still be reachable —
    // proving it was actually detached rather than killed with the CLI process.
    let client = CdpClient::attach(port).expect("browser must remain reachable after the launch command exits");
    assert_eq!(
        client.evaluate("document.getElementById('y').textContent").unwrap(),
        "from-launch"
    );
    let (width, height) = client.get_viewport_size().unwrap();
    assert_eq!((width, height), (800, 600), "--width/--height must apply to the real viewport");

    // Clean up the detached process via CDP itself — no OS process tracking needed.
    let _ = client.send("Browser.close", serde_json::json!({}));
}

/// @covers: launch
#[test]
fn test_launch_returns_exit_2_when_url_missing() {
    let output = cli().args(["launch", "--port", "9222"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

// ---------------------------------------------------------------------------
// stop
// ---------------------------------------------------------------------------

/// @covers: stop
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_stop_terminates_real_detached_launch_and_leaves_others_running() {
    let target_port = next_port();
    let bystander_port = next_port();

    // A `launch`ed session, detached exactly like a real caller would use it.
    let launch_output = cli()
        .args([
            "launch",
            "--url", "data:text/html,<h1>target</h1>",
            "--port", &target_port.to_string(),
        ])
        .output()
        .expect("failed to run chromiumctl-cli launch");
    assert!(launch_output.status.success(), "setup: launch must succeed");

    // A second, independent instance that `stop` must NOT touch.
    let bystander = CdpClientBuilder::new("data:text/html,<h1>bystander</h1>")
        .port(bystander_port)
        .launch()
        .expect("setup: bystander launch must succeed");

    let stop_output = cli()
        .args(["stop", "--port", &target_port.to_string()])
        .output()
        .expect("failed to run chromiumctl-cli stop");
    assert!(
        stop_output.status.success(),
        "stop must exit 0, stderr: {}",
        String::from_utf8_lossy(&stop_output.stderr)
    );

    assert!(
        CdpClient::attach(target_port).is_err(),
        "the targeted browser must no longer be reachable after stop"
    );
    assert!(
        bystander.evaluate("1").is_ok(),
        "an unrelated browser instance must survive stopping a different port"
    );

    drop(bystander);
}

/// @covers: stop
#[test]
fn test_stop_returns_exit_4_when_no_browser_listening() {
    let output = cli().args(["stop", "--port", "19997"]).output().unwrap();
    assert_eq!(output.status.code(), Some(4), "unreachable debugger must exit 4 (connection failed)");
}

/// @covers: stop
#[test]
fn test_stop_returns_exit_2_when_port_and_package_both_given() {
    let output = cli()
        .args(["stop", "--port", "9222", "--package", "com.example.app"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "--port and --package together must exit 2 (invalid args)"
    );
}
