//! Test-only stand-in for the real `adb` binary.
//!
//! There is no Android device or `adb` install available in CI/dev for this
//! crate, so `adb_locator_e2e_test.rs` points `ADB_PATH` at this binary to
//! exercise `AdbLocator`'s and `attach_android`'s real orchestration logic —
//! process spawning, output parsing, control flow — while the "forwarded"
//! port actually points at a real, already-running headless browser. Only
//! the real `adb` binary's own wire behavior for these commands is not
//! exercised by this substitute.

use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();

    match refs.as_slice() {
        ["shell", "cat", "/proc/net/unix"] => {
            println!("Num       RefCount Protocol Flags    Type St Inode Path");
            println!(
                "0000000000000000: 00000003 00000000 00000000 0001 01 22222 @webview_devtools_remote_424242"
            );
        }
        ["shell", "ps", "-A"] => {
            let package = env::var("FAKE_ADB_PACKAGE").unwrap_or_else(|_| "com.example.fake".to_string());
            println!("USER     PID    PPID  VSZ    RSS   WCHAN  ADDR S NAME");
            println!("u0_a123  424242 456   123456 65432 0      0    S {}", package);
        }
        ["forward", "tcp:0", remote] if remote.starts_with("localabstract:") => match env::var("FAKE_ADB_FORWARD_PORT")
        {
            Ok(port) => println!("{}", port),
            Err(_) => {
                eprintln!("fake-adb: FAKE_ADB_FORWARD_PORT must be set");
                std::process::exit(1);
            }
        },
        ["forward", "--remove", _port_arg] => {
            if let Ok(log_path) = env::var("FAKE_ADB_REMOVE_LOG") {
                let _ = std::fs::write(log_path, "removed\n");
            }
        }
        other => {
            eprintln!("fake-adb: unrecognized invocation: {:?}", other);
            std::process::exit(1);
        }
    }
}
