//! Owner-only temporary directories for terminal session files.
//!
//! kitty and gnome-terminal tab launches write wrapper scripts plus a session
//! file into a temp directory that must outlive this process: both are GUI
//! programs that read the files asynchronously after being spawned, so deleting
//! on exit would race the terminal.
//!
//! A predictable path (the old `quickdev-<pid>` scheme) let another local user
//! on a shared temp filesystem pre-create the directory and plant symlinks where
//! QuickDev writes. The directory is therefore randomly named and mode `0700`,
//! and every file is created with `create_new` so an existing path — symlink or
//! not — is refused rather than followed. Since the files outlive the process,
//! stale directories are reaped on the next grouped launch — but only ones this
//! module created, identified by both a specific prefix and a marker file.

use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Session directories older than this are removed on the next grouped launch.
/// A wrapper is read once, at tab startup, so an hour is far beyond the window
/// in which the terminal still needs the files.
const STALE_AFTER: Duration = Duration::from_secs(60 * 60);

/// Deliberately narrower than `quickdev-`: the npm installer stages downloads in
/// `mkdtemp(tmpdir/quickdev-)`, and other tooling may reasonably use the bare
/// product name too. Reaping on the generic prefix would delete a long-running
/// installation's staging directory out from under it.
const PREFIX: &str = "quickdev-tabs-";

/// Written inside every session directory. The reaper removes only directories
/// carrying it, so a name collision alone is never enough to get something
/// deleted.
const MARKER: &str = ".quickdev-session";

/// Create a fresh randomly-named `0700` session directory under the temp dir,
/// after reaping stale ones. The directory is intentionally not deleted when
/// this process exits.
pub fn create_session_dir() -> Result<PathBuf, String> {
    reap_stale_session_dirs();
    let dir = tempfile::Builder::new()
        .prefix(PREFIX)
        .tempdir()
        .map_err(|e| format!("failed to create temp dir: {e}"))?;
    // `tempdir()` honours the umask, which typically leaves the directory
    // world-readable. Narrow it before `keep()`: the wrappers inside carry the
    // user's startup commands, which may contain tokens, and while the directory
    // is still a `TempDir` a failure here removes it instead of leaking it. The
    // unnarrowed window is harmless — the directory is empty and never group- or
    // world-*writable*, so nothing can be planted inside it before this runs.
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("failed to restrict temp dir permissions: {e}"))?;
    write_new(&dir.path().join(MARKER), "", 0o600)?;
    Ok(dir.keep())
}

/// Write `body` to a new file at `path` with mode `mode`.
///
/// Fails if `path` already exists: inside an owner-only random directory that
/// only happens if we generated a duplicate name, never through a planted
/// symlink, so refusing is always correct.
pub fn write_new(path: &Path, body: &str, mode: u32) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|e| format!("failed to create {}: {e}", path.display()))?;
    file.write_all(body.as_bytes())
        .map_err(|e| format!("failed to write {}: {e}", path.display()))
}

/// Best-effort removal of session directories left behind by earlier runs.
///
/// A directory is removed only when it matches [`PREFIX`], is a real directory
/// rather than a symlink, carries our [`MARKER`], and has not been touched for
/// [`STALE_AFTER`]. Anything failing one of those is left alone: deleting a temp
/// directory that is not ours is far worse than leaking one that is.
///
/// Errors are ignored throughout — another user's directory on a shared temp
/// filesystem is not ours to delete, and a failed cleanup must never block a
/// launch.
fn reap_stale_session_dirs() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        if is_reapable(&entry.path(), now) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Whether `path` is one of our session directories and old enough to remove.
/// `now` is a parameter so the age rule stays testable without backdating files.
pub fn is_reapable(path: &Path, now: SystemTime) -> bool {
    if !path
        .file_name()
        .map(|name| name.to_string_lossy().starts_with(PREFIX))
        .unwrap_or(false)
    {
        return false;
    }
    let Ok(meta) = path.symlink_metadata() else {
        return false;
    };
    if !meta.is_dir() {
        return false;
    }
    // `symlink_metadata`: a marker reached through a symlink proves nothing.
    let marked = path
        .join(MARKER)
        .symlink_metadata()
        .map(|m| m.is_file())
        .unwrap_or(false);
    if !marked {
        return false;
    }
    meta.modified()
        .ok()
        .and_then(|modified| now.duration_since(modified).ok())
        .is_some_and(|age| age > STALE_AFTER)
}
