//! Tests that drive the built binary. `cli.rs` lives in the binary crate and is
//! `pub(crate)`, so the clap wiring can't be imported — Cargo hands integration
//! tests the compiled binary's path via `CARGO_BIN_EXE_<name>` instead.

use std::process::Command;

fn quickdev() -> Command {
    Command::new(env!("CARGO_BIN_EXE_quickdev"))
}

#[test]
fn version_flag_reports_the_crate_version() {
    let output = quickdev().arg("--version").output().unwrap();

    assert!(
        output.status.success(),
        "--version exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // clap prints "<name> <version>"; assert on the version so a stale or
    // hardcoded value can't pass.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("quickdev {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn short_version_flag_matches_long_form() {
    let long = quickdev().arg("--version").output().unwrap();
    let short = quickdev().arg("-V").output().unwrap();

    assert!(short.status.success());
    assert_eq!(short.stdout, long.stdout);
}

#[test]
fn version_flag_does_not_require_a_subcommand() {
    // Every other invocation demands a subcommand; --version must short-circuit
    // that, otherwise it would exit 2 with a usage error.
    let output = quickdev().arg("--version").output().unwrap();
    assert_eq!(output.status.code(), Some(0));

    // Sanity check the contrast: a bare invocation still errors.
    let bare = quickdev().output().unwrap();
    assert!(!bare.status.success());
}

/// Run quickdev in `dir` with an isolated global config, so no test can touch
/// (or migrate) the developer's real one.
fn quickdev_in(dir: &std::path::Path) -> Command {
    let mut cmd = quickdev();
    cmd.current_dir(dir)
        .env("QUICKDEV_CONFIG", dir.join("global.toml"))
        .env_remove("NO_COLOR");
    cmd
}

#[test]
fn validate_json_reports_an_unparseable_config_as_invalid() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".quickdev.toml"), "[project\n").unwrap();

    let output = quickdev_in(dir.path())
        .args(["validate", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    assert!(stdout.contains("\"valid\": false"), "got: {stdout}");
    assert!(
        stdout.contains("failed to parse project config"),
        "got: {stdout}"
    );
}

#[test]
fn color_flag_takes_precedence_over_color_environment_variables() {
    // Explicit flag beats environment, as with cargo's --color: CI often sets
    // CLICOLOR_FORCE globally, and --color never must still yield plain text.
    let dir = tempfile::tempdir().unwrap();
    let esc = |output: std::process::Output| output.stdout.contains(&0x1b);

    let forced_off = quickdev_in(dir.path())
        .args(["--color", "never", "prune"])
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();
    assert!(
        !esc(forced_off),
        "--color never must win over CLICOLOR_FORCE"
    );

    let forced_on = quickdev_in(dir.path())
        .args(["--color", "always", "prune"])
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(esc(forced_on), "--color always must win over NO_COLOR");

    let env_only = quickdev_in(dir.path())
        .arg("prune")
        .env("CLICOLOR_FORCE", "1")
        .output()
        .unwrap();
    assert!(esc(env_only), "without the flag, CLICOLOR_FORCE applies");
}

#[test]
fn validate_json_reports_a_missing_config_without_prompting() {
    // No .quickdev.toml here or above: JSON mode must not open the project
    // picker (a cancelled picker exits 0), but report the failure as JSON.
    let dir = tempfile::tempdir().unwrap();

    let output = quickdev_in(dir.path())
        .args(["validate", "--json"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success());
    assert!(stdout.contains("\"path\": null"), "got: {stdout}");
    assert!(stdout.contains("\"valid\": false"), "got: {stdout}");
    assert!(stdout.contains("no .quickdev.toml found"), "got: {stdout}");
}

#[test]
fn shell_completion_offers_registered_project_names() {
    // Guards the dynamic-completion wiring, which rides on clap_complete's
    // unstable API (pinned exactly in Cargo.toml).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("global.toml"),
        "[[projects]]\nname = \"my-api\"\npath = \"/p/my-api\"\n",
    )
    .unwrap();

    let output = quickdev_in(dir.path())
        .env("COMPLETE", "fish")
        .args(["--", "quickdev", "launch", "my"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success());
    assert!(stdout.contains("my-api\t/p/my-api"), "got: {stdout}");

    let script = quickdev_in(dir.path())
        .args(["completions", "zsh"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&script.stdout).contains("COMPLETE=\"zsh\""));
}
