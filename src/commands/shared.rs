use crate::models::ProjectConfig;

pub(crate) fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    Ok(input.trim().to_string())
}

pub(crate) fn build_item_display_list(config: &ProjectConfig) -> Vec<String> {
    let mut items = Vec::new();
    for t in &config.terminals {
        let cmd_part = t
            .command
            .as_ref()
            .map(|c| format!(" ({c})"))
            .unwrap_or_default();
        items.push(format!("[terminal] {} — {}{}", t.name, t.path, cmd_part));
    }
    for a in &config.applications {
        items.push(format!("[app] {} — {}", a.name, a.path));
    }
    items
}

pub(crate) fn parse_selected_items(selected: &[String]) -> (Vec<String>, Vec<String>) {
    let mut terminal_names = Vec::new();
    let mut app_names = Vec::new();

    for line in selected {
        if line.starts_with("[terminal] ") {
            let name = line
                .strip_prefix("[terminal] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            terminal_names.push(name);
        } else if line.starts_with("[app] ") {
            let name = line
                .strip_prefix("[app] ")
                .and_then(|s| s.split(" — ").next())
                .unwrap_or("")
                .to_string();
            app_names.push(name);
        }
    }

    (terminal_names, app_names)
}
