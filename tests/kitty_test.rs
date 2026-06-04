use quickdev::kitty::{build_session, escape_session_value, SessionTab};

#[test]
fn single_tab_has_no_new_tab_line() {
    let tabs = [SessionTab {
        title: "only",
        wrapper_path: "/tmp/qd/tab0.sh",
    }];
    let session = build_session(&tabs);
    assert!(session.contains("launch --title 'only' /bin/sh '/tmp/qd/tab0.sh'"));
    assert!(!session.contains("new_tab"));
}

#[test]
fn multiple_tabs_have_one_new_tab_per_extra_tab() {
    let tabs = [
        SessionTab { title: "api", wrapper_path: "/tmp/qd/tab0.sh" },
        SessionTab { title: "web", wrapper_path: "/tmp/qd/tab1.sh" },
        SessionTab { title: "logs", wrapper_path: "/tmp/qd/tab2.sh" },
    ];
    let session = build_session(&tabs);
    // First tab is implicit: no leading new_tab.
    assert!(!session.starts_with("new_tab"));
    assert!(session.starts_with("launch --title 'api' /bin/sh '/tmp/qd/tab0.sh'"));
    // Exactly two new_tab lines for three tabs.
    assert_eq!(session.matches("new_tab").count(), 2);
    assert!(session.contains("new_tab web\nlaunch /bin/sh '/tmp/qd/tab1.sh'"));
    assert!(session.contains("new_tab logs\nlaunch /bin/sh '/tmp/qd/tab2.sh'"));
}

#[test]
fn first_tab_title_single_quotes_are_escaped() {
    let tabs = [SessionTab {
        title: "o'brien",
        wrapper_path: "/tmp/qd/tab0.sh",
    }];
    let session = build_session(&tabs);
    assert!(session.contains("launch --title 'o'\\''brien' /bin/sh '/tmp/qd/tab0.sh'"));
}

#[test]
fn escape_session_value_doubles_single_quotes() {
    assert_eq!(escape_session_value("a'b"), "a'\\''b");
    assert_eq!(escape_session_value("plain"), "plain");
}

#[cfg(target_os = "linux")]
#[test]
fn write_session_creates_executable_wrappers_and_session_file() {
    use quickdev::kitty::{write_session, KittyTab};
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("quickdev-kitty-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let tabs = [
        KittyTab { title: "api", cwd: "/tmp", command: Some("echo hi") },
        KittyTab { title: "web", cwd: "/", command: None },
    ];
    let session = write_session(&dir, &tabs).unwrap();

    assert!(session.exists());
    let body = std::fs::read_to_string(&session).unwrap();
    assert!(body.contains("launch --title 'api' /bin/sh '"));
    assert!(body.contains("new_tab web"));

    let tab0 = dir.join("tab0.sh");
    assert!(tab0.exists());
    let mode = std::fs::metadata(&tab0).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    let body0 = std::fs::read_to_string(&tab0).unwrap();
    assert!(body0.contains("cd '/tmp'"));
    assert!(body0.contains("echo hi"));

    std::fs::remove_dir_all(&dir).ok();
}
