use quickdev::config::{
    find_project_config, load_global_config, load_project_config, register_existing_project_config,
    remove_config_with, resolve_project_config, save_global_config, save_project_config,
    unique_project_name,
};
use quickdev::models::{
    GlobalConfig, GlobalProjectEntry, ProjectConfig, ProjectEntry, TerminalEntry,
};
use std::fs;

#[test]
fn save_and_load_project_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "dev".to_string(),
            path: ".".to_string(),
            command: Some("cargo run".to_string()),
            emulator: None,
        }],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();
    let loaded = load_project_config(&config_path).unwrap();

    assert_eq!(loaded.project.name, "test-proj");
    assert_eq!(loaded.terminals.len(), 1);
    assert_eq!(loaded.terminals[0].name, "dev");
}

#[test]
fn save_and_load_global_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = GlobalConfig {
        emulator: None,
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![GlobalProjectEntry {
            name: "proj-a".to_string(),
            path: "/tmp/proj-a".to_string(),
        }],
    };

    save_global_config(&config_path, &config).unwrap();
    let loaded = load_global_config(&config_path).unwrap();

    assert_eq!(loaded.projects.len(), 1);
    assert_eq!(loaded.projects[0].name, "proj-a");
}

#[test]
fn load_global_config_creates_empty_if_missing() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("nonexistent").join("config.toml");

    let loaded = load_global_config(&config_path).unwrap();
    assert!(loaded.projects.is_empty());
}

#[test]
fn find_project_config_walks_parents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let nested = root.join("a").join("b").join("c");
    fs::create_dir_all(&nested).unwrap();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "root-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&root.join(".quickdev.toml"), &config).unwrap();

    let found = find_project_config(&nested).unwrap();
    assert_eq!(found.0, root.join(".quickdev.toml"));
    assert_eq!(found.1, root.to_path_buf());
}

#[test]
fn find_project_config_returns_error_if_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let result = find_project_config(dir.path());
    assert!(result.is_err());
}

#[test]
fn unique_project_name_appends_suffix() {
    let config = GlobalConfig {
        emulator: None,
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![
            GlobalProjectEntry {
                name: "my-app".to_string(),
                path: "/a".to_string(),
            },
            GlobalProjectEntry {
                name: "my-app-2".to_string(),
                path: "/b".to_string(),
            },
        ],
    };

    assert_eq!(unique_project_name("my-app", &config), "my-app-3");
    assert_eq!(unique_project_name("new-proj", &config), "new-proj");
}

#[test]
fn save_project_config_adds_comment_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev project configuration"),
        "should start with comment header, got:\n{content}"
    );
    assert!(content.contains("[project]"));
    assert!(content.contains("name = \"test-proj\""));
}

#[test]
fn save_project_config_preserves_existing_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    save_project_config(&config_path, &config).unwrap();

    let config2 = ProjectConfig {
        project: ProjectEntry {
            name: "test-proj".to_string(),
        },
        terminals: vec![TerminalEntry {
            name: "dev".to_string(),
            path: ".".to_string(),
            command: None,
            emulator: None,
        }],
        applications: vec![],
    };
    save_project_config(&config_path, &config2).unwrap();

    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev project configuration"),
        "header should be preserved after re-save"
    );
    assert!(content.contains("[[terminals]]"));
    assert!(content.contains("name = \"dev\""));
    assert_eq!(
        content.matches("# QuickDev project configuration").count(),
        1,
        "header should not be duplicated"
    );
}

#[test]
fn resolve_project_config_finds_local() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "local-proj".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&root.join(".quickdev.toml"), &config).unwrap();

    let result = resolve_project_config(root);
    assert!(result.is_ok());
    let (config_path, project_root) = result.unwrap();
    assert_eq!(config_path, root.join(".quickdev.toml"));
    assert_eq!(project_root, root.to_path_buf());
}

#[test]
fn renamed_project_config_persists() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let cfg = ProjectConfig {
        project: ProjectEntry {
            name: "api".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&config_path, &cfg).unwrap();

    // Global already has "api" -> init must pick a unique name.
    let global = GlobalConfig {
        emulator: None,
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![GlobalProjectEntry {
            name: "api".to_string(),
            path: "/tmp/other".to_string(),
        }],
    };
    let unique = unique_project_name("api", &global);
    assert_eq!(unique, "api-2");

    // The fix writes the unique name back to the local config.
    let mut existing = load_project_config(&config_path).unwrap();
    existing.project.name = unique.clone();
    save_project_config(&config_path, &existing).unwrap();

    let reloaded = load_project_config(&config_path).unwrap();
    assert_eq!(reloaded.project.name, "api-2");
}

