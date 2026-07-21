use std::process::Command;

/// Best-effort liveness/identity checks for OS processes, used by `reap` to
/// tell whether a `launch` caller is still around.
///
/// Shells out to `tasklist`/PowerShell (Windows) or `ps` (Unix) rather than
/// adding a process-inspection crate, matching the existing convention in
/// `core/browser/platform_browser_locator.rs` and `core/android/adb_locator.rs`.
/// PowerShell, not `wmic`, is the Windows tool here: `wmic` is deprecated
/// and already absent on current Windows builds (confirmed missing on
/// Windows 11 26200), where PowerShell is guaranteed present.
pub(crate) struct ProcessLocator;

impl ProcessLocator {
    /// Whether a process with `pid` is currently running.
    pub(crate) fn is_alive(pid: u32) -> bool {
        if cfg!(windows) {
            Command::new("tasklist")
                .args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"])
                .output()
                .map(|out| tasklist_csv_has_pid(&String::from_utf8_lossy(&out.stdout), pid))
                .unwrap_or(false)
        } else {
            Command::new("ps")
                .args(["-p", &pid.to_string()])
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false)
        }
    }

    /// The parent process ID of `pid`, or `None` if it can't be determined
    /// (tool missing, `pid` already gone).
    ///
    /// Used to find the *real* caller of `launch`: `launch` itself always
    /// exits right after spawning the browser (that's the whole point of
    /// detaching it), so `std::process::id()` inside `launch`'s own
    /// execution is never a usable liveness signal — it's already a dead
    /// PID by the time anything could check it. The process that actually
    /// stays alive and is expected to eventually call `stop` is whatever
    /// spawned `launch` in the first place, i.e. its parent.
    pub(crate) fn parent_pid(pid: u32) -> Option<u32> {
        if cfg!(windows) {
            let out = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("(Get-CimInstance Win32_Process -Filter 'ProcessId={}').ParentProcessId", pid),
                ])
                .output()
                .ok()?;
            parse_pid_output(&String::from_utf8_lossy(&out.stdout))
        } else {
            let out = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "ppid="])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            String::from_utf8_lossy(&out.stdout).trim().parse().ok()
        }
    }

    /// A best-effort fingerprint of *when* `pid` started, used to detect PID
    /// reuse: a dead caller's PID can be handed to an unrelated process by
    /// the OS before `reap` runs, and that unrelated process being "alive"
    /// must not be mistaken for the original caller still being alive.
    ///
    /// Returns `None` when the fingerprint can't be obtained (tool missing,
    /// process gone, unexpected output) — callers should fall back to the
    /// bare [`Self::is_alive`] check rather than treat that as "not alive".
    pub(crate) fn start_time_fingerprint(pid: u32) -> Option<String> {
        if cfg!(windows) {
            let out = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("(Get-Process -Id {}).StartTime.Ticks", pid),
                ])
                .output()
                .ok()?;
            parse_ticks_output(&String::from_utf8_lossy(&out.stdout))
        } else {
            let out = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "lstart="])
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let start = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if start.is_empty() {
                None
            } else {
                Some(start)
            }
        }
    }
}

/// Check whether a `tasklist /FO CSV /NH` line names `pid` in its PID column
/// (the second quoted field). Every field is quoted, so splitting on the
/// literal `","` boundary is safe even when a field (e.g. mem usage) has an
/// embedded comma of its own.
fn tasklist_csv_has_pid(output: &str, pid: u32) -> bool {
    let target = pid.to_string();
    output.lines().any(|line| {
        line.trim_matches('"')
            .split("\",\"")
            .nth(1)
            .map(|field| field == target)
            .unwrap_or(false)
    })
}

/// Parse a bare PID from PowerShell `... .ParentProcessId` output: the
/// number alone, or empty/whitespace when PowerShell found no such process
/// (its error goes to stderr, not stdout).
fn parse_pid_output(output: &str) -> Option<u32> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

/// Parse a bare `DateTime.Ticks` value from PowerShell `... .StartTime.Ticks`
/// output: the number alone, or empty/whitespace when PowerShell found no
/// such process.
fn parse_ticks_output(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_tasklist_csv_has_pid_finds_matching_row() {
        let output = "\"chrome.exe\",\"12345\",\"Console\",\"1\",\"50,000 K\"\r\n";
        assert!(tasklist_csv_has_pid(output, 12345));
    }

    #[test]
    fn test_tasklist_csv_has_pid_returns_false_for_other_pid() {
        let output = "\"chrome.exe\",\"12345\",\"Console\",\"1\",\"50,000 K\"\r\n";
        assert!(!tasklist_csv_has_pid(output, 99999));
    }

    #[test]
    fn test_tasklist_csv_has_pid_returns_false_when_no_tasks_found() {
        let output = "INFO: No tasks are running which match the specified criteria.\r\n";
        assert!(!tasklist_csv_has_pid(output, 12345));
    }

    #[test]
    fn test_parse_ticks_output_extracts_value() {
        assert_eq!(
            parse_ticks_output("\r\n639198444010163984\r\n"),
            Some("639198444010163984".to_string())
        );
    }

    #[test]
    fn test_parse_ticks_output_returns_none_when_blank() {
        assert_eq!(parse_ticks_output("\r\n"), None);
        assert_eq!(parse_ticks_output(""), None);
    }

    #[test]
    fn test_parse_pid_output_extracts_value() {
        assert_eq!(parse_pid_output("\r\n4242\r\n"), Some(4242));
    }

    #[test]
    fn test_parse_pid_output_returns_none_when_blank() {
        assert_eq!(parse_pid_output("\r\n"), None);
        assert_eq!(parse_pid_output(""), None);
    }

    #[test]
    fn test_parse_pid_output_returns_none_for_non_numeric_garbage() {
        assert_eq!(parse_pid_output("not a pid"), None);
    }

    #[test]
    fn test_parent_pid_of_current_process_is_a_live_distinct_process() {
        let pid = std::process::id();
        let parent = ProcessLocator::parent_pid(pid).expect("current process must have a parent");
        assert_ne!(parent, pid, "a process cannot be its own parent");
        assert!(ProcessLocator::is_alive(parent), "the test runner that spawned us must still be alive");
    }

    #[test]
    fn test_parent_pid_returns_none_for_implausible_pid() {
        assert_eq!(ProcessLocator::parent_pid(u32::MAX - 1), None);
    }

    #[test]
    fn test_is_alive_returns_true_for_current_process() {
        assert!(ProcessLocator::is_alive(std::process::id()));
    }

    #[test]
    fn test_is_alive_returns_false_for_implausible_pid() {
        // Far beyond any realistic PID range on Windows or Unix (PID 0 is
        // *not* safe to use here — it's the Windows "System Idle Process",
        // which `tasklist` reports as very much alive).
        assert!(!ProcessLocator::is_alive(u32::MAX - 1));
    }
}
