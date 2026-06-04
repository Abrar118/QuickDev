mod adapters;
mod apps;
mod capture;
mod cli;
mod commands;
mod config;
mod doctor;
mod fzf;
// Tab-grouping modules used from platform-gated code in the binary.
// `tab_strategy` is compiled on macOS and Linux (Linux needs it for gnome-terminal
// tab dispatch); the other two (ghostty_applescript, terminal_app) remain macOS-only
// consumers but are declared unconditionally — suppress dead-code on non-macOS
// without masking it on the macOS build.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod ghostty_applescript;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod gnome_terminal;
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod kitty;
mod launch;
mod models;
mod parse;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod tab_strategy;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod terminal_app;
mod validate;

use clap::Parser;
use cli::{Cli, Commands};
use std::process;

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { from } => commands::cmd_init(from),
        Commands::Launch {
            project,
            all,
            dry_run,
        } => commands::cmd_launch(project, all, dry_run),
        Commands::List { missing, json } => commands::cmd_list(missing, json),
        Commands::Add { kind } => commands::cmd_add(kind),
        Commands::Remove { kind } => commands::cmd_remove(kind),
        Commands::Edit { global } => commands::cmd_edit(global),
        Commands::Deregister { delete } => commands::cmd_deregister(delete),
        Commands::Config { action } => commands::cmd_config(action),
        Commands::Prune => commands::cmd_prune(),
        Commands::Validate => commands::cmd_validate(),
        Commands::Doctor { fix } => commands::cmd_doctor(fix),
        Commands::Capture { all } => commands::cmd_capture(all),
    };

    if let Err(e) = result {
        if fzf::is_cancellation(&e) {
            println!("Cancelled.");
            process::exit(0);
        }
        eprintln!("error: {e}");
        process::exit(1);
    }
}
