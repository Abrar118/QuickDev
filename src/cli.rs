use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "quickdev",
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
  quickdev deregister                                   Unregister project"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Create .quickdev.toml in the current directory and register the project
    Init {
        /// Clone config from another project by name
        #[arg(long)]
        from: Option<String>,
    },
    /// Launch terminals and applications for a project
    Launch {
        /// Project to launch from the global index (omit to use current directory); the picker still appears unless --all
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
    Validate,
    /// Diagnose global config and registered projects (--fix to repair)
    Doctor {
        /// Create missing config, prune dead registrations, normalize configs
        #[arg(long)]
        fix: bool,
    },
    /// Capture currently-running apps into this project's .quickdev.toml
    Capture {
        /// Add all detected apps without interactive selection
        #[arg(long)]
        all: bool,
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
        /// Terminal emulator to use (ghostty, terminal, gnome-terminal, ptyxis). Omit for auto-detect.
        #[arg(long)]
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
    Terminal { name: String },
    /// Remove an application entry by name
    App { name: String },
}

#[derive(Subcommand)]
pub(crate) enum ConfigAction {
    /// Set a global setting, e.g. `config set emulator ghostty`
    Set { key: String, value: String },
    /// Print a global setting, e.g. `config get emulator`
    Get { key: String },
    /// Clear a global setting, e.g. `config unset emulator`
    Unset { key: String },
}
