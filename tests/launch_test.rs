use quickdev::ghostty_applescript::{build_script, ResolvedTerminal};
#[cfg(not(target_os = "windows"))]
use quickdev::launch::pgrep_args_for_process;
use quickdev::launch::{
    editor_args, effective_app_args, emulator_watch_process, escape_applescript_string,
    escape_powershell_single_quotes, normalize_path, poll_until, resolve_app_args,
    resolve_terminal_path, PlaceholderContext,
};
use quickdev::tab_strategy::{select_tab_strategy, TabCapabilities, TabStrategy};
use quickdev::terminal_app::{
    build_auto_tab_script, build_terminal_command, parse_tabbing_preference, TabbingPreference,
};
use std::path::Path;

#[test]
fn watch_process_prefers_ghostty_only_on_macos_for_unspecified_emulator() {
    // None + ghostty installed: macOS attempts Ghostty first, so we watch it.
    assert_eq!(
        emulator_watch_process(None, true, false, "macos"),
        Some("ghostty")
    );
    // Linux never auto-launches Ghostty for an unspecified emulator
    // (try_ghostty is macOS-gated), so we watch the native terminal even when
    // the ghostty binary is present — and honor the ptyxis preference.
    assert_eq!(
        emulator_watch_process(None, true, false, "linux"),
        Some("gnome-terminal-server")
    );
    assert_eq!(
        emulator_watch_process(None, true, true, "linux"),
        Some("ptyxis")
    );
}

#[test]
fn watch_process_falls_back_to_native_when_no_ghostty() {
    assert_eq!(
        emulator_watch_process(None, false, false, "macos"),
        Some("Terminal")
    );
    assert_eq!(
        emulator_watch_process(None, false, false, "linux"),
        Some("gnome-terminal-server")
    );
    assert_eq!(
        emulator_watch_process(None, false, false, "windows"),
        Some("WindowsTerminal.exe")
    );
}

#[test]
fn watch_process_prefers_ptyxis_on_linux_when_available() {
    assert_eq!(
        emulator_watch_process(None, false, true, "linux"),
        Some("ptyxis")
    );
}

#[test]
fn watch_process_honors_explicit_emulator() {
    // Explicit ghostty is watched on every platform (not cfg-gated).
    assert_eq!(
        emulator_watch_process(Some("ghostty"), false, false, "linux"),
        Some("ghostty")
    );
    assert_eq!(
        emulator_watch_process(Some("terminal"), false, false, "macos"),
        Some("Terminal")
    );
    // Explicit terminal on Linux honors the ptyxis preference too.
    assert_eq!(
        emulator_watch_process(Some("terminal"), false, true, "linux"),
        Some("ptyxis")
    );
}

#[test]
fn watch_process_unknown_emulator_is_none() {
    assert_eq!(
        emulator_watch_process(Some("kitty"), true, false, "linux"),
        None
    );
}

#[test]
fn poll_until_returns_true_when_condition_met() {
    let mut n = 0;
    let ok = poll_until(
        || {
            n += 1;
            n >= 3
        },
        5,
        std::time::Duration::from_millis(0),
    );
    assert!(ok);
    assert_eq!(n, 3);
}

#[test]
fn poll_until_returns_false_when_never_met() {
    let ok = poll_until(|| false, 3, std::time::Duration::from_millis(0));
    assert!(!ok);
}

#[test]
fn resolve_terminal_path_joins_relative() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, ".").unwrap();
    // Normalize separators: the function emits `\` on Windows, `/` elsewhere.
    assert_eq!(result.replace('\\', "/"), "/home/user/my-project");
}

#[test]
fn resolve_terminal_path_joins_subdir() {
    let project_root = Path::new("/home/user/my-project");
    let result = resolve_terminal_path(project_root, "./src/server").unwrap();
    assert_eq!(
        result.replace('\\', "/"),
        "/home/user/my-project/src/server"
    );
}

#[test]
fn resolve_terminal_path_rejects_parent_escape() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "../../outside").is_err());
}

#[test]
fn resolve_terminal_path_rejects_absolute() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "/etc/passwd").is_err());
}

