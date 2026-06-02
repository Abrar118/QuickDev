mod adapters;
mod apps;
mod capture;
mod cli;
mod commands;
mod config;
mod doctor;
mod fzf;
// Tab-grouping modules are driven only from macOS-gated code in the binary.
// `tab_strategy` has no other consumer, so it's macOS-only here; the other two
// are needed cross-platform (terminal_app by command handling, ghostty_applescript
// as its dependency) but their tab-building items are exercised only on macOS,
// so suppress dead-code there without masking it on the macOS build.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod ghostty_applescript;
mod launch;
mod models;
mod parse;
#[cfg(target_os = "macos")]
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
