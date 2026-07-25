#![cfg(any(target_os = "macos", target_os = "linux"))]

use quickdev::session_dir::{create_session_dir, write_new};
use std::os::unix::fs::PermissionsExt;

#[test]
fn session_dir_is_owner_only_and_unpredictable() {
    let a = create_session_dir().unwrap();
    let b = create_session_dir().unwrap();

    for dir in [&a, &b] {
        assert!(dir.is_dir());
        let mode = std::fs::metadata(dir).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "session dir must not be readable or writable by other users"
        );
    }
    // Two calls must not collide: a predictable name (the old `quickdev-<pid>`)
    // is what let another local user pre-create the directory.
    assert_ne!(a, b);

    std::fs::remove_dir_all(&a).ok();
    std::fs::remove_dir_all(&b).ok();
}

#[test]
fn write_new_applies_the_requested_mode() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("wrapper.sh");

    write_new(&path, "#!/bin/sh\n", 0o700).unwrap();

    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "#!/bin/sh\n");
}

#[test]
fn write_new_refuses_an_existing_symlink_instead_of_following_it() {
    let temp = tempfile::tempdir().unwrap();
    let victim = temp.path().join("victim");
    std::fs::write(&victim, "original").unwrap();

    let link = temp.path().join("wrapper.sh");
    std::os::unix::fs::symlink(&victim, &link).unwrap();

    assert!(write_new(&link, "planted", 0o700).is_err());
    assert_eq!(
        std::fs::read_to_string(&victim).unwrap(),
        "original",
        "writing through a planted symlink must not clobber the target"
    );
}

#[test]
fn reaper_only_targets_marked_session_directories() {
    use quickdev::session_dir::is_reapable;
    use std::time::{Duration, SystemTime};

    let long_after = SystemTime::now() + Duration::from_secs(48 * 60 * 60);

    let ours = create_session_dir().unwrap();
    // Fresh: nothing to reap yet, even though it is ours.
    assert!(!is_reapable(&ours, SystemTime::now()));
    // Old enough, ours, marked.
    assert!(is_reapable(&ours, long_after));

    // Fixtures live in a private sandbox, never at fixed paths in the shared
    // temp dir: `is_reapable` judges a path by name, marker and age regardless of
    // where it sits, and a test that creates and recursively deletes
    // `/tmp/quickdev-abc123` could collide with a real process — including the
    // very npm staging directory this case exists to protect.
    let sandbox = tempfile::tempdir().unwrap();

    // The npm installer stages downloads in mkdtemp(tmpdir/"quickdev-"). It must
    // never be reaped, however old — a slow install would lose its archive.
    let npm_style = sandbox.path().join("quickdev-abc123");
    std::fs::create_dir_all(&npm_style).unwrap();
    assert!(!is_reapable(&npm_style, long_after));

    // Right prefix, but not ours: no marker file.
    let unmarked = sandbox.path().join("quickdev-tabs-not-ours");
    std::fs::create_dir_all(&unmarked).unwrap();
    assert!(!is_reapable(&unmarked, long_after));

    // `ours` is a uniquely-named directory we created, so removing it is safe.
    std::fs::remove_dir_all(&ours).ok();
}