#[cfg(windows)]
#[test]
fn resolve_terminal_path_rejects_windows_rooted_path() {
    let project_root = Path::new(r"C:\Users\me\my-project");
    assert!(resolve_terminal_path(project_root, r"\Users\me").is_err());
}

#[cfg(windows)]
#[test]
fn resolve_terminal_path_rejects_windows_drive_relative_path() {
    let project_root = Path::new(r"C:\Users\me\my-project");
    assert!(resolve_terminal_path(project_root, r"C:Users\me").is_err());
}

#[test]
fn resolve_terminal_path_rejects_embedded_parent() {
    let project_root = Path::new("/home/user/my-project");
    assert!(resolve_terminal_path(project_root, "src/../../escape").is_err());
}

#[cfg(not(target_os = "windows"))]
#[test]
fn gnome_terminal_process_probe_uses_full_command_line() {
    assert_eq!(
        pgrep_args_for_process("gnome-terminal-server"),
        vec!["-f", "gnome-terminal-server"]
    );
}

#[cfg(not(target_os = "windows"))]
#[test]
fn default_process_probe_uses_exact_name() {
    assert_eq!(pgrep_args_for_process("ghostty"), vec!["-x", "ghostty"]);
}

fn sample_ctx() -> PlaceholderContext {
    PlaceholderContext {
        root: "/home/user/project".to_string(),
        config: "/home/user/project/.quickdev.toml".to_string(),
        name: "myproj".to_string(),
        cwd: "/home/user/elsewhere".to_string(),
    }
}

#[test]
fn resolve_app_args_dot_aliases_root() {
    let args = vec![".".to_string(), "--flag".to_string()];
    assert_eq!(
        resolve_app_args(&args, &sample_ctx()),
        vec!["/home/user/project", "--flag"]
    );
}

#[test]
fn resolve_app_args_expands_root_substring() {
    let args = vec!["{root}/README.md".to_string()];
    assert_eq!(
        resolve_app_args(&args, &sample_ctx()),
        vec!["/home/user/project/README.md"]
    );
}

#[test]
fn resolve_app_args_expands_all_placeholders() {
    let args = vec![
        "{config}".to_string(),
        "{name}".to_string(),
        "{cwd}".to_string(),
    ];
    assert_eq!(
        resolve_app_args(&args, &sample_ctx()),
        vec![
            "/home/user/project/.quickdev.toml",
            "myproj",
            "/home/user/elsewhere"
        ]
    );
}

#[test]
fn resolve_app_args_leaves_plain_and_unknown_untouched() {
    let args = vec![
        "--verbose".to_string(),
        "file.txt".to_string(),
        "{unknown}".to_string(),
    ];
    assert_eq!(
        resolve_app_args(&args, &sample_ctx()),
        vec!["--verbose", "file.txt", "{unknown}"]
    );
}

#[test]
fn normalize_path_expands_tilde() {
    let result = normalize_path("~/Documents/test");
    assert!(
        !result.starts_with('~'),
        "tilde should be expanded, got: {result}"
    );
}

#[test]
fn normalize_path_leaves_absolute_unchanged() {
    let result = normalize_path("/usr/local/bin/app");
    assert_eq!(result, "/usr/local/bin/app");
}

