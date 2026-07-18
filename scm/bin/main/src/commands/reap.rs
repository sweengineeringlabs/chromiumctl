use crate::os_process::ProcessLocator;
use crate::session::{now_unix_secs, SessionRecord, SessionStore};

use super::{expect_value, CliError};

pub fn execute(args: &[String]) -> Result<(), CliError> {
    let mut dry_run = false;
    let mut max_age_secs: Option<u64> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => dry_run = true,
            "--max-age" => {
                i += 1;
                max_age_secs = Some(parse_duration_secs(&expect_value(args, i, "--max-age")?)?);
            }
            other => return Err(CliError::InvalidArgs(format!("unknown option: {}", other))),
        }
        i += 1;
    }

    let reaped = reap_sessions(dry_run, max_age_secs);
    print_outcomes(&reaped, dry_run);
    Ok(())
}

/// Shared by `execute` (the `reap` subcommand) and `launch --reap-stale`
/// (opportunistic reap-on-launch, caller-liveness only).
pub(crate) fn print_outcomes(reaped: &[ReapOutcome], dry_run: bool) {
    if reaped.is_empty() {
        println!("No orphaned or stale sessions found.");
        return;
    }
    let verb = if dry_run { "Would reap" } else { "Reaped" };
    for outcome in reaped {
        println!("{} session on port {} ({})", verb, outcome.record.port, outcome.reason);
    }
    println!(
        "{} {} session(s).",
        if dry_run { "Found" } else { "Reaped" },
        reaped.len()
    );
}

pub(crate) struct ReapOutcome {
    pub(crate) record: SessionRecord,
    pub(crate) reason: String,
}

/// Scan every tracked session and reap (or, with `dry_run`, just report) the
/// ones whose caller is gone or that exceed `max_age_secs`. A session whose
/// caller is still alive and within `max_age_secs` (or no age limit was
/// given) is left untouched.
pub(crate) fn reap_sessions(dry_run: bool, max_age_secs: Option<u64>) -> Vec<ReapOutcome> {
    SessionStore::list()
        .into_iter()
        .filter_map(|record| {
            let reason = classify(&record, max_age_secs)?;
            if !dry_run {
                close_and_forget(&record);
                SessionStore::delete(record.port);
            }
            Some(ReapOutcome { record, reason })
        })
        .collect()
}

/// `Some(reason)` if `record` should be reaped, `None` if its caller is
/// still alive and it isn't past `max_age_secs`.
fn classify(record: &SessionRecord, max_age_secs: Option<u64>) -> Option<String> {
    if !caller_is_alive(record) {
        return Some("caller process is no longer running".to_string());
    }
    if let Some(max_age) = max_age_secs {
        let age = now_unix_secs().saturating_sub(record.launched_at);
        if age > max_age {
            return Some(format!("exceeded --max-age ({}s old)", age));
        }
    }
    None
}

/// Whether `record`'s caller is still the same process that called
/// `launch` — not merely a different process that has since reused its PID.
fn caller_is_alive(record: &SessionRecord) -> bool {
    if !ProcessLocator::is_alive(record.caller_pid) {
        return false;
    }
    match (
        &record.caller_start_time,
        ProcessLocator::start_time_fingerprint(record.caller_pid),
    ) {
        (Some(recorded), Some(current)) => *recorded == current,
        // No fingerprint available (either at launch time or now) — fall
        // back to the bare PID-liveness check rather than assume reuse.
        _ => true,
    }
}

/// Best-effort: ask the browser to close itself over CDP. A failure here
/// (already gone, port unreachable) is not an error — the session record is
/// removed either way, matching RFC-0003's step 4.
fn close_and_forget(record: &SessionRecord) {
    if let Ok(client) = cdp_client::CdpClient::attach(record.port) {
        let _ = client.send("Browser.close", serde_json::json!({}));
    }
}

