use crate::config::{
    find_project_config, global_config_path_readonly, load_global_config, load_project_config,
    SUPPORTED_EMULATORS,
};
use clap::{Parser, Subcommand};
use clap_complete::{ArgValueCandidates, CompletionCandidate};

#[derive(Parser)]
#[command(
    name = "quickdev",
    // Sourced from CARGO_PKG_VERSION, so `quickdev --version` always reports the
    // version the binary was built at — the only way to check which release an
    // installed binary or the npm wrapper actually shipped.
    version,
    about = "Manage and launch project terminal/app configurations",
    after_help = "\
Examples:
  quickdev init                                         Initialize a project
  quickdev init --from my-api                           Clone config from another project
  quickdev launch                                       Select items to launch
  quickdev launch --all                                 Launch everything
  quickdev launch my-api                                Interactive picker for a named project
  quickdev add                                          Interactive add
  quickdev remove                                       Interactive removal picker
  quickdev list                                         Show all projects
  quickdev edit                                         Edit project config
  quickdev edit --global                                Edit global config
  quickdev deregister                                   Unregister project
  quickdev completions zsh                              Print shell completion setup"
)]
pub(crate) struct Cli {
    /// When to use colors: auto (default; off when piped or NO_COLOR is set), always, never
    #[arg(long, global = true, value_name = "WHEN", default_value = "auto")]
    pub(crate) color: clap::ColorChoice,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Create .quickdev.toml in the current directory and register the project
    Init {
        /// Clone config from another project by name
        #[arg(long, add = ArgValueCandidates::new(project_candidates))]
        from: Option<String>,
    },
    /// Launch terminals and applications for a project
    Launch {
        /// Project to launch from the global index (omit to use current directory); the picker still appears unless --all
        #[arg(add = ArgValueCandidates::new(project_candidates))]
        project: Option<String>,
        /// Launch all items without interactive selection
        #[arg(long)]
        all: bool,
        /// Print what would launch without starting anything
        #[arg(long)]
        dry_run: bool,
    },
    /// List all indexed projects
    List {
        /// Show only projects whose path or .quickdev.toml is missing
        #[arg(long)]
        missing: bool,
        /// Output as a JSON array
        #[arg(long)]
        json: bool,
    },
    /// Add a terminal or application entry (interactive if no subcommand given)
    Add {
        #[command(subcommand)]
        kind: Option<AddKind>,
    },
    /// Remove terminals/apps (interactive picker, or specify: remove terminal <name>)
    Remove {
        #[command(subcommand)]
        kind: Option<RemoveKind>,
    },
    /// Open .quickdev.toml in $EDITOR (or --global for global config)
    Edit {
        /// Edit global config instead of project config
        #[arg(long)]
        global: bool,
    },
    /// Remove current project from global index
    Deregister {
        /// Also delete the .quickdev.toml file
        #[arg(long)]
        delete: bool,
    },
    /// Get or set global settings (currently: emulator)
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Remove registrations whose path or .quickdev.toml no longer exists
    Prune,
    /// Check the current project's .quickdev.toml for problems
    Validate {
        /// Output the result as JSON (exit status still reports validity)
        #[arg(long)]
        json: bool,
    },
    /// Diagnose global config and registered projects (--fix to repair)
    Doctor {
        /// Create missing config, prune dead registrations, normalize configs
        #[arg(long)]
        fix: bool,
        /// Output the report as JSON (exit status still reports health)
        #[arg(long, conflicts_with = "fix")]
        json: bool,
    },
    /// Capture currently-running apps into this project's .quickdev.toml
    Capture {
        /// Add all detected apps without interactive selection
        #[arg(long)]
        all: bool,
    },
    /// Print the shell setup that enables tab completion
    #[command(after_help = "\
Add one line to your shell's startup file:
  zsh         echo 'source <(quickdev completions zsh)' >> ~/.zshrc
  bash        echo 'source <(quickdev completions bash)' >> ~/.bashrc
  fish        echo 'quickdev completions fish | source' >> ~/.config/fish/config.fish
  powershell  Add-Content $PROFILE 'quickdev completions powershell | Out-String | Invoke-Expression'

Completions call back into quickdev, so project and item names stay current.")]
    Completions {
        /// Shell to set up
        shell: clap_complete::aot::Shell,
    },
}