#[test]
fn escape_applescript_escapes_backslash_and_quote() {
    assert_eq!(escape_applescript_string(r#"a\b"c"#), r#"a\\b\"c"#);
}

#[test]
fn escape_applescript_leaves_plain_text() {
    assert_eq!(escape_applescript_string("plain text"), "plain text");
}

#[test]
fn escape_powershell_doubles_single_quotes() {
    assert_eq!(escape_powershell_single_quotes("Abrar's PC"), "Abrar''s PC");
}

#[test]
fn render_results_formats_success_detail_and_failure() {
    use quickdev::launch::{render_results, LaunchResult};
    let results = vec![
        LaunchResult {
            label: "dev".to_string(),
            kind: "terminal",
            success: true,
            error: None,
            detail: Some("/home/user/p · npm run dev".to_string()),
        },
        LaunchResult {
            label: "Cursor".to_string(),
            kind: "app",
            success: true,
            error: None,
            detail: Some("/Applications/Cursor.app".to_string()),
        },
        LaunchResult {
            label: "logs".to_string(),
            kind: "terminal",
            success: false,
            error: Some("bad path".to_string()),
            detail: None,
        },
    ];
    let out = render_results("Launched 2/3 items:", &results);
    assert_eq!(
        out,
        "Launched 2/3 items:\n  ✓ terminal dev — /home/user/p · npm run dev\n  ✓ app Cursor — /Applications/Cursor.app\n  ✗ terminal logs — bad path\n"
    );
}

#[test]
fn plan_launch_marks_valid_items_success() {
    use quickdev::launch::plan_launch;
    use quickdev::models::{AppEntry, ProjectConfig, ProjectEntry, TerminalEntry};
    let config = ProjectConfig {
        project: ProjectEntry {
            name: "p".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "dev".to_string(),
            path: "./src".to_string(),
            command: Some("npm run dev".to_string()),
            emulator: None,
        }],
        applications: vec![AppEntry {
            name: "Cursor".to_string(),
            path: "/Applications/Cursor.app".to_string(),
            args: None,
        }],
    };
    let plan = plan_launch(&config, Path::new("/home/user/project"));
    assert_eq!(plan.len(), 2);
    assert!(plan[0].success);
    // Normalize separators: the resolved terminal path uses `\` on Windows.
    assert_eq!(
        plan[0].detail.as_deref().map(|d| d.replace('\\', "/")),
        Some("/home/user/project/src · npm run dev".to_string())
    );
    assert!(plan[1].success);
    // Cursor is an editor tool with no args — detail should include project root.
    assert_eq!(
        plan[1].detail.as_deref(),
        Some("/Applications/Cursor.app · args: /home/user/project")
    );
}

#[test]
fn plan_launch_includes_resolved_app_args_in_detail() {
    use quickdev::launch::plan_launch;
    use quickdev::models::{AppEntry, ProjectConfig, ProjectEntry};
    let config = ProjectConfig {
        project: ProjectEntry {
            name: "p".to_string(),
        },
        terminals: vec![],
        applications: vec![AppEntry {
            name: "Cursor".to_string(),
            path: "/Applications/Cursor.app".to_string(),
            args: Some(vec![".".to_string(), "--flag".to_string()]),
        }],
    };

    let plan = plan_launch(&config, Path::new("/home/user/project"));

    assert_eq!(plan.len(), 1);
    assert!(plan[0].success);
    assert_eq!(
        plan[0].detail.as_deref(),
        Some("/Applications/Cursor.app · args: /home/user/project --flag")
    );
}

#[test]
fn resolve_app_args_does_not_double_expand_placeholder_values() {
    let ctx = PlaceholderContext {
        root: "/r".to_string(),
        config: "/r/.quickdev.toml".to_string(),
        name: "{cwd}".to_string(),
        cwd: "/the/cwd".to_string(),
    };
    // {name} expands to the literal "{cwd}" and must NOT be re-expanded.
    let args = vec!["{name}".to_string()];
    assert_eq!(resolve_app_args(&args, &ctx), vec!["{cwd}"]);
}

#[test]
fn plan_launch_flags_escaping_terminal_path() {
    use quickdev::launch::plan_launch;
    use quickdev::models::{ProjectConfig, ProjectEntry, TerminalEntry};
    let config = ProjectConfig {
        project: ProjectEntry {
            name: "p".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "bad".to_string(),
            path: "../escape".to_string(),
            command: None,
            emulator: None,
        }],
        applications: vec![],
    };
    let plan = plan_launch(&config, Path::new("/home/user/project"));
    assert_eq!(plan.len(), 1);
    assert!(!plan[0].success);
    assert!(plan[0].error.is_some());
}

#[test]
fn plan_launch_editor_app_without_args_previews_project_root() {
    use quickdev::launch::plan_launch;
    use quickdev::models::{AppEntry, ProjectConfig, ProjectEntry};
    let config = ProjectConfig {
        project: ProjectEntry {
            name: "p".to_string(),
        },
        terminals: vec![],
        applications: vec![AppEntry {
            name: "Cursor".to_string(),
            path: "/Applications/Cursor.app".to_string(),
            args: None,
        }],
    };
    let plan = plan_launch(&config, Path::new("/home/user/project"));
    assert_eq!(plan.len(), 1);
    let detail = plan[0].detail.as_deref().unwrap();
    // editor tool with no args should preview opening the project root
    assert!(
        detail.contains("/home/user/project"),
        "expected project root in detail, got: {detail}"
    );
}

