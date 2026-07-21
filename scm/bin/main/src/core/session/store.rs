use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::record::SessionRecord;

/// Reads/writes [`SessionRecord`]s to a session directory, one JSON file per
/// port. Overridable via `BROWSECTL_SESSION_DIR` (tests use this to avoid
/// touching a real machine's session state), defaulting to
/// `<tmp>/browsectl/sessions`.
pub(crate) struct SessionStore;

impl SessionStore {
    /// Write (or overwrite) the session record for `record.port`, via
    /// temp-file-then-rename so a concurrent `reap` scan never observes a
    /// partially-written file.
    pub(crate) fn write(record: &SessionRecord) -> Result<(), String> {
        write_in(&base_dir(), record)
    }

    /// Delete the session record for `port`, if one exists. A missing file
    /// is not an error — `stop`/`reap` may race harmlessly on cleanup.
    pub(crate) fn delete(port: u16) {
        delete_in(&base_dir(), port)
    }

    /// Read every session record currently on disk. Unreadable/corrupt
    /// entries are skipped rather than failing the whole scan — a torn
    /// write from a crashed `launch` shouldn't block `reap` from cleaning up
    /// everything else.
    pub(crate) fn list() -> Vec<SessionRecord> {
        list_in(&base_dir())
    }
}

/// The current time as Unix seconds. `0` (the epoch) on the practically
/// impossible case of a clock before 1970 — never mistaken for a real
/// `launched_at`, since every real session is younger than that.
pub(crate) fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn base_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("BROWSECTL_SESSION_DIR") {
        return PathBuf::from(dir);
    }
    std::env::temp_dir().join("browsectl").join("sessions")
}

fn path_for(dir: &Path, port: u16) -> PathBuf {
    dir.join(format!("{}.json", port))
}

fn write_in(dir: &Path, record: &SessionRecord) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|e| format!("failed to create session dir '{}': {}", dir.display(), e))?;

    let body = serde_json::to_string_pretty(record)
        .map_err(|e| format!("failed to serialize session record: {}", e))?;

    // Unique per-writer temp name so concurrent `launch`es never clobber
    // each other's in-flight temp file before the atomic rename.
    let tmp_path = dir.join(format!("{}.json.tmp-{}", record.port, std::process::id()));
    fs::write(&tmp_path, &body)
        .map_err(|e| format!("failed to write '{}': {}", tmp_path.display(), e))?;

    let final_path = path_for(dir, record.port);
    fs::rename(&tmp_path, &final_path).map_err(|e| {
        format!(
            "failed to rename '{}' to '{}': {}",
            tmp_path.display(),
            final_path.display(),
            e
        )
    })
}

fn delete_in(dir: &Path, port: u16) {
    let _ = fs::remove_file(path_for(dir, port));
}

fn list_in(dir: &Path) -> Vec<SessionRecord> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|body| serde_json::from_str::<SessionRecord>(&body).ok())
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn unique_test_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "browsectl_session_test_{}_{}_{}",
            std::process::id(),
            name,
            now_unix_secs()
        ))
    }

    fn sample_record(port: u16) -> SessionRecord {
        SessionRecord {
            port,
            launched_at: 1_752_700_000,
            caller_pid: 4242,
            caller_start_time: Some("20260716211523.123456+120".to_string()),
        }
    }

    #[test]
    fn test_write_in_then_list_in_round_trips_the_record() {
        let dir = unique_test_dir("round_trip");
        write_in(&dir, &sample_record(9401)).expect("write must succeed");

        let records = list_in(&dir);
        assert_eq!(records, vec![sample_record(9401)]);
    }

    #[test]
    fn test_write_in_leaves_no_temp_file_behind() {
        let dir = unique_test_dir("no_temp_leftover");
        write_in(&dir, &sample_record(9402)).expect("write must succeed");

        let names: Vec<String> = fs::read_dir(&dir)
            .expect("dir must exist")
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["9402.json".to_string()], "only the final file should remain");
    }

    #[test]
    fn test_write_in_overwrites_existing_record_for_same_port() {
        let dir = unique_test_dir("overwrite");
        write_in(&dir, &sample_record(9403)).expect("first write must succeed");

        let mut updated = sample_record(9403);
        updated.caller_pid = 9999;
        write_in(&dir, &updated).expect("second write must succeed");

        let records = list_in(&dir);
        assert_eq!(records, vec![updated]);
    }

    #[test]
    fn test_delete_in_removes_the_record() {
        let dir = unique_test_dir("delete");
        write_in(&dir, &sample_record(9404)).expect("write must succeed");
        delete_in(&dir, 9404);
        assert!(list_in(&dir).is_empty());
    }

    #[test]
    fn test_delete_in_is_not_an_error_when_no_record_exists() {
        let dir = unique_test_dir("delete_missing");
        fs::create_dir_all(&dir).expect("setup: dir must be creatable");
        delete_in(&dir, 9405); // must not panic
    }

    #[test]
    fn test_list_in_returns_empty_when_dir_does_not_exist() {
        let dir = unique_test_dir("does_not_exist");
        assert!(list_in(&dir).is_empty());
    }

    #[test]
    fn test_list_in_skips_corrupt_entries_but_returns_the_rest() {
        let dir = unique_test_dir("skip_corrupt");
        write_in(&dir, &sample_record(9406)).expect("write must succeed");
        fs::write(dir.join("not-json.json"), "{ this is not valid json").expect("setup write must succeed");

        let records = list_in(&dir);
        assert_eq!(records, vec![sample_record(9406)]);
    }

    #[test]
    fn test_now_unix_secs_returns_a_plausible_recent_timestamp() {
        // 2026-01-01T00:00:00Z, as a sanity floor — must not be near-zero or negative.
        assert!(now_unix_secs() > 1_767_225_600);
    }
}
