use crate::adapters::command_exists;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn check_fzf() -> bool {
    command_exists("fzf")
}

pub fn fzf_install_hint() -> String {
    let os_hint = if cfg!(target_os = "macos") {
        "brew install fzf"
    } else if cfg!(target_os = "windows") {
        "choco install fzf"
    } else {
        "apt install fzf"
    };
    format!("fzf is required for interactive selection.\nInstall: {os_hint}")
}

pub fn fzf_select_one(items: &[String], header: &str) -> Result<String, String> {
    if !check_fzf() {
        return Err(fzf_install_hint());
    }
    if items.is_empty() {
        return Err("no items to select from".to_string());
    }

    let input = items.join("\n");

    let mut child = Command::new("fzf")
        .args(["--header", header, "--height", "~50%", "--reverse"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to start fzf: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("failed to write to fzf: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("fzf failed: {e}"))?;

    if !output.status.success() {
        return Err("selection cancelled".to_string());
    }

    let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if selected.is_empty() {
        return Err("no selection made".to_string());
    }

    Ok(selected)
}

pub fn fzf_select_multi(items: &[String], header: &str) -> Result<Vec<String>, String> {
    if !check_fzf() {
        return Err(fzf_install_hint());
    }
    if items.is_empty() {
        return Err("no items to select from".to_string());
    }

    let input = items.join("\n");

    let mut child = Command::new("fzf")
        .args([
            "--multi",
            "--header",
            header,
            "--height",
            "~50%",
            "--reverse",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("failed to start fzf: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(input.as_bytes())
            .map_err(|e| format!("failed to write to fzf: {e}"))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("fzf failed: {e}"))?;

    if !output.status.success() {
        return Err("selection cancelled".to_string());
    }

    let selected: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if selected.is_empty() {
        return Err("no selection made".to_string());
    }

    Ok(selected)
}
