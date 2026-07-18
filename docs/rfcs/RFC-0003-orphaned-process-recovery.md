# RFC-0003: Orphaned Process Recovery

**Status:** Implemented (shipped in 0.3.0)
**Date:** 2026-07-16  
**Author:** Amu Hlongwane  

## Problem

`launch` intentionally detaches its spawned Chrome process so the browser can outlive the CLI invocation — `std::mem::forget()` is called on the `CdpClient` in `bin/chromiumctl-cli/main/src/commands/launch.rs` for exactly this reason. `stop` closes that browser only via a CDP `Browser.close` sent to `--port` (`bin/chromiumctl-cli/main/src/commands/stop.rs`); it never tracks or kills anything by OS PID, because on Windows the process `Command::spawn()` observes is a launcher stub that re-execs and exits immediately, making the real browser PID unobservable to the CLI in the first place.

This works correctly as long as every `launch` is eventually paired with a `stop`. There is no pidfile, state directory, registry, watchdog, or `--parent-pid` monitor anywhere in the codebase — confirmed by a full-repo grep. **If the process that called `launch` dies (crash, kill, CI job cancellation, timeout) before it calls `stop`, the headless Chrome instance leaks permanently**, with no fallback recovery path of any kind.

This was discovered by RAM exhaustion during an extended CDP-automation session: 79 distinct orphaned headless Chrome trees were found running simultaneously (each with a unique `--remote-debugging-port` and `--user-data-dir`), consuming hundreds of MB each. All were traced to `launch` calls whose caller had died mid-session (crashes, stale-binary reinstalls, timeouts) without reaching a corresponding `stop`.

## Proposed Solution

Add a lightweight, opt-in process registry that lets a separate `chromiumctl reap` command (or a background watchdog) clean up sessions whose caller is gone, without changing `stop`'s existing CDP-based happy path.

## Design

### State tracking
On `launch`, before `mem::forget`-ing the client, write a small session record to `%TEMP%/chromiumctl/sessions/<port>.json` (or `$XDG_RUNTIME_DIR`/`/tmp` equivalent on Unix):
```json
{
  "port": 63656,
  "user_data_dir": "C:\\Users\\...\\HeadlessChrome51080107563046",
  "launched_at": "2026-07-16T09:12:03Z",
  "caller_pid": 51080
}
```
`caller_pid` is the PID of the process that invoked `launch` (available via `std::process::id()` at spawn time), not the browser's own PID — this sidesteps the Windows launcher-stub problem entirely, since we only need to know whether the *caller* is still alive, not track the browser process itself.

`stop` deletes the record on successful `Browser.close`, same as today's happy path — no behavior change for callers who clean up correctly.

### `reap` command
```bash
chromiumctl reap [--dry-run] [--max-age 1h]
```
For each session record:
1. Check if `caller_pid` is still a live process. If dead → the session is orphaned.
2. If still alive but `launched_at` exceeds `--max-age`, treat as stale too (covers hung callers, not just dead ones).
3. For orphaned/stale sessions, attempt CDP `Browser.close` against `port` (same mechanism `stop` already uses); on failure (port unreachable — browser already gone or hung), fall back to killing by port-derived process lookup or leave to the OS.
4. Delete the stale session record regardless of outcome.
5. `--dry-run` lists what would be reaped without acting.

### Trigger points
- Manual: users/CI run `chromiumctl reap` as a periodic cleanup step.
- Optional: `launch` can opportunistically reap stale records for *other* dead sessions before creating its own (cheap, since it already touches the session directory) — bounds worst-case leak growth without requiring a standing daemon.

## Implementation

1. Add `session` module: read/write JSON records to the session directory, keyed by port.
2. Wire `launch` to write a record instead of (in addition to) `mem::forget`-ing silently.
3. Wire `stop` to delete the record on success.
4. Add `reap` subcommand: liveness-check `caller_pid` per-record (via existing OS process query, no new dependency needed on Windows/Unix), CDP-close or skip, delete record.
5. Add `--reap-stale` flag to `launch` for the opportunistic-reap trigger point.
6. Integration test: spawn a fake "caller" process that launches a browser then is killed without calling `stop`; assert `reap` finds and closes it.
7. Document the leak scenario and `reap` in README, next to the existing "chrome never became reachable" limitation (README.md:161).

## Benefits

- ✓ Fixes a confirmed, reproducible leak (79 orphaned instances observed in one session)
- ✓ No change to the existing `launch`/`stop` happy-path behavior or CDP-only design philosophy
- ✓ `caller_pid` liveness check avoids the Windows launcher-stub PID problem that made `stop` CDP-only in the first place
- ✓ `--dry-run` makes it safe to run in unfamiliar environments (won't blindly kill browsers it doesn't recognize)

## Risks

- Session directory could itself leak stale records if `reap` is never run.
  - Mitigation: opportunistic reap-on-launch bounds this; `reap` is cheap enough to also recommend as a cron/CI-teardown step.
- False-positive reap of a session whose caller PID was reused by an unrelated process after death.
  - Mitigation: also compare `launched_at` against process start time where the OS exposes it (Windows: `CreationDate` via WMI/CIM; Unix: `/proc/<pid>/stat` start time) before trusting a live PID match.
- Race between a slow `launch` writing its record and a concurrent `reap` running mid-write.
  - Mitigation: write via temp-file-then-rename, matching the atomicity pattern already implied by the CLI's existing config/output handling.

## Alternatives

1. Standing background watchdog daemon — rejected as a bigger operational footprint than a CLI tool should require; `reap` as an explicit/opportunistic command keeps chromiumctl dependency-free of any persistent service.
2. OS-level job objects / process groups (Windows Job Objects, Linux cgroups) to auto-kill children when the parent dies — would fully solve this without a registry, but is a much larger platform-specific implementation and doesn't help already-detached sessions from `mem::forget`; worth a future RFC but out of scope here since it's a bigger structural change than the leak warrants right now.
3. Status quo (document the limitation, rely on callers to always clean up) — insufficient given this is a real, already-observed resource exhaustion issue, not just a theoretical gap.

## Questions

- Should `reap`'s default `--max-age` be time-based only, or should a caller-liveness-only check (no age cutoff) be the default, with age as an opt-in stricter mode?
- Should `chromiumctl-cli launch` print the session-record path so external tooling can inspect/clean it up without needing `reap`?
- Is Job Objects/cgroups (Alternative 2) worth pursuing later as the "real" fix, with this RFC's registry-based `reap` treated as an interim mitigation?