#[test]
fn register_existing_project_config_syncs_local_name_and_global_index() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    let cfg = ProjectConfig {
        project: ProjectEntry {
            name: "api".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };
    save_project_config(&config_path, &cfg).unwrap();

    let mut global = GlobalConfig {
        emulator: None,
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![GlobalProjectEntry {
            name: "api".to_string(),
            path: "/tmp/other".to_string(),
        }],
    };

    let registered_name = register_existing_project_config(
        &config_path,
        dir.path().to_string_lossy().to_string(),
        &mut global,
    )
    .unwrap();

    assert_eq!(registered_name, "api-2");
    assert_eq!(global.projects.last().unwrap().name, "api-2");
    assert_eq!(
        global.projects.last().unwrap().path,
        dir.path().to_string_lossy()
    );
    let reloaded = load_project_config(&config_path).unwrap();
    assert_eq!(reloaded.project.name, "api-2");
}

#[test]
fn save_global_config_adds_comment_header() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let config = GlobalConfig {
        emulator: Some("ghostty".to_string()),
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![],
    };

    save_global_config(&config_path, &config).unwrap();
    let content = std::fs::read_to_string(&config_path).unwrap();

    assert!(
        content.starts_with("# QuickDev global configuration"),
        "should start with comment header"
    );
    assert!(content.contains("emulator = \"ghostty\""));
}

#[test]
fn is_supported_emulator_accepts_known_only() {
    use quickdev::config::is_supported_emulator;
    assert!(is_supported_emulator("ghostty"));
    assert!(is_supported_emulator("terminal"));
    assert!(is_supported_emulator("kitty"));
    assert!(!is_supported_emulator("nonexistent-terminal"));
}

#[test]
fn gnome_terminal_and_ptyxis_are_supported_emulators() {
    use quickdev::config::is_supported_emulator;
    assert!(is_supported_emulator("gnome-terminal"));
    assert!(is_supported_emulator("ptyxis"));
}

#[test]
fn set_global_emulator_accepts_new_linux_terminals() {
    use quickdev::config::set_global_setting;
    for value in ["gnome-terminal", "ptyxis"] {
        let mut config = GlobalConfig {
            emulator: None,
            terminal_app_tabbing_prompt_declined: false,
            projects: vec![],
        };
        let msg = set_global_setting(&mut config, "emulator", value).unwrap();
        assert_eq!(config.emulator.as_deref(), Some(value));
        assert!(msg.contains(value));
    }
}

#[test]
fn set_get_unset_global_emulator() {
    use quickdev::config::{get_global_setting, set_global_setting, unset_global_setting};
    let mut config = GlobalConfig {
        emulator: None,
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![],
    };
    set_global_setting(&mut config, "emulator", "ghostty").unwrap();
    assert_eq!(config.emulator.as_deref(), Some("ghostty"));
    assert_eq!(
        get_global_setting(&config, "emulator").unwrap(),
        "emulator = ghostty"
    );
    unset_global_setting(&mut config, "emulator").unwrap();
    assert!(config.emulator.is_none());
    assert_eq!(
        get_global_setting(&config, "emulator").unwrap(),
        "emulator is not set (auto-detect)"
    );
}

#[test]
fn set_global_setting_rejects_bad_value_and_unknown_key() {
    use quickdev::config::set_global_setting;
    let mut config = GlobalConfig {
        emulator: None,
        terminal_app_tabbing_prompt_declined: false,
        projects: vec![],
    };
    assert!(set_global_setting(&mut config, "emulator", "nonexistent-terminal").is_err());
    assert!(set_global_setting(&mut config, "theme", "dark").is_err());
    assert!(config.emulator.is_none());
}

