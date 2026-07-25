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
