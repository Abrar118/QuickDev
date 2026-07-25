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
//! stale directories are reaped on the next grouped launch.

use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Session directories older than this are removed on the next grouped launch.
/// A wrapper is read once, at tab startup, so an hour is far beyond the window
/// in which the terminal still needs the files.
const STALE_AFTER: Duration = Duration::from_secs(60 * 60);

const PREFIX: &str = "quickdev-";

/// Create a fresh randomly-named `0700` session directory under the temp dir,
/// after reaping stale ones. The directory is intentionally not deleted when
/// this process exits.
pub fn create_session_dir() -> Result<PathBuf, String> {
    reap_stale_session_dirs();
    let dir = tempfile::Builder::new()
        .prefix(PREFIX)
        .tempdir()
        .map(|dir| dir.keep())
        .map_err(|e| format!("failed to create temp dir: {e}"))?;
    // `tempdir()` honours the umask, which typically leaves the directory
    // world-readable. Narrow it: the wrappers inside carry the user's startup
    // commands, which may contain tokens. The unnarrowed window is harmless —
    // the directory is never group- or world-*writable*, so nothing can be
    // planted inside it before this runs.
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("failed to restrict temp dir permissions: {e}"))?;
    Ok(dir)
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
/// Errors are ignored throughout: another user's directory on a shared temp
/// filesystem is not ours to delete, and a failed cleanup must never block a
/// launch. Symlinks are skipped so a planted link cannot redirect the delete.
fn reap_stale_session_dirs() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(PREFIX) {
            continue;
        }
        let Ok(meta) = entry.path().symlink_metadata() else {
            continue;
        };
        if !meta.is_dir() {
            continue;
        }
        let stale = meta
            .modified()
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age > STALE_AFTER);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
