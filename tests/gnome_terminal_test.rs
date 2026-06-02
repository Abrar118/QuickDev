use quickdev::gnome_terminal::{
    build_load_config, build_wrapper_script, escape_gkeyfile_value, LoadConfigTab,
};

#[test]
fn wrapper_runs_command_then_keeps_shell_open() {
    let script = build_wrapper_script("/home/me/api", Some("npm run dev"));
    assert!(script.starts_with("#!/bin/sh\n"));
    assert!(script.contains("cd '/home/me/api' || exit 1\n"));
    assert!(script.contains("npm run dev\n"));
    assert!(script.trim_end().ends_with("exec \"${SHELL:-/bin/sh}\" -l"));
}

#[test]
fn wrapper_without_command_just_opens_shell_in_dir() {
    let script = build_wrapper_script("/srv", None);
    assert!(script.contains("cd '/srv' || exit 1\n"));
    assert!(!script.contains("npm"));
    assert!(script.trim_end().ends_with("exec \"${SHELL:-/bin/sh}\" -l"));
}

#[test]
fn wrapper_escapes_single_quotes_in_cwd() {
    let script = build_wrapper_script("/home/o'brien", None);
    assert!(script.contains("cd '/home/o'\\''brien' || exit 1\n"));
}

#[test]
fn load_config_groups_all_tabs_in_one_window() {
    let tabs = [
        LoadConfigTab {
            title: "api",
            wrapper_path: "/tmp/qd/tab0.sh",
        },
        LoadConfigTab {
            title: "web",
            wrapper_path: "/tmp/qd/tab1.sh",
        },
    ];
    let conf = build_load_config(&tabs);
    assert!(conf.contains("[GNOME Terminal Configuration]"));
    assert!(conf.contains("Windows=Window0;"));
    assert!(conf.contains("Terminals=Terminal0;Terminal1;"));
    assert!(conf.contains("[Terminal0]"));
    assert!(conf.contains("Title=api"));
    assert!(conf.contains("Command=/bin/sh '/tmp/qd/tab0.sh'"));
    assert!(conf.contains("[Terminal1]"));
    assert!(conf.contains("Title=web"));
    assert!(conf.contains("Command=/bin/sh '/tmp/qd/tab1.sh'"));
}

#[test]
fn load_config_single_tab_has_trailing_semicolon() {
    let tabs = [LoadConfigTab {
        title: "only",
        wrapper_path: "/tmp/qd/tab0.sh",
    }];
    let conf = build_load_config(&tabs);
    assert!(conf.contains("Terminals=Terminal0;"));
}

#[test]
fn gkeyfile_escape_handles_backslash_and_newline() {
    assert_eq!(escape_gkeyfile_value("a\\b"), "a\\\\b");
    assert_eq!(escape_gkeyfile_value("a\nb"), "a\\nb");
}

#[test]
fn load_config_single_quotes_wrapper_path_with_spaces() {
    let tabs = [LoadConfigTab {
        title: "api",
        wrapper_path: "/tmp/my path/tab0.sh",
    }];
    let conf = build_load_config(&tabs);
    assert!(conf.contains("Command=/bin/sh '/tmp/my path/tab0.sh'"));
}

#[cfg(target_os = "linux")]
#[test]
fn write_session_creates_executable_wrappers_and_conf() {
    use quickdev::gnome_terminal::{write_session, GnomeTab};
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join(format!("quickdev-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let tabs = [
        GnomeTab {
            title: "api",
            cwd: "/tmp",
            command: Some("echo hi"),
        },
        GnomeTab {
            title: "web",
            cwd: "/",
            command: None,
        },
    ];
    let conf = write_session(&dir, &tabs).unwrap();

    assert!(conf.exists());
    let conf_body = std::fs::read_to_string(&conf).unwrap();
    assert!(conf_body.contains("Terminals=Terminal0;Terminal1;"));

    let tab0 = dir.join("tab0.sh");
    assert!(tab0.exists());
    let mode = std::fs::metadata(&tab0).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o755);
    let body0 = std::fs::read_to_string(&tab0).unwrap();
    assert!(body0.contains("cd '/tmp'"));
    assert!(body0.contains("echo hi"));

    std::fs::remove_dir_all(&dir).ok();
}