#[test]
fn rewriting_a_project_config_keeps_comments_and_unmodelled_keys() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    fs::write(
        &config_path,
        r#"# my own notes about this project
[project]
name = "demo"

[[terminals]]
# the API server, do not remove
name = "api"
path = "./api"
written_by_a_newer_quickdev = "keep me"
"#,
    )
    .unwrap();

    let mut config = load_project_config(&config_path).unwrap();
    config.terminals.push(TerminalEntry {
        name: "web".to_string(),
        path: "./web".to_string(),
        command: Some("npm run dev".to_string()),
        emulator: None,
    });
    save_project_config(&config_path, &config).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.starts_with("# my own notes about this project"));
    assert!(content.contains("# the API server, do not remove"));
    assert!(
        content.contains("written_by_a_newer_quickdev = \"keep me\""),
        "a key this build does not model must survive a rewrite: {content}"
    );
    // And the new terminal actually landed.
    let reloaded = load_project_config(&config_path).unwrap();
    assert_eq!(reloaded.terminals.len(), 2);
    assert_eq!(reloaded.terminals[1].name, "web");
    assert_eq!(
        reloaded.terminals[1].command.as_deref(),
        Some("npm run dev")
    );
}

#[test]
fn removing_an_optional_field_clears_it_from_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");

    fs::write(
        &config_path,
        "[project]\nname = \"demo\"\n\n[[terminals]]\nname = \"api\"\npath = \".\"\ncommand = \"echo hi\"\n",
    )
    .unwrap();

    let mut config = load_project_config(&config_path).unwrap();
    config.terminals[0].command = None;
    save_project_config(&config_path, &config).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();
    assert!(
        !content.contains("command"),
        "stale key left behind: {content}"
    );
    assert!(load_project_config(&config_path).unwrap().terminals[0]
        .command
        .is_none());
}

#[test]
fn adding_a_top_level_key_to_a_global_config_with_projects_stays_valid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");

    // A global config whose only content is an array-of-tables. A naive rewrite
    // would append `emulator = ...` after `[[projects]]`, silently making it a
    // key of the last project instead of a top-level setting.
    fs::write(
        &path,
        "terminal_app_tabbing_prompt_declined = false\n\n[[projects]]\nname = \"demo\"\npath = \"/tmp/demo\"\n",
    )
    .unwrap();

    let mut global = load_global_config(&path).unwrap();
    global.emulator = Some("kitty".to_string());
    save_global_config(&path, &global).unwrap();

    let reloaded = load_global_config(&path).unwrap();
    assert_eq!(reloaded.emulator.as_deref(), Some("kitty"));
    assert_eq!(reloaded.projects.len(), 1);
    assert_eq!(reloaded.projects[0].name, "demo");
}

#[test]
fn saving_over_an_unparseable_config_refuses_rather_than_destroying_it() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");
    let broken = "[project\nname = \"demo\"\n";
    fs::write(&config_path, broken).unwrap();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "demo".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    assert!(save_project_config(&config_path, &config).is_err());
    assert_eq!(fs::read_to_string(&config_path).unwrap(), broken);
}

#[cfg(unix)]
#[test]
fn saving_keeps_the_existing_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join(".quickdev.toml");
    fs::write(&config_path, "[project]\nname = \"demo\"\n").unwrap();
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o644)).unwrap();

    let config = load_project_config(&config_path).unwrap();
    save_project_config(&config_path, &config).unwrap();

    assert_eq!(
        fs::metadata(&config_path).unwrap().permissions().mode() & 0o777,
        0o644,
        "an atomic replace must not silently re-chmod the user's config"
    );
}

#[test]
fn saving_refuses_when_the_file_changed_since_it_was_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".quickdev.toml");
    fs::write(&path, "[project]\nname = \"demo\"\n").unwrap();

    let config = load_project_config(&path).unwrap();

    // Stands in for a second quickdev invocation writing between our load and
    // our save. Without the check, the save below would silently discard it.
    fs::write(
        &path,
        "[project]\nname = \"demo\"\n\n[[terminals]]\nname = \"added-elsewhere\"\npath = \".\"\n",
    )
    .unwrap();

    let err = save_project_config(&path, &config).unwrap_err();
    assert!(err.contains("changed on disk"), "got: {err}");
    assert!(
        fs::read_to_string(&path)
            .unwrap()
            .contains("added-elsewhere"),
        "the concurrent write must survive"
    );
}