#[test]
fn editor_args_uses_configured_args_when_present() {
    let resolved = vec![
        "/home/user/project".to_string(),
        "--reuse-window".to_string(),
    ];
    assert_eq!(editor_args(Some(&resolved), "/home/user/project"), resolved);
}

#[test]
fn editor_args_defaults_to_root_when_none_or_empty() {
    assert_eq!(editor_args(None, "/root"), vec!["/root".to_string()]);
    let empty: Vec<String> = vec![];
    assert_eq!(
        editor_args(Some(&empty), "/root"),
        vec!["/root".to_string()]
    );
}

#[test]
fn effective_app_args_defaults_editor_fallback_to_project_root() {
    assert_eq!(
        effective_app_args("Cursor", "/Applications/Cursor.app", None, "/project"),
        Some(vec!["/project".to_string()])
    );
}

#[test]
fn tab_strategy_selects_macos_ghostty_applescript_when_supported() {
    let caps = TabCapabilities {
        ghostty_available: true,
        ghostty_version: Some("1.3.0".to_string()),
        ghostty_applescript: true,
        ptyxis_available: false,
        gnome_terminal_available: false,
        wt_available: false,
    };

    assert_eq!(
        select_tab_strategy("macos", Some("ghostty"), &caps),
        TabStrategy::AppleScriptTab
    );
    // An unspecified emulator resolves to Ghostty AppleScript too.
    assert_eq!(
        select_tab_strategy("macos", None, &caps),
        TabStrategy::AppleScriptTab
    );
}

#[test]
fn tab_strategy_rejects_macos_ghostty_without_applescript_support() {
    let caps = TabCapabilities {
        ghostty_available: true,
        ghostty_version: Some("1.2.0".to_string()),
        ghostty_applescript: true,
        ptyxis_available: false,
        gnome_terminal_available: false,
        wt_available: false,
    };

    assert_eq!(
        select_tab_strategy("macos", Some("ghostty"), &caps),
        TabStrategy::WindowOnly
    );
}

#[test]
fn tab_strategy_macos_none_keeps_ghostty_windows_when_applescript_unsupported() {
    // Ghostty installed but AppleScript unusable: must stay WindowOnly so the
    // fallback loop opens Ghostty CLI windows, not silently switch to
    // Terminal.app tabs. Covers both the old-version and disabled cases.
    let old_version = TabCapabilities {
        ghostty_available: true,
        ghostty_version: Some("1.2.0".to_string()),
        ghostty_applescript: true,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("macos", None, &old_version),
        TabStrategy::WindowOnly
    );

    let applescript_off = TabCapabilities {
        ghostty_available: true,
        ghostty_version: Some("1.3.1".to_string()),
        ghostty_applescript: false,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("macos", None, &applescript_off),
        TabStrategy::WindowOnly
    );
}

#[test]
fn tab_strategy_macos_none_uses_terminal_app_only_without_ghostty() {
    let caps = TabCapabilities {
        ghostty_available: false,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("macos", None, &caps),
        TabStrategy::TerminalAppTab
    );
}

#[test]
fn tab_strategy_linux_ptyxis_present_auto_is_window_only() {
    // Ptyxis has no single-window CLI tab support; when it is available auto
    // selection must prefer WindowOnly even if gnome-terminal is also present.
    let caps = TabCapabilities {
        ghostty_available: false,
        ghostty_version: None,
        ghostty_applescript: false,
        ptyxis_available: true,
        gnome_terminal_available: true,
        wt_available: false,
    };

    assert_eq!(
        select_tab_strategy("linux", None, &caps),
        TabStrategy::WindowOnly
    );
}

