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
