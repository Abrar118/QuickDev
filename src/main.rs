mod adapters;
mod apps;
mod cli;
mod commands;
mod config;
mod fzf;
mod launch;
mod models;
mod parse;

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
        Commands::List => commands::cmd_list(),
        Commands::Add { kind } => commands::cmd_add(kind),
        Commands::Remove { kind } => commands::cmd_remove(kind),
        Commands::Edit { global } => commands::cmd_edit(global),
        Commands::Deregister { delete } => commands::cmd_deregister(delete),
        Commands::Config { action } => commands::cmd_config(action),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        process::exit(1);
    }
}
