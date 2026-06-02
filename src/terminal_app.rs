use crate::ghostty_applescript::ResolvedTerminal;
use crate::launch::escape_applescript_string;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabbingPreference {
    Always,
    Fullscreen,
    Manual,
}

pub fn parse_tabbing_preference(output: &str) -> Option<TabbingPreference> {
    match output.trim() {
        "always" => Some(TabbingPreference::Always),
        "fullscreen" => Some(TabbingPreference::Fullscreen),
        "manual" => Some(TabbingPreference::Manual),
        "" => None,
        _ => None,
    }
}

pub fn build_terminal_command(cwd: &str, command: Option<&str>) -> String {
    let cd_part = format!("cd '{}'", cwd.replace('\'', "'\\''"));
    match command {
        Some(cmd) if !cmd.is_empty() => format!("{cd_part} && {cmd}"),
        _ => cd_part,
    }
}

pub fn build_auto_tab_script(terminals: &[ResolvedTerminal<'_>]) -> String {
    let mut script = String::from("tell application \"Terminal\"\n  activate\n");
    for terminal in terminals {
        let command =
            escape_applescript_string(&build_terminal_command(terminal.cwd, terminal.command));
        script.push_str(&format!("  do script \"{command}\"\n"));
    }
    script.push_str("end tell");
    script
}

pub fn build_window_script(terminal: ResolvedTerminal<'_>) -> String {
    let command =
        escape_applescript_string(&build_terminal_command(terminal.cwd, terminal.command));
    format!("tell application \"Terminal\"\n  activate\n  do script \"{command}\"\nend tell")
}

pub fn build_system_events_tab_script(terminals: &[ResolvedTerminal<'_>]) -> String {
    let mut script = String::from("tell application \"Terminal\"\n  activate\n");
    for (index, terminal) in terminals.iter().enumerate() {
        let command =
            escape_applescript_string(&build_terminal_command(terminal.cwd, terminal.command));
        if index == 0 {
            script.push_str(&format!("  do script \"{command}\"\n"));
        } else {
            script.push_str("  tell application \"System Events\" to tell process \"Terminal\" to keystroke \"t\" using command down\n");
            script.push_str("  delay 0.3\n");
            script.push_str(&format!(
                "  do script \"{command}\" in selected tab of front window\n"
            ));
        }
    }
    script.push_str("end tell");
    script
}

pub fn read_tabbing_preference() -> Option<TabbingPreference> {
    read_domain_tabbing_preference("com.apple.Terminal")
        .or_else(|| read_domain_tabbing_preference("NSGlobalDomain"))
}

fn read_domain_tabbing_preference(domain: &str) -> Option<TabbingPreference> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("defaults")
            .args(["read", domain, "AppleWindowTabbingMode"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        parse_tabbing_preference(&String::from_utf8_lossy(&output.stdout))
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = domain;
        None
    }
}

pub fn prompt_to_enable_terminal_tabbing(declined: bool) -> PromptOutcome {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = declined;
        return PromptOutcome::NoChange;
    }

    #[cfg(target_os = "macos")]
    {
        if declined || !io::stdout().is_terminal() {
            return PromptOutcome::NoChange;
        }

        print!("Enable Terminal.app tabs by setting AppleWindowTabbingMode=always? [y/N] ");
        let _ = io::stdout().flush();

        let mut answer = String::new();
        if io::stdin().read_line(&mut answer).is_err() {
            return PromptOutcome::NoChange;
        }

        if matches!(answer.trim().to_lowercase().as_str(), "y" | "yes") {
            match write_terminal_tabbing_always() {
                Ok(()) => PromptOutcome::Accepted,
                Err(e) => PromptOutcome::WriteFailed(e),
            }
        } else {
            PromptOutcome::Declined
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    Accepted,
    Declined,
    NoChange,
    WriteFailed(String),
}

fn write_terminal_tabbing_always() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("defaults")
            .args([
                "write",
                "com.apple.Terminal",
                "AppleWindowTabbingMode",
                "-string",
                "always",
            ])
            .status()
            .map_err(|e| format!("failed to run defaults: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("defaults write failed".to_string())
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}
