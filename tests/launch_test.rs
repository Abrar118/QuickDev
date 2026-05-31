#[cfg(not(target_os = "windows"))]
use quickdev::launch::pgrep_args_for_process;
use quickdev::launch::{
    editor_args, effective_app_args, emulator_watch_process, escape_applescript_string,
    escape_powershell_single_quotes, normalize_path, poll_until, resolve_app_args,
    resolve_terminal_path, PlaceholderContext,
};
use std::path::Path;

#[test]
fn watch_process_prefers_ghostty_when_available() {
    assert_eq!(emulator_watch_process(None, true, "macos"), Some("ghostty"));
    assert_eq!(emulator_watch_process(None, true, "linux"), Some("ghostty"));
}

#[test]
fn watch_process_falls_back_to_native_when_no_ghostty() {
    assert_eq!(
        emulator_watch_process(None, false, "macos"),
        Some("Terminal")
    );
    assert_eq!(
        emulator_watch_process(None, false, "linux"),
        Some("gnome-terminal-server")
    );
    assert_eq!(
        emulator_watch_process(None, false, "windows"),
        Some("WindowsTerminal.exe")
    );
}

#[test]
fn watch_process_honors_explicit_emulator() {
    assert_eq!(
        emulator_watch_process(Some("ghostty"), false, "linux"),
        Some("ghostty")
    );
    assert_eq!(
        emulator_watch_process(Some("terminal"), false, "macos"),
        Some("Terminal")
    );
}

#[test]
fn watch_process_unknown_emulator_is_none() {
    assert_eq!(emulator_watch_process(Some("kitty"), true, "linux"), None);
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
