pub mod adapters;
pub mod apps;
pub mod capture;
pub mod config;
pub mod doctor;
pub mod fzf;
pub mod ghostty_applescript;
pub mod gnome_terminal;
pub mod kitty;
pub mod launch;
pub mod models;
pub mod parse;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub mod session_dir;
pub mod tab_strategy;
pub mod terminal_app;
pub mod validate;
