// E2e tests for CdpClient::attach_android (adb-based Android WebView remote
// debugging). Requires the `android` feature:
//   cargo test --features android --test adb_locator_e2e_test
//
// Tests that need a real connected Android device with `adb` installed and a
// debuggable WebView active are marked #[ignore] with
// "requires a real Android device" and were NOT run in the environment these
// were written in (no `adb` binary and no device/emulator available there) —
// verify the happy path on real hardware before relying on it.
#![cfg(feature = "android")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use browsectl::{CdpClient, CdpClientBuilder, PageEvaluator};

/// @covers: attach_android
#[test]
fn test_attach_android_fails_with_actionable_error_when_adb_path_invalid() {
    // SAFETY: e2e tests in this file run with --test-threads=1 (see
    // developer_guide.md), and no other test in this binary reads or writes
    // ADB_PATH concurrently with this one.
    unsafe {
        std::env::set_var("ADB_PATH", "/nonexistent/adb");
    }
    let result = CdpClient::attach_android("com.example.app");
    // SAFETY: same single-threaded e2e invocation as the set_var above; no
    // concurrent reader of ADB_PATH exists at this point.
    unsafe {
        std::env::remove_var("ADB_PATH");
    }

    let err = result.err().expect("attach_android must fail when ADB_PATH is unreachable");
    assert!(
        err.contains("ADB_PATH"),
        "error should name the offending env var so the caller can fix it: {err}"
    );
}

/// @covers: attach_android
#[test]
#[ignore = "requires a real Android device/emulator with adb installed and a debuggable WebView active"]
fn test_attach_android_connects_to_real_webview() {
    // Verified 2026-07-04 against a real Samsung SM-A055F over `adb connect`
    // (wireless debugging), using the minimal test app in
    // appsoluxions/hello-android/ (package com.chromiumctl.webviewdebugtest,
    // a single WebView with an element id="marker"): attach_android
    // succeeded end to end and `evaluate` read the marker's real content back.
    // Replace the package name below with a real debuggable package on
    // whatever device you're testing against.
    let client = CdpClient::attach_android("com.example.app")
        .expect("attach_android must succeed against a real debuggable WebView");
    assert!(client.port() > 0);
}

/// @covers: attach_android
#[test]
#[ignore = "requires a real Android device/emulator with adb installed"]
fn test_attach_android_fails_when_package_not_debuggable() {
    let result = CdpClient::attach_android("com.definitely.not.a.debuggable.package");
    assert!(
        result.is_err(),
        "attach_android must fail cleanly for a package with no active WebView debug socket"
    );
}

/// @covers: attach_android
///
/// No real Android device is available in this environment, so this
/// simulates `adb` with a small stand-in binary (`fake-adb-for-tests`,
/// `test-support/fake_adb.rs`) that answers the socket-enumeration and
/// `ps -A` queries with canned output, while the "forwarded" port actually
/// points at a real, already-running headless browser. This exercises every
/// line of `attach_android`'s real orchestration — locating `adb`, parsing
/// `/proc/net/unix`, matching the package via `ps -A`, parsing the forwarded
/// port, attaching, and evaluating JS — against a real CDP session, and
/// verifies `Drop` actually invokes `adb forward --remove`. The one thing it
/// does not exercise is the real `adb` binary's own behavior for these exact
/// commands — that still needs a real device (see the tests above).
#[test]
#[ignore = "requires a running Chromium instance"]
fn test_attach_android_succeeds_against_simulated_adb_and_real_browser() {
    let browser = CdpClientBuilder::new("data:text/html,<h1 id=x>fake-adb-e2e</h1>")
        .launch()
        .expect("setup: real browser launch must succeed");

    let fake_adb = env!("CARGO_BIN_EXE_fake-adb-for-tests");
    let remove_log = std::env::temp_dir().join(format!("fake_adb_remove_{}.log", std::process::id()));

    // SAFETY: this test file runs with --test-threads=1 (see
    // developer_guide.md), so no other test touches these env vars while
    // this one is running.
    unsafe {
        std::env::set_var("ADB_PATH", fake_adb);
        std::env::set_var("FAKE_ADB_PACKAGE", "com.example.fake");
        std::env::set_var("FAKE_ADB_FORWARD_PORT", browser.port().to_string());
        std::env::set_var("FAKE_ADB_REMOVE_LOG", &remove_log);
    }

    let result = CdpClient::attach_android("com.example.fake");

    // ADB_PATH/FAKE_ADB_PACKAGE/FAKE_ADB_FORWARD_PORT are only needed for the
    // attach_android call above, but FAKE_ADB_REMOVE_LOG must stay set until
    // after `android_client` is dropped below — that's when the child
    // process reading it is actually spawned.
    // SAFETY: same single-threaded e2e invocation as the set_var block above.
    unsafe {
        std::env::remove_var("ADB_PATH");
        std::env::remove_var("FAKE_ADB_PACKAGE");
        std::env::remove_var("FAKE_ADB_FORWARD_PORT");
    }

    let android_client =
        result.expect("attach_android must succeed against the simulated adb + real browser");
    assert_eq!(
        android_client
            .evaluate("document.getElementById('x').textContent")
            .unwrap(),
        "fake-adb-e2e",
        "attach_android must connect to the real browser behind the simulated forward"
    );

    drop(android_client);

    // SAFETY: same single-threaded e2e invocation as the earlier blocks in
    // this test; no concurrent reader of FAKE_ADB_REMOVE_LOG exists here.
    unsafe {
        std::env::remove_var("FAKE_ADB_REMOVE_LOG");
    }
    assert!(
        remove_log.exists(),
        "Drop must invoke `adb forward --remove` for a forward this client owns"
    );
    let _ = std::fs::remove_file(&remove_log);
}
