use crate::launch::escape_applescript_string;

#[derive(Debug, Clone, Copy)]
pub struct ResolvedTerminal<'a> {
    pub cwd: &'a str,
    pub command: Option<&'a str>,
}

pub fn build_script(terminals: &[ResolvedTerminal<'_>]) -> Result<String, String> {
    if terminals.is_empty() {
        return Err("no terminals to launch".to_string());
    }

    let mut script = String::from("tell application \"Ghostty\"\n");
    for (index, terminal) in terminals.iter().enumerate() {
        let config_name = format!("c{index}");
        script.push_str(&format!(
            "  set {config_name} to {{{}}}\n",
            surface_configuration(terminal)
        ));
        if index == 0 {
            script.push_str(&format!(
                "  set w to new window with configuration {config_name}\n"
            ));
        } else {
            script.push_str(&format!(
                "  new tab in w with configuration {config_name}\n"
            ));
        }
    }
    script.push_str("end tell");
    Ok(script)
}

fn surface_configuration(terminal: &ResolvedTerminal<'_>) -> String {
    let cwd = escape_applescript_string(terminal.cwd);
    let mut fields = vec![format!("initial working directory:\"{cwd}\"")];
    if let Some(command) = terminal.command {
        fields.push(format!(
            "command:\"{}\"",
            escape_applescript_string(command)
        ));
    }
    fields.push("wait after command:true".to_string());
    fields.join(", ")
}
