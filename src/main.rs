mod adapters;
mod apps;
mod cli;
mod commands;
mod config;
mod doctor;
mod fzf;
mod launch;
mod models;
mod parse;
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
