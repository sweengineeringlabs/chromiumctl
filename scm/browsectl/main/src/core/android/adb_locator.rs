use std::path::Path;
use std::process::Command;

/// Locates the Android `adb` binary and enumerates WebView remote-debugging
/// sockets over it.
///
/// Used by `CdpClient::attach_android` the same way `PlatformBrowserLocator`
/// is used by `CdpClient::launch`.
pub(crate) struct AdbLocator;

impl AdbLocator {
    /// Locate the `adb` binary: `ADB_PATH` env var, well-known Android SDK
    /// `platform-tools` install locations, then `adb` on `PATH`.
    pub(crate) fn find() -> Result<String, String> {
        if let Ok(path) = std::env::var("ADB_PATH") {
            if Path::new(&path).exists() {
                return Ok(path);
            }
            return Err(format!(
                "ADB_PATH is set to '{}' but that path does not exist",
                path
            ));
        }

        let mut candidates: Vec<String> = Vec::new();
        if cfg!(windows) {
            if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
                candidates.push(format!(
                    "{}\\Android\\Sdk\\platform-tools\\adb.exe",
                    local_app_data
                ));
            }
        } else if cfg!(target_os = "macos") {
            if let Ok(home) = std::env::var("HOME") {
                candidates.push(format!("{}/Library/Android/sdk/platform-tools/adb", home));
            }
        } else {
            if let Ok(home) = std::env::var("HOME") {
                candidates.push(format!("{}/Android/Sdk/platform-tools/adb", home));
            }
            candidates.push("/usr/lib/android-sdk/platform-tools/adb".to_string());
        }

        for candidate in &candidates {
            if Path::new(candidate).exists() {
                return Ok(candidate.clone());
            }
        }

        let on_path = if cfg!(windows) {
            Command::new("where").arg("adb").output()
        } else {
            Command::new("which").arg("adb").output()
        };
        if let Ok(out) = on_path {
            if out.status.success() {
                return Ok("adb".to_string());
            }
        }

