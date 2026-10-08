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
#[cfg_attr(not(any(target_os = "macos", target_os = "linux")), allow(dead_code))]
mod kitty;
mod launch;
mod models;
mod parse;
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod session_dir;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[cfg(any(target_os = "macos", target_os = "linux"))]
mod tab_strategy;
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod terminal_app;
mod ui;
mod validate;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use cli::{Cli, Commands};
use std::process;

fn main() {
    // Answers shell TAB requests (`COMPLETE=<shell> quickdev …`) and exits;
    // a normal run falls straight through. Must run before anything is printed.
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();
    // Every styled print goes through anstream, which consults this global.
    anstream::ColorChoice::write_global(match cli.color {
        clap::ColorChoice::Always => anstream::ColorChoice::Always,
        clap::ColorChoice::Never => anstream::ColorChoice::Never,
        clap::ColorChoice::Auto => anstream::ColorChoice::Auto,
    });

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
        Commands::Validate { json } => commands::cmd_validate(json),
        Commands::Doctor { fix, json } => commands::cmd_doctor(fix, json),
        Commands::Capture { all } => commands::cmd_capture(all),
        Commands::Completions { shell } => commands::cmd_completions(shell),
    };

    if let Err(e) = result {
        if fzf::is_cancellation(&e) {
            anstream::println!("{}", ui::paint(ui::DIM, "Cancelled."));
            process::exit(0);
        }
        anstream::eprintln!("{} {e}", ui::paint(ui::RED.bold(), "error:"));
        process::exit(1);
    }
}