#[test]
fn saving_refuses_to_overwrite_a_file_this_process_never_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".quickdev.toml");
    let existing = "[project]\nname = \"someone-elses\"\n";
    fs::write(&path, existing).unwrap();

    let config = ProjectConfig {
        project: ProjectEntry {
            name: "mine".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    assert!(save_project_config(&path, &config).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), existing);
}

#[test]
fn repeated_saves_in_one_process_are_allowed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".quickdev.toml");

    let mut config = ProjectConfig {
        project: ProjectEntry {
            name: "demo".to_string(),
        },
        terminals: vec![],
        applications: vec![],
    };

    // Creating it, then editing it again, must not trip the change detection:
    // a save records what it wrote.
    save_project_config(&path, &config).unwrap();
    config.terminals.push(TerminalEntry {
        name: "api".to_string(),
        path: ".".to_string(),
        command: None,
        emulator: None,
    });
    save_project_config(&path, &config).unwrap();

    assert_eq!(load_project_config(&path).unwrap().terminals.len(), 1);
}

/// Writes a `.quickdev.toml` and returns its path.
fn seeded_config(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join(".quickdev.toml");
    fs::write(&path, "[project]\nname = \"demo\"\n").unwrap();
    path
}

/// The staging file `remove_config_with` created, if it is still there.
fn staged_leftovers(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(".quickdev-deregister-"))
        })
        .collect()
}

#[test]
fn remove_config_with_commits_and_leaves_nothing_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_config(dir.path());

    remove_config_with(&path, || Ok(())).unwrap();

    assert!(!path.exists());
    assert!(staged_leftovers(dir.path()).is_empty());
}

#[test]
fn remove_config_with_does_not_clobber_an_earlier_recovery_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_config(dir.path());

    // A recovery copy left by an earlier failed cleanup. A fixed staging name
    // would be renamed over, destroying it.
    let earlier = dir.path().join(".quickdev-deregister-earlier");
    fs::write(&earlier, "an earlier recovery copy").unwrap();

    remove_config_with(&path, || Ok(())).unwrap();

    assert_eq!(
        fs::read_to_string(&earlier).unwrap(),
        "an earlier recovery copy"
    );
}

#[test]
fn remove_config_with_restores_the_config_when_the_commit_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_config(dir.path());

    let err =
        remove_config_with(&path, || Err::<(), _>("index write failed".to_string())).unwrap_err();

    assert_eq!(err, "index write failed");
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "[project]\nname = \"demo\"\n",
        "the config must be exactly as it started"
    );
    assert!(staged_leftovers(dir.path()).is_empty());
}

#[test]
fn remove_config_with_names_the_recovery_path_when_restoration_also_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_config(dir.path());
    let blocked = path.clone();

    let err = remove_config_with(&path, move || {
        // Fault injection: occupying the config's path with a directory makes
        // the restoring rename fail, the one case where the caller is left with
        // data only in the staging file.
        fs::create_dir(&blocked).unwrap();
        Err::<(), _>("index write failed".to_string())
    })
    .unwrap_err();

    assert!(err.contains("index write failed"));
    assert!(err.contains("could not be put back"), "got: {err}");
    let leftovers = staged_leftovers(dir.path());
    assert_eq!(leftovers.len(), 1, "the staged copy must survive");
    assert!(
        err.contains(leftovers[0].to_str().unwrap()),
        "the error must name the recovery path; got: {err}"
    );
    assert_eq!(
        fs::read_to_string(&leftovers[0]).unwrap(),
        "[project]\nname = \"demo\"\n"
    );
}

#[test]
fn remove_config_with_reports_a_failed_staged_cleanup() {
    let dir = tempfile::tempdir().unwrap();
    let path = seeded_config(dir.path());
    let parent = dir.path().to_path_buf();

    let err = remove_config_with(&path, move || {
        // Fault injection: swap the staged file for a directory of the same
        // name, so the final `remove_file` fails after the commit succeeded.
        let staged = staged_leftovers(&parent).pop().unwrap();
        fs::remove_file(&staged).unwrap();
        fs::create_dir(&staged).unwrap();
        Ok(())
    })
    .unwrap_err();

    assert!(err.contains("could not be deleted"), "got: {err}");
    assert!(!path.exists(), "the commit still happened");
}
