use crate::config::{global_config_path, resolve_project_config};
use crate::parse;

pub(crate) fn cmd_edit(global: bool) -> Result<(), String> {
    let config_path = if global {
        global_config_path()
    } else {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let (path, _root) = resolve_project_config(&cwd)?;
        path
    };

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());

    let parts = parse::parse_shell_args(&editor)?;
    let (program, leading) = parts.split_first().ok_or("editor command is empty")?;

    std::process::Command::new(program)
        .args(leading)
        .arg(&config_path)
        .status()
        .map_err(|e| format!("failed to open editor '{}': {}", editor, e))?;

    Ok(())
}
