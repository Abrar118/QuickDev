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