/// Parse a plain integer (seconds) or a suffixed duration (`30s`, `5m`, `1h`).
fn parse_duration_secs(raw: &str) -> Result<u64, CliError> {
    let raw = raw.trim();
    let invalid = || {
        CliError::InvalidArgs(format!(
            "invalid --max-age value: '{}' (expected e.g. 30, 30s, 5m, 1h)",
            raw
        ))
    };
    let (num_part, multiplier) = match raw.chars().last() {
        Some('h') => (&raw[..raw.len() - 1], 3600),
        Some('m') => (&raw[..raw.len() - 1], 60),
        Some('s') => (&raw[..raw.len() - 1], 1),
        _ => (raw, 1),
    };
    num_part
        .parse::<u64>()
        .map_err(|_| invalid())
        .map(|n| n * multiplier)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_secs_accepts_plain_integer() {
        assert_eq!(parse_duration_secs("45").unwrap(), 45);
    }

    #[test]
    fn test_parse_duration_secs_accepts_seconds_suffix() {
        assert_eq!(parse_duration_secs("30s").unwrap(), 30);
    }

    #[test]
    fn test_parse_duration_secs_accepts_minutes_suffix() {
        assert_eq!(parse_duration_secs("5m").unwrap(), 300);
    }

    #[test]
    fn test_parse_duration_secs_accepts_hours_suffix() {
        assert_eq!(parse_duration_secs("1h").unwrap(), 3600);
    }

    #[test]
    fn test_parse_duration_secs_rejects_garbage() {
        assert!(parse_duration_secs("banana").is_err());
    }

    #[test]
    fn test_parse_duration_secs_rejects_suffix_with_no_number() {
        assert!(parse_duration_secs("h").is_err());
    }

    #[test]
    fn test_classify_reaps_when_caller_pid_is_implausible() {
        let record = SessionRecord {
            port: 1,
            launched_at: 0,
            // Far beyond any realistic PID range — never a real caller.
            // (PID 0 is not safe here: it's the Windows "System Idle
            // Process", which `tasklist` reports as alive.)
            caller_pid: u32::MAX - 1,
            caller_start_time: None,
        };
        assert_eq!(
            classify(&record, None),
            Some("caller process is no longer running".to_string())
        );
    }

    #[test]
    fn test_classify_leaves_alive_caller_alone_when_no_max_age() {
        let record = SessionRecord {
            port: 1,
            launched_at: now_unix_secs(),
            caller_pid: std::process::id(),
            caller_start_time: None,
        };
        assert_eq!(classify(&record, None), None);
    }

    #[test]
    fn test_classify_reaps_alive_caller_once_max_age_exceeded() {
        let record = SessionRecord {
            port: 1,
            launched_at: 0, // effectively infinitely old
            caller_pid: std::process::id(),
            caller_start_time: None,
        };
        let reason = classify(&record, Some(60)).expect("must be reaped once past max_age");
        assert!(reason.contains("--max-age"), "reason should explain why: {}", reason);
    }

    #[test]
    fn test_classify_leaves_alive_caller_alone_within_max_age() {
        let record = SessionRecord {
            port: 1,
            launched_at: now_unix_secs(),
            caller_pid: std::process::id(),
            caller_start_time: None,
        };
        assert_eq!(classify(&record, Some(3600)), None);
    }

    #[test]
    fn test_caller_is_alive_detects_pid_reuse_via_start_time_mismatch() {
        let record = SessionRecord {
            port: 1,
            launched_at: now_unix_secs(),
            caller_pid: std::process::id(),
            caller_start_time: Some("stale-fingerprint-that-cannot-match".to_string()),
        };
        // The current process is alive, but its *real* fingerprint will never
        // equal this deliberately-wrong recorded one — unless the platform
        // fingerprint lookup itself is unavailable (returns None), in which
        // case the bare PID check is the documented fallback.
        let alive = caller_is_alive(&record);
        let real_fingerprint = ProcessLocator::start_time_fingerprint(std::process::id());
        if real_fingerprint.is_some() {
            assert!(!alive, "a mismatched fingerprint must not be treated as the same caller");
        }
    }
}