#[test]
fn ghostty_applescript_builds_window_then_tabs_with_configuration() {
    let script = build_script(&[
        ResolvedTerminal {
            cwd: "/Users/me/project",
            command: Some("npm run dev"),
        },
        ResolvedTerminal {
            cwd: "/Users/me/project/logs",
            command: None,
        },
    ])
    .unwrap();

    assert!(script.contains("tell application \"Ghostty\""));
    assert!(script.contains("set w to new window with configuration c0"));
    assert!(script.contains("new tab in w with configuration c1"));
    assert!(script.contains("initial working directory:\"/Users/me/project\""));
    // Commands are typed into the interactive shell, not run as a one-shot
    // `command`, so the shell stays open afterward.
    assert!(script.contains("initial input:\"npm run dev\" & return"));
    assert!(!script.contains("wait after command"));
    assert!(!script.contains("command:\"npm run dev\""));
}

#[test]
fn ghostty_applescript_escapes_paths_and_commands() {
    let script = build_script(&[ResolvedTerminal {
        cwd: r#"/Users/me/Quote "Project"/a\b"#,
        command: Some(r#"printf "ok\done""#),
    }])
    .unwrap();

    assert!(script.contains(r#"Quote \"Project\"/a\\b"#));
    assert!(script.contains(r#"initial input:"printf \"ok\\done\"" & return"#));
}

#[test]
fn terminal_app_command_quotes_working_directory() {
    assert_eq!(
        build_terminal_command("/Users/me/O'Reilly", Some("npm test")),
        "cd '/Users/me/O'\\''Reilly' && npm test"
    );
}

#[test]
fn terminal_app_auto_tab_script_uses_plain_do_script_calls() {
    let script = build_auto_tab_script(&[
        ResolvedTerminal {
            cwd: "/Users/me/project",
            command: Some("npm run dev"),
        },
        ResolvedTerminal {
            cwd: "/Users/me/project/logs",
            command: None,
        },
    ]);

    assert!(script.contains("tell application \"Terminal\""));
    assert!(script.contains("do script \"cd '/Users/me/project' && npm run dev\""));
    assert!(script.contains("do script \"cd '/Users/me/project/logs'\""));
    assert!(!script.contains("System Events"));
}

#[test]
fn terminal_app_tabbing_preference_parses_always() {
    assert_eq!(
        parse_tabbing_preference("always\n"),
        Some(TabbingPreference::Always)
    );
    assert_eq!(
        parse_tabbing_preference("fullscreen"),
        Some(TabbingPreference::Fullscreen)
    );
    assert_eq!(parse_tabbing_preference(""), None);
}

#[test]
fn linux_gnome_terminal_explicit_uses_load_config() {
    let caps = TabCapabilities {
        gnome_terminal_available: true,
        ptyxis_available: true,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("linux", Some("gnome-terminal"), &caps),
        TabStrategy::GnomeTerminalLoadConfig
    );
}

#[test]
fn linux_ptyxis_and_ghostty_explicit_are_window_only() {
    let caps = TabCapabilities {
        gnome_terminal_available: true,
        ptyxis_available: true,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("linux", Some("ptyxis"), &caps),
        TabStrategy::WindowOnly
    );
    assert_eq!(
        select_tab_strategy("linux", Some("ghostty"), &caps),
        TabStrategy::WindowOnly
    );
}

#[test]
fn linux_auto_prefers_ptyxis_windows_over_gnome_tabs() {
    let caps = TabCapabilities {
        gnome_terminal_available: true,
        ptyxis_available: true,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("linux", None, &caps),
        TabStrategy::WindowOnly
    );
    assert_eq!(
        select_tab_strategy("linux", Some("terminal"), &caps),
        TabStrategy::WindowOnly
    );
}

#[test]
fn linux_auto_tabs_when_only_gnome_terminal() {
    let caps = TabCapabilities {
        gnome_terminal_available: true,
        ptyxis_available: false,
        ..TabCapabilities::default()
    };
    assert_eq!(
        select_tab_strategy("linux", None, &caps),
        TabStrategy::GnomeTerminalLoadConfig
    );
}

#[test]
fn linux_no_terminals_is_window_only() {
    let caps = TabCapabilities::default();
    assert_eq!(
        select_tab_strategy("linux", None, &caps),
        TabStrategy::WindowOnly
    );
}