        Err("adb not found. Install Android SDK platform-tools or set ADB_PATH.".into())
    }

    /// Enumerate `webview_devtools_remote_<pid>` abstract sockets on the
    /// connected device and return the one owned by `package_name` (exact
    /// match, or `package_name:sub_process` for a named sub-process). Returns
    /// the first match if more than one WebView is debuggable at once.
    pub(crate) fn find_webview_socket(adb: &str, package_name: &str) -> Result<String, String> {
        let proc_net_unix = Command::new(adb)
            .args(["shell", "cat", "/proc/net/unix"])
            .output()
            .map_err(|e| format!("failed to run '{} shell cat /proc/net/unix': {}", adb, e))?;
        if !proc_net_unix.status.success() {
            return Err(format!(
                "'{} shell cat /proc/net/unix' failed: {}",
                adb,
                String::from_utf8_lossy(&proc_net_unix.stderr)
            ));
        }
        let sockets = parse_webview_sockets(&String::from_utf8_lossy(&proc_net_unix.stdout));
        if sockets.is_empty() {
            return Err(
                "no WebView debug sockets found on the device — is \
                 WebView.setWebContentsDebuggingEnabled(true) active for any app?"
                    .to_string(),
            );
        }

        let ps = Command::new(adb)
            .args(["shell", "ps", "-A"])
            .output()
            .map_err(|e| format!("failed to run '{} shell ps -A': {}", adb, e))?;
        if !ps.status.success() {
            return Err(format!(
                "'{} shell ps -A' failed: {}",
                adb,
                String::from_utf8_lossy(&ps.stderr)
            ));
        }
        let ps_output = String::from_utf8_lossy(&ps.stdout);

        for socket in &sockets {
            let Some(pid) = extract_pid(socket) else {
                continue;
            };
            if ps_process_name_matches(&ps_output, pid, package_name) {
                return Ok(socket.clone());
            }
        }

        Err(format!(
            "no active WebView debug socket found for package '{}' ({} other socket(s) found on the device)",
            package_name,
            sockets.len()
        ))
    }

    /// `adb forward tcp:0 localabstract:<socket>` — forward an OS-assigned
    /// local port to the remote abstract socket and return the assigned port.
    pub(crate) fn forward(adb: &str, socket: &str) -> Result<u16, String> {
        let out = Command::new(adb)
            .args(["forward", "tcp:0", &format!("localabstract:{}", socket)])
            .output()
            .map_err(|e| format!("failed to run '{} forward': {}", adb, e))?;
        if !out.status.success() {
            return Err(format!(
                "'{} forward tcp:0 localabstract:{}' failed: {}",
                adb,
                socket,
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        parse_forward_port(&String::from_utf8_lossy(&out.stdout))
    }

    /// Best-effort removal of a port forward created by [`forward`]. Errors
    /// are not fatal — a lingering local forward is harmless, and the
    /// device-side socket is unaffected either way.
    pub(crate) fn remove_forward(adb: &str, port: u16) {
        let _ = Command::new(adb)
            .args(["forward", "--remove", &format!("tcp:{}", port)])
            .output();
    }
}

/// Extract `webview_devtools_remote_<pid>` socket names from `/proc/net/unix`
/// output (the last whitespace-separated column, when present, is the path).
fn parse_webview_sockets(proc_net_unix: &str) -> Vec<String> {
    proc_net_unix
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .filter(|field| field.contains("webview_devtools_remote_"))
        .map(|field| field.trim_start_matches('@').to_string())
        .collect()
}

/// Extract the trailing `<pid>` from a `webview_devtools_remote_<pid>` socket
/// name, or `None` if the trailing segment isn't a valid numeric pid.
fn extract_pid(socket_name: &str) -> Option<&str> {
    let pid = socket_name.rsplit('_').next()?;
    if !pid.is_empty() && pid.chars().all(|c| c.is_ascii_digit()) {
        Some(pid)
    } else {
        None
    }
}

/// Check whether `pid`'s row in `adb shell ps -A` output names a process
/// belonging to `package_name` (exact match, or `package_name:sub_process`).
fn ps_process_name_matches(ps_output: &str, pid: &str, package_name: &str) -> bool {
    for line in ps_output.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 2 || fields[1] != pid {
            continue;
        }
        let Some(name) = fields.last() else {
            continue;
        };
        return *name == package_name || name.starts_with(&format!("{}:", package_name));
    }
    false
}

/// Parse the port number `adb forward tcp:0 ...` prints on success.
fn parse_forward_port(output: &str) -> Result<u16, String> {
    output
        .trim()
        .parse()
        .map_err(|_| format!("could not parse port from adb forward output: '{}'", output.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_webview_sockets_finds_matching_entries() {
        let proc_net_unix = "\
Num       RefCount Protocol Flags    Type St Inode Path
0000000000000000: 00000002 00000000 00010000 0001 01 12345 /dev/socket/zygote
0000000000000000: 00000003 00000000 00000000 0001 01 22222 @webview_devtools_remote_5678
0000000000000000: 00000002 00000000 00010000 0001 01 33333 @webview_devtools_remote_9012
";
        let sockets = parse_webview_sockets(proc_net_unix);
        assert_eq!(
            sockets,
            vec!["webview_devtools_remote_5678", "webview_devtools_remote_9012"]
        );
    }

    #[test]
    fn test_parse_webview_sockets_returns_empty_when_none_present() {
        let proc_net_unix = "\
Num       RefCount Protocol Flags    Type St Inode Path
0000000000000000: 00000002 00000000 00010000 0001 01 12345 /dev/socket/zygote
";
        assert!(parse_webview_sockets(proc_net_unix).is_empty());
    }

    #[test]
    fn test_extract_pid_reads_trailing_number() {
        assert_eq!(extract_pid("webview_devtools_remote_5678"), Some("5678"));
    }

    #[test]
    fn test_extract_pid_returns_none_when_trailing_segment_is_not_numeric() {
        assert_eq!(extract_pid("webview_devtools_remote_"), None);
        assert_eq!(extract_pid("not_a_pid_abc"), None);
    }

    #[test]
    fn test_ps_process_name_matches_exact_package_name() {
        let ps = "\
USER     PID   PPID  VSZ    RSS   WCHAN  ADDR S NAME
u0_a123  5678  456   123456 65432 0      0    S com.example.app
";
        assert!(ps_process_name_matches(ps, "5678", "com.example.app"));
    }

    #[test]
    fn test_ps_process_name_matches_named_sub_process() {
        let ps = "\
USER     PID   PPID  VSZ    RSS   WCHAN  ADDR S NAME
u0_a123  9012  456   123456 65432 0      0    S com.example.app:webview_service
";
        assert!(ps_process_name_matches(ps, "9012", "com.example.app"));
    }

    #[test]
    fn test_ps_process_name_matches_returns_false_for_other_package() {
        let ps = "\
USER     PID   PPID  VSZ    RSS   WCHAN  ADDR S NAME
u0_a999  4242  456   123456 65432 0      0    S com.other.app
";
        assert!(!ps_process_name_matches(ps, "4242", "com.example.app"));
    }

    #[test]
    fn test_ps_process_name_matches_returns_false_for_unknown_pid() {
        let ps = "\
USER     PID   PPID  VSZ    RSS   WCHAN  ADDR S NAME
u0_a123  5678  456   123456 65432 0      0    S com.example.app
";
        assert!(!ps_process_name_matches(ps, "9999", "com.example.app"));
    }

    #[test]
    fn test_parse_forward_port_reads_valid_port() {
        assert_eq!(parse_forward_port("41235\n"), Ok(41235));
    }

    #[test]
    fn test_parse_forward_port_fails_on_garbage_output() {
        assert!(parse_forward_port("error: no devices/emulators found\n").is_err());
    }
}
