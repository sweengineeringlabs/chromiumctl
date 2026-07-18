// E2e tests for the `browse` binary.
//
// Tests that don't need a running browser (arg validation, connection
// failures) run by default. Tests that drive a real Chromium instance
// require a Chromium-based browser. Run with:
//   cargo test --test cli_e2e_test -- --ignored --test-threads=1
#![allow(clippy::unwrap_used, clippy::expect_used)]

use browsectl::{CdpClient, CdpClientBuilder, PageEvaluator};
use std::process::Command;
use std::sync::atomic::{AtomicU16, Ordering};

static NEXT_TEST_PORT: AtomicU16 = AtomicU16::new(9400);

fn next_port() -> u16 {
    NEXT_TEST_PORT.fetch_add(1, Ordering::Relaxed)
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_browse"))
}

fn unique_temp_file(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("chromiumctl_cli_test_{}_{}", std::process::id(), name))
}

/// A page with a button inside an *open* shadow root, wired to set
/// `document.title` on click — used to prove shadow-piercing works through
/// the real CLI dispatch path (`click`/`wait --selector`), not just the
/// library's `PageEvaluator` trait directly.
fn shadow_button_fixture_url() -> &'static str {
    r#"data:text/html,<div id="host"></div><script>
        var root = document.getElementById('host').attachShadow({mode: 'open'});
        var btn = document.createElement('button');
        btn.id = 'shadow-btn';
        btn.textContent = 'go';
        btn.addEventListener('click', function() { document.title = 'clicked'; });
        root.appendChild(btn);
    </script>"#
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
#[ignore = "requires a running Chromium instance"]
fn test_eval_prints_resolved_value_of_async_iife() {
    let port = next_port();
    let client = CdpClientBuilder::new("data:text/html,<p>test</p>")
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args([
            "eval",
            "--port", &port.to_string(),
            "--script",
            "(function(){ return new Promise(function(resolve){ setTimeout(function(){ resolve('some-value'); }, 200); }); })()",
        ])
        .output()
        .expect("failed to run chromiumctl-cli eval");

    assert!(
        output.status.success(),
        "eval must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "some-value",
        "eval must print the promise's resolved value, not exit silently"
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
fn test_wait_selector_with_embedded_apostrophe_does_not_break() {
    let port = next_port();
    let client = CdpClientBuilder::new(r#"data:text/html,<button id=btn data-x="a">ok</button>"#)
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    // wait.rs already had correct escaping (its own local json_string())
    // before shadow-piercing was wired in alongside it — this confirms
    // that pre-existing correctness wasn't disturbed by that change.
    let output = cli()
        .args(["wait", "--port", &port.to_string(), "--selector", "[data-x='a']", "--timeout", "5"])
        .output()
        .expect("failed to run chromiumctl-cli wait");

    assert!(
        output.status.success(),
        "a selector with a literal ' must not break wait --selector, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

/// @covers: wait
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_wait_selector_finds_element_inside_open_shadow_root() {
    let port = next_port();
    let client = CdpClientBuilder::new(shadow_button_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args(["wait", "--port", &port.to_string(), "--selector", "#shadow-btn", "--timeout", "5"])
        .output()
        .expect("failed to run chromiumctl-cli wait");

    assert!(
        output.status.success(),
        "wait --selector must pierce into an open shadow root, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(client);
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
fn test_click_dispatches_real_mouse_event_inside_open_shadow_root() {
    let port = next_port();
    let client = CdpClientBuilder::new(shadow_button_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    // Confirms the fixture actually exercises shadow piercing: a plain
    // querySelector must NOT see the button before `click` is even run.
    assert_eq!(
        client.evaluate("document.querySelector('#shadow-btn') === null ? 'blind' : 'sees-it'").unwrap(),
        "blind",
        "fixture setup: plain querySelector must NOT see into the shadow root"
    );

    let output = cli()
        .args(["click", "--port", &port.to_string(), "--selector", "#shadow-btn"])
        .output()
        .expect("failed to run chromiumctl-cli click");

    assert!(
        output.status.success(),
        "click must pierce into an open shadow root, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

/// @covers: input
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_input_selector_with_embedded_apostrophe_does_not_break() {
    let port = next_port();
    let client = CdpClientBuilder::new(r#"data:text/html,<input id=box type=text data-x="a">"#)
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    // input.rs already had correct escaping (serde_json::to_string) before
    // shadow-piercing was wired in alongside it — confirms that
    // pre-existing correctness wasn't disturbed by that change.
    let output = cli()
        .args(["input", "--port", &port.to_string(), "--selector", "[data-x='a']", "--text", "ok"])
        .output()
        .expect("failed to run chromiumctl-cli input");

    assert!(
        output.status.success(),
        "a selector with a literal ' must not break input, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(client.evaluate("document.getElementById('box').value").unwrap(), "ok");

    drop(client);
}

// ---------------------------------------------------------------------------
// set-files
// ---------------------------------------------------------------------------

fn file_input_fixture_url() -> &'static str {
    r#"data:text/html,<input type="file" id="file-input"><div id="result">none</div><script>
        document.getElementById('file-input').addEventListener('change', function(e) {
            var f = e.target.files[0];
            document.getElementById('result').textContent = f ? (f.name + ':' + f.size) : 'none';
        });
    </script>"#
}

fn multi_file_input_fixture_url() -> &'static str {
    r#"data:text/html,<input type="file" id="file-input" multiple><div id="result">none</div><script>
        document.getElementById('file-input').addEventListener('change', function(e) {
            var names = [];
            for (var i = 0; i < e.target.files.length; i++) {
                names.push(e.target.files[i].name + ':' + e.target.files[i].size);
            }
            document.getElementById('result').textContent = names.length + '|' + names.join(',');
        });
    </script>"#
}

/// @covers: set-files
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_set_files_sets_a_real_file_and_fires_change() {
    let port = next_port();
    let client = CdpClientBuilder::new(file_input_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let path = unique_temp_file("set_files_single.txt");
    std::fs::write(&path, b"hello set-files").expect("setup: fixture file must be writable");

    let output = cli()
        .args([
            "set-files",
            "--port", &port.to_string(),
            "--selector", "#file-input",
            "--files", path.to_str().expect("setup: path must be valid UTF-8"),
        ])
        .output()
        .expect("failed to run chromiumctl-cli set-files");

    assert!(
        output.status.success(),
        "set-files must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected_name = path.file_name().unwrap().to_str().unwrap();
    let result = client.evaluate("document.getElementById('result').textContent").unwrap();
    assert_eq!(
        result,
        format!("{}:{}", expected_name, "hello set-files".len()),
        "the input's change handler must see a real File with the real name and size — \
         proves DOM.setFileInputFiles fires 'change' natively, no manual dispatch needed"
    );

    drop(client);
}

/// @covers: set-files
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_set_files_sets_multiple_real_files_on_a_multiple_input() {
    let port = next_port();
    let client = CdpClientBuilder::new(multi_file_input_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let path_a = unique_temp_file("set_files_multi_a.txt");
    let path_b = unique_temp_file("set_files_multi_b.txt");
    std::fs::write(&path_a, b"aaa").expect("setup: fixture file a must be writable");
    std::fs::write(&path_b, b"bbbb").expect("setup: fixture file b must be writable");

    let files_arg = format!(
        "{},{}",
        path_a.to_str().expect("setup: path a must be valid UTF-8"),
        path_b.to_str().expect("setup: path b must be valid UTF-8"),
    );
    let output = cli()
        .args(["set-files", "--port", &port.to_string(), "--selector", "#file-input", "--files", &files_arg])
        .output()
        .expect("failed to run chromiumctl-cli set-files");

    assert!(
        output.status.success(),
        "set-files with multiple comma-separated paths must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = format!(
        "2|{}:3,{}:4",
        path_a.file_name().unwrap().to_str().unwrap(),
        path_b.file_name().unwrap().to_str().unwrap(),
    );
    let result = client.evaluate("document.getElementById('result').textContent").unwrap();
    assert_eq!(
        result, expected,
        "an <input multiple> must receive all comma-separated files, not just the first"
    );

    drop(client);
}

/// @covers: set-files
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_set_files_accepts_a_relative_path() {
    let port = next_port();
    let client = CdpClientBuilder::new(file_input_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let dir = std::env::temp_dir().join(format!("chromiumctl_cli_test_setfiles_relative_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("setup: temp dir must be creatable");
    std::fs::write(dir.join("relative.txt"), b"rel").expect("setup: fixture file must be writable");

    let output = cli()
        .args(["set-files", "--port", &port.to_string(), "--selector", "#file-input", "--files", "relative.txt"])
        .current_dir(&dir)
        .output()
        .expect("failed to run chromiumctl-cli set-files");

    assert!(
        output.status.success(),
        "a relative --files path must resolve against this process's cwd, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        client.evaluate("document.getElementById('result').textContent").unwrap(),
        "relative.txt:3"
    );

    drop(client);
}

/// @covers: set-files
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_set_files_returns_actionable_error_for_missing_file() {
    let port = next_port();
    let client = CdpClientBuilder::new(file_input_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let output = cli()
        .args([
            "set-files",
            "--port", &port.to_string(),
            "--selector", "#file-input",
            "--files", "this-file-does-not-exist-anywhere.tmp",
        ])
        .output()
        .expect("failed to run chromiumctl-cli set-files");

    assert_eq!(output.status.code(), Some(1), "a missing file must fail (exit 1), not silently no-op");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("this-file-does-not-exist-anywhere.tmp"),
        "error must name the exact missing path, stderr: {}",
        stderr
    );
    assert_eq!(
        client.evaluate("document.getElementById('result').textContent").unwrap(),
        "none",
        "no CDP call should have been attempted, so the input must be untouched"
    );

    drop(client);
}

/// @covers: set-files
#[test]
fn test_set_files_returns_exit_2_when_files_missing() {
    let output = cli().args(["set-files", "--port", "9222", "--selector", "#x"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

/// @covers: set-files
#[test]
fn test_set_files_returns_exit_2_when_selector_missing() {
    let output = cli().args(["set-files", "--port", "9222", "--files", "a.png"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
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

// ---------------------------------------------------------------------------
// reap
// ---------------------------------------------------------------------------

fn unique_session_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("chromiumctl_cli_test_sessions_{}_{}", std::process::id(), name))
}

/// Spawn `chromiumctl-cli launch` via a short-lived wrapper process so the
/// recorded `caller_pid` (the wrapper, not this test process) is already
/// dead by the time the wrapper's own `.output()` call returns here —
/// simulating exactly the RFC-0003 scenario (the process that called
/// `launch` died before ever calling `stop`).
fn launch_with_dead_caller(port: u16, dir: &std::path::Path, url: &str) {
    let cli_path = env!("CARGO_BIN_EXE_browse");
    let output = if cfg!(windows) {
        // Separate argv entries (not one joined string) so cmd.exe never
        // re-parses `<`/`>`/`|`/`&` out of our own arguments — cmd.exe only
        // sees a plain, unquoted, space-tokenized tail here.
        Command::new("cmd")
            .arg("/C")
            .arg(cli_path)
            .args(["launch", "--url", url, "--port", &port.to_string()])
            .env("CHROMIUMCTL_SESSION_DIR", dir)
            .output()
    } else {
        // `; true` prevents the shell from tail-call-exec'ing directly into
        // `launch` (which would make the shell *become* `launch`'s own PID
        // instead of staying a separate parent that then exits).
        Command::new("sh")
            .arg("-c")
            .arg(format!("'{}' launch --url '{}' --port {} ; true", cli_path, url, port))
            .env("CHROMIUMCTL_SESSION_DIR", dir)
            .output()
    }
    .expect("failed to run wrapped chromiumctl-cli launch");
    assert!(
        output.status.success(),
        "wrapped launch must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// @covers: reap
#[test]
fn test_reap_dry_run_reports_no_sessions_when_dir_is_empty() {
    let dir = unique_session_dir("empty");
    let output = cli()
        .args(["reap", "--dry-run"])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli reap");
    assert!(
        output.status.success(),
        "reap must exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("No orphaned or stale sessions found."),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

/// @covers: reap
#[test]
fn test_reap_returns_exit_2_for_unknown_option() {
    let output = cli().args(["reap", "--bogus"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

/// @covers: reap
#[test]
fn test_reap_returns_exit_2_for_invalid_max_age() {
    let output = cli().args(["reap", "--max-age", "banana"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

/// @covers: launch
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_launch_writes_a_session_record_reap_can_read() {
    let port = next_port();
    let dir = unique_session_dir("launch_writes_record");

    let output = cli()
        .args(["launch", "--url", "data:text/html,launch-record-test", "--port", &port.to_string()])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli launch");
    assert!(output.status.success(), "launch must exit 0, stderr: {}", String::from_utf8_lossy(&output.stderr));

    let record_path = dir.join(format!("{}.json", port));
    let body = std::fs::read_to_string(&record_path)
        .unwrap_or_else(|e| panic!("session record must exist at {}: {}", record_path.display(), e));
    let record: serde_json::Value = serde_json::from_str(&body).expect("session record must be valid JSON");
    assert_eq!(record["port"], port);
    assert!(record["caller_pid"].as_u64().unwrap() > 0, "caller_pid must be a plausible PID");
    assert!(record["launched_at"].as_u64().unwrap() > 0, "launched_at must be a real timestamp");

    let client = CdpClient::attach(port).expect("browser must be reachable");
    let _ = client.send("Browser.close", serde_json::json!({}));
}

/// @covers: stop
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_stop_deletes_the_session_record() {
    let port = next_port();
    let dir = unique_session_dir("stop_deletes_record");

    let launch_output = cli()
        .args(["launch", "--url", "data:text/html,stop-record-test", "--port", &port.to_string()])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli launch");
    assert!(launch_output.status.success(), "setup: launch must succeed");

    let record_path = dir.join(format!("{}.json", port));
    assert!(record_path.exists(), "setup: session record must exist after launch");

    let stop_output = cli()
        .args(["stop", "--port", &port.to_string()])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli stop");
    assert!(
        stop_output.status.success(),
        "stop must exit 0, stderr: {}",
        String::from_utf8_lossy(&stop_output.stderr)
    );

    assert!(!record_path.exists(), "session record must be deleted after stop");
}

/// @covers: reap
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_reap_leaves_alive_callers_session_untouched() {
    let port = next_port();
    let dir = unique_session_dir("reap_leaves_alive_alone");

    // This test process is `launch`'s parent for the whole test, so its
    // session must never look orphaned to `reap`.
    let launch_output = cli()
        .args(["launch", "--url", "data:text/html,alive-caller-test", "--port", &port.to_string()])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli launch");
    assert!(launch_output.status.success(), "setup: launch must succeed");

    let reap_output = cli()
        .args(["reap"])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli reap");
    assert!(
        reap_output.status.success(),
        "reap must exit 0, stderr: {}",
        String::from_utf8_lossy(&reap_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&reap_output.stdout).contains("No orphaned or stale sessions found."),
        "reap must not touch a session whose caller (this test process) is still alive, stdout: {}",
        String::from_utf8_lossy(&reap_output.stdout)
    );

    let record_path = dir.join(format!("{}.json", port));
    assert!(record_path.exists(), "session record for a still-alive caller must not be deleted");

    let client = CdpClient::attach(port).expect("browser of a still-alive caller must remain reachable");
    let _ = client.send("Browser.close", serde_json::json!({}));
}

/// @covers: reap
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_reap_with_max_age_leaves_alive_caller_untouched_across_repeated_calls() {
    let port = next_port();
    let dir = unique_session_dir("reap_max_age_alive_repeated");

    // This test process is `launch`'s parent for the whole test — a
    // generous --max-age must not cause an otherwise-healthy, freshly
    // launched session to be reaped just because an age limit was given.
    let launch_output = cli()
        .args(["launch", "--url", "data:text/html,max-age-alive-test", "--port", &port.to_string()])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli launch");
    assert!(launch_output.status.success(), "setup: launch must succeed");

    let record_path = dir.join(format!("{}.json", port));

    // Call twice: a live, in-max-age session must be a no-op both times,
    // not just on a first pass.
    for attempt in 1..=2 {
        let reap_output = cli()
            .args(["reap", "--max-age", "1h"])
            .env("CHROMIUMCTL_SESSION_DIR", &dir)
            .output()
            .expect("failed to run chromiumctl-cli reap");
        assert!(
            reap_output.status.success(),
            "reap --max-age 1h must exit 0 on attempt {}, stderr: {}",
            attempt,
            String::from_utf8_lossy(&reap_output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&reap_output.stdout).contains("No orphaned or stale sessions found."),
            "attempt {}: a live caller's fresh session must not be reaped just because --max-age was given, stdout: {}",
            attempt,
            String::from_utf8_lossy(&reap_output.stdout)
        );
        assert!(record_path.exists(), "attempt {}: session record must survive", attempt);
    }

    let client = CdpClient::attach(port).expect("browser of a still-alive, in-max-age caller must remain reachable");
    let _ = client.send("Browser.close", serde_json::json!({}));
}

/// @covers: reap
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_reap_detects_pid_reuse_via_start_time_mismatch_and_reaps() {
    let port = next_port();
    let dir = unique_session_dir("reap_pid_reuse_simulated");

    // Real caller (this test process), so `caller_pid` alone would read as
    // alive — that's the point: this proves the fingerprint check, not bare
    // PID liveness, is what actually gates `reap` here. Real OS PID reuse
    // can't be forced deterministically from a test, so this simulates it
    // the honest way: corrupt the on-disk fingerprint that was captured at
    // launch time, as if this PID had since been handed to a different
    // process than the one `launch` originally recorded.
    let launch_output = cli()
        .args(["launch", "--url", "data:text/html,pid-reuse-test", "--port", &port.to_string()])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli launch");
    assert!(launch_output.status.success(), "setup: launch must succeed");

    let record_path = dir.join(format!("{}.json", port));
    let body = std::fs::read_to_string(&record_path).expect("setup: session record must exist");
    let mut record: serde_json::Value = serde_json::from_str(&body).expect("setup: record must be valid JSON");
    assert!(
        record["caller_start_time"].is_string(),
        "setup: launch must have captured a real fingerprint on this platform for this test to be meaningful, record: {}",
        record
    );
    record["caller_start_time"] = serde_json::json!("simulated-reused-pid-fingerprint-mismatch");
    std::fs::write(&record_path, serde_json::to_string_pretty(&record).unwrap())
        .expect("setup: must be able to rewrite the session record");

    let reap_output = cli()
        .args(["reap"])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli reap");
    assert!(
        reap_output.status.success(),
        "reap must exit 0, stderr: {}",
        String::from_utf8_lossy(&reap_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&reap_output.stdout);
    assert!(
        stdout.contains("Reaped"),
        "reap must treat a fingerprint mismatch as an orphan, even though caller_pid resolves to a genuinely live process, stdout: {}",
        stdout
    );

    assert!(!record_path.exists(), "reap must delete the record once a fingerprint mismatch is detected");
    assert!(
        CdpClient::attach(port).is_err(),
        "reap must have actually closed the browser despite caller_pid still being alive"
    );
}

/// @covers: reap
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_reap_dry_run_reports_orphaned_session_without_closing_it() {
    let port = next_port();
    let dir = unique_session_dir("reap_dry_run_orphan");
    launch_with_dead_caller(port, &dir, "data:text/html,dry-run-orphan-test");

    let reap_output = cli()
        .args(["reap", "--dry-run"])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli reap");
    assert!(
        reap_output.status.success(),
        "reap --dry-run must exit 0, stderr: {}",
        String::from_utf8_lossy(&reap_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&reap_output.stdout);
    assert!(stdout.contains(&port.to_string()), "dry-run must report the orphaned port, stdout: {}", stdout);
    assert!(stdout.contains("Would reap"), "dry-run must not claim to have acted, stdout: {}", stdout);

    // Dry-run must not have touched anything.
    let record_path = dir.join(format!("{}.json", port));
    assert!(record_path.exists(), "dry-run must not delete the session record");
    let client = CdpClient::attach(port).expect("dry-run must not close the orphaned browser");
    let _ = client.send("Browser.close", serde_json::json!({}));
}

/// @covers: reap
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_reap_closes_orphaned_session_and_deletes_its_record() {
    let port = next_port();
    let dir = unique_session_dir("reap_closes_orphan");
    launch_with_dead_caller(port, &dir, "data:text/html,reap-closes-orphan-test");

    let reap_output = cli()
        .args(["reap"])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli reap");
    assert!(
        reap_output.status.success(),
        "reap must exit 0, stderr: {}",
        String::from_utf8_lossy(&reap_output.stderr)
    );
    let stdout = String::from_utf8_lossy(&reap_output.stdout);
    assert!(stdout.contains("Reaped"), "reap must report what it reaped, stdout: {}", stdout);

    let record_path = dir.join(format!("{}.json", port));
    assert!(!record_path.exists(), "reap must delete the session record after closing the browser");
    assert!(CdpClient::attach(port).is_err(), "reap must have actually closed the orphaned browser");
}

/// @covers: launch
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_launch_reap_stale_closes_other_orphans_before_launching() {
    let orphan_port = next_port();
    let new_port = next_port();
    let dir = unique_session_dir("launch_reap_stale");

    launch_with_dead_caller(orphan_port, &dir, "data:text/html,reap-stale-orphan");

    let launch_output = cli()
        .args([
            "launch",
            "--url", "data:text/html,reap-stale-new-session",
            "--port", &new_port.to_string(),
            "--reap-stale",
        ])
        .env("CHROMIUMCTL_SESSION_DIR", &dir)
        .output()
        .expect("failed to run chromiumctl-cli launch --reap-stale");
    assert!(
        launch_output.status.success(),
        "launch --reap-stale must exit 0, stderr: {}",
        String::from_utf8_lossy(&launch_output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&launch_output.stdout).contains("Reaped"),
        "launch --reap-stale must report the orphan it cleaned up, stdout: {}",
        String::from_utf8_lossy(&launch_output.stdout)
    );

    assert!(
        CdpClient::attach(orphan_port).is_err(),
        "--reap-stale must have closed the pre-existing orphaned browser"
    );
    assert!(
        !dir.join(format!("{}.json", orphan_port)).exists(),
        "--reap-stale must have deleted the orphan's session record"
    );

    // The new session itself must be unaffected by the opportunistic reap.
    assert!(
        dir.join(format!("{}.json", new_port)).exists(),
        "the newly launched session's own record must exist"
    );
    let client = CdpClient::attach(new_port).expect("the newly launched browser must be reachable");
    let _ = client.send("Browser.close", serde_json::json!({}));
}

// ---------------------------------------------------------------------------
// mock
// ---------------------------------------------------------------------------

fn fetch_fixture_url() -> &'static str {
    r#"data:text/html,<div id="result">pending</div><script>
        window.runFetch = function(url) {
            fetch(url)
                .then(function(r) { return r.text(); })
                .then(function(t) { document.getElementById('result').textContent = 'ok:' + t; })
                .catch(function(e) { document.getElementById('result').textContent = 'error:' + e.message; });
        };
    </script>"#
}

/// Spawn `chromiumctl-cli mock` in the background (it blocks until
/// interrupted) and wait for its own "ready" stdout line before returning,
/// so callers never race a fetch against a not-yet-registered interception.
fn spawn_mock_and_wait_ready(
    port: u16,
    url_pattern: &str,
    status: &str,
    body: &str,
) -> std::process::Child {
    use std::io::{BufRead, BufReader};

    let mut child = cli()
        .args([
            "mock",
            "--port", &port.to_string(),
            "--url-pattern", url_pattern,
            "--status", status,
            "--body", body,
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn chromiumctl-cli mock");

    let stdout = child.stdout.take().expect("mock's stdout must be piped");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    let _ = reader.read_line(&mut line);
    assert!(
        line.contains("Mocking requests"),
        "mock's first stdout line must confirm it's ready to intercept, got: {:?}",
        line
    );

    child
}

/// @covers: mock
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_mock_fulfills_a_matching_request_with_the_fake_response() {
    let port = next_port();
    let client = CdpClientBuilder::new(fetch_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    let mut mock = spawn_mock_and_wait_ready(
        port,
        "*mock-target.invalid*",
        "200",
        r#"{"faked":true}"#,
    );

    client
        .evaluate("window.runFetch('https://mock-target.invalid/api'); 'started'")
        .expect("triggering the fetch must not itself error");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut result = "pending".to_string();
    while std::time::Instant::now() < deadline {
        result = client.evaluate("document.getElementById('result').textContent").unwrap();
        if result != "pending" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    let _ = mock.kill();
    let _ = mock.wait();
    drop(client);

    assert_eq!(
        result,
        r#"ok:{"faked":true}"#,
        "a request matching --url-pattern must receive the configured fake body"
    );
}

/// Trigger `window.runFetch(url)` against `client`'s page (having first
/// reset `#result` to `pending`) and return `(elapsed, final_result)`.
/// Shared by the "no added latency" comparison below, so the baseline and
/// with-mock-active measurements are driven identically.
fn run_fetch_and_time_it(client: &CdpClient, url: &str) -> (std::time::Duration, String) {
    client.evaluate("document.getElementById('result').textContent = 'pending'").unwrap();
    let started = std::time::Instant::now();
    client.evaluate(&format!("window.runFetch('{}'); 'started'", url)).expect("triggering the fetch must not itself error");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut result = "pending".to_string();
    while std::time::Instant::now() < deadline {
        result = client.evaluate("document.getElementById('result').textContent").unwrap();
        if result != "pending" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    (started.elapsed(), result)
}

/// @covers: mock
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_mock_leaves_non_matching_requests_untouched() {
    let port = next_port();
    let client = CdpClientBuilder::new(fetch_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    // Baseline: how long the same failing (nonexistent-host) fetch takes
    // with no interception active at all.
    let (baseline_elapsed, baseline_result) = run_fetch_and_time_it(&client, "https://other-target.invalid/api");
    assert!(baseline_result.starts_with("error:"), "setup: baseline fetch must fail with a real network error, got: {}", baseline_result);

    // Registers interception for a *different* pattern than the one this
    // test actually fetches from.
    let mut mock = spawn_mock_and_wait_ready(
        port,
        "*mock-target.invalid*",
        "200",
        r#"{"faked":true}"#,
    );

    let (with_mock_elapsed, result) = run_fetch_and_time_it(&client, "https://other-target.invalid/api");

    let _ = mock.kill();
    let _ = mock.wait();
    drop(client);

    // A non-matching URL is never paused by Chromium's own Fetch pattern
    // filter, so it hits the real (nonexistent) network and fails with a
    // genuine DNS/connection error — proof it was never faked.
    assert!(
        result.starts_with("error:"),
        "a request NOT matching --url-pattern must reach the real network untouched, got: {}",
        result
    );
    assert_ne!(result, r#"ok:{"faked":true}"#, "must not have received the fake body meant for a different pattern");

    // "No added latency/stalling": a relative comparison against this same
    // test's own baseline, not an absolute threshold — robust to whatever
    // this machine's real DNS-failure latency happens to be, while still
    // catching a real regression (e.g. mock's loop swallowing the event
    // and only releasing it after some retry/backoff).
    assert!(
        with_mock_elapsed <= baseline_elapsed * 3 + std::time::Duration::from_secs(1),
        "a non-matching request took {:?} with mock active vs {:?} baseline — mock must not add meaningful latency to unrelated traffic",
        with_mock_elapsed,
        baseline_elapsed
    );
}

/// @covers: mock
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_normal_commands_are_unaffected_when_mock_is_never_invoked() {
    let port = next_port();
    let client = CdpClientBuilder::new(fetch_fixture_url())
        .port(port)
        .launch()
        .expect("setup: launch must succeed");

    // `mock` is never spawned anywhere in this test — the whole point is a
    // session that has never touched the Fetch domain at all, proving
    // chromiumctl-cli's behavior is byte-for-byte the same as it was
    // before `mock` existed, not merely "safe once mock has run and been
    // killed" (which the other `mock` tests above already exercise).
    let eval_output = cli()
        .args(["eval", "--port", &port.to_string(), "--script", "1 + 1"])
        .output()
        .expect("failed to run chromiumctl-cli eval");
    assert!(eval_output.status.success(), "eval must behave normally, stderr: {}", String::from_utf8_lossy(&eval_output.stderr));
    assert_eq!(String::from_utf8_lossy(&eval_output.stdout).trim(), "2");

    // A real network request must reach its real (nonexistent) destination
    // exactly as it always has — no interception machinery is listening.
    let (_, result) = run_fetch_and_time_it(&client, "https://never-mocked-target.invalid/api");
    assert!(
        result.starts_with("error:"),
        "with mock never invoked, a fetch must behave exactly as it always has (real network error), got: {}",
        result
    );

    drop(client);
}

/// @covers: mock
#[test]
fn test_mock_returns_exit_2_when_url_pattern_missing() {
    let output = cli().args(["mock", "--port", "9222"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}