#[derive(Subcommand)]
pub(crate) enum AddKind {
    /// Add a terminal entry
    #[command(after_help = "\
Examples:
  quickdev add terminal server .                        Open shell in project root
  quickdev add terminal dev . --command \"npm run dev\"   Run a command on open
  quickdev add terminal logs ./logs                     Open shell in subdirectory")]
    Terminal {
        /// Name for this terminal tab
        name: String,
        /// Working directory relative to project root
        path: String,
        /// Startup command to run in the terminal
        #[arg(long)]
        command: Option<String>,
        /// Terminal emulator to use (ghostty, terminal, gnome-terminal, ptyxis, kitty). Omit for auto-detect.
        #[arg(long, add = ArgValueCandidates::new(emulator_candidates))]
        emulator: Option<String>,
    },
    /// Add an application entry
    #[command(after_help = "\
Examples:
  quickdev add app Cursor /Applications/Cursor.app --args \".\"
  quickdev add app Firefox /usr/bin/firefox")]
    App {
        /// Application display name
        name: String,
        /// Executable or .app bundle path
        path: String,
        /// Arguments passed to the application
        #[arg(long, num_args = 1..)]
        args: Option<Vec<String>>,
    },
}

#[derive(Subcommand)]
pub(crate) enum RemoveKind {
    /// Remove a terminal entry by name
    Terminal {
        #[arg(add = ArgValueCandidates::new(terminal_candidates))]
        name: String,
    },
    /// Remove an application entry by name
    App {
        #[arg(add = ArgValueCandidates::new(app_candidates))]
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConfigAction {
    /// Set a global setting, e.g. `config set emulator ghostty`
    Set {
        #[arg(add = ArgValueCandidates::new(setting_candidates))]
        key: String,
        #[arg(add = ArgValueCandidates::new(emulator_candidates))]
        value: String,
    },
    /// Print a global setting, e.g. `config get emulator`
    Get {
        #[arg(add = ArgValueCandidates::new(setting_candidates))]
        key: String,
    },
    /// Clear a global setting, e.g. `config unset emulator`
    Unset {
        #[arg(add = ArgValueCandidates::new(setting_candidates))]
        key: String,
    },
}

// Completion candidates. These run on every TAB press, so they only read —
// never migrate, prompt, or print — and an unreadable config just means no
// suggestions.

fn project_candidates() -> Vec<CompletionCandidate> {
    let Ok(global) = global_config_path_readonly().and_then(|path| load_global_config(&path))
    else {
        return vec![];
    };
    global
        .projects
        .into_iter()
        .map(|p| CompletionCandidate::new(p.name).help(Some(p.path.into())))
        .collect()
}

fn emulator_candidates() -> Vec<CompletionCandidate> {
    SUPPORTED_EMULATORS
        .iter()
        .map(CompletionCandidate::new)
        .collect()
}

fn setting_candidates() -> Vec<CompletionCandidate> {
    vec![CompletionCandidate::new("emulator")]
}

/// Names from the `.quickdev.toml` governing the current directory.
fn current_project_items(
    names: fn(crate::models::ProjectConfig) -> Vec<String>,
) -> Vec<CompletionCandidate> {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| find_project_config(&cwd).ok())
        .and_then(|(path, _root)| load_project_config(&path).ok())
        .map(names)
        .unwrap_or_default()
        .into_iter()
        .map(CompletionCandidate::new)
        .collect()
}

fn terminal_candidates() -> Vec<CompletionCandidate> {
    current_project_items(|cfg| cfg.terminals.into_iter().map(|t| t.name).collect())
}

fn app_candidates() -> Vec<CompletionCandidate> {
    current_project_items(|cfg| cfg.applications.into_iter().map(|a| a.name).collect())
}
