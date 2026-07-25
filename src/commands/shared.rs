use crate::fzf::sanitize_row;
use crate::models::ProjectConfig;

pub(crate) fn prompt(message: &str) -> Result<String, String> {
    eprint!("{message}");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    Ok(input.trim().to_string())
}

/// Picker rows for every terminal then every application, in config order.
///
/// The row's position is what identifies it — see [`selected_items`]. Terminals
/// occupy positions `0..terminals.len()`, applications follow.
pub(crate) fn build_item_display_list(config: &ProjectConfig) -> Vec<String> {
    let mut items = Vec::new();
    for t in &config.terminals {
        let cmd_part = t
            .command
            .as_ref()
            .map(|c| format!(" ({})", sanitize_row(c)))
            .unwrap_or_default();
        items.push(format!(
            "[terminal] {} — {}{}",
            sanitize_row(&t.name),
            sanitize_row(&t.path),
            cmd_part
        ));
    }
    for a in &config.applications {
        items.push(format!(
            "[app] {} — {}",
            sanitize_row(&a.name),
            sanitize_row(&a.path)
        ));
    }
    items
}

/// Split picker positions back into terminal and application indices.
///
/// Positions come from fzf's hidden index column rather than the visible text:
/// a name containing the display separator (` — `) never round-tripped through
/// string splitting, so such an item could not be launched or removed.
pub(crate) fn selected_items(
    config: &ProjectConfig,
    positions: &[usize],
) -> (Vec<usize>, Vec<usize>) {
    let boundary = config.terminals.len();
    let mut terminals = Vec::new();
    let mut apps = Vec::new();
    for &position in positions {
        if position < boundary {
            terminals.push(position);
        } else {
            apps.push(position - boundary);
        }
    }
    (terminals, apps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppEntry, ProjectEntry, TerminalEntry};

    fn config() -> ProjectConfig {
        ProjectConfig {
            project: ProjectEntry {
                name: "p".to_string(),
            },
            terminals: vec![
                TerminalEntry {
                    name: "api".to_string(),
                    path: ".".to_string(),
                    command: None,
                    emulator: None,
                },
                TerminalEntry {
                    name: "web".to_string(),
                    path: "./web".to_string(),
                    command: None,
                    emulator: None,
                },
            ],
            applications: vec![AppEntry {
                name: "Cursor".to_string(),
                path: "/Applications/Cursor.app".to_string(),
                args: None,
            }],
        }
    }

    #[test]
    fn positions_split_at_the_terminal_application_boundary() {
        let config = config();
        assert_eq!(selected_items(&config, &[0, 2]), (vec![0], vec![0]));
        assert_eq!(selected_items(&config, &[1]), (vec![1], vec![]));
        assert_eq!(selected_items(&config, &[]), (vec![], vec![]));
    }

    #[test]
    fn names_containing_the_display_separator_still_round_trip() {
        // " — " is the visible separator; the old parser split on it and lost
        // everything after the first occurrence in a name.
        let mut config = config();
        config.terminals[0].name = "api — v2".to_string();

        let items = build_item_display_list(&config);
        assert!(items[0].starts_with("[terminal] api — v2 — ."));
        // Selection is by position, so the name's content is irrelevant.
        assert_eq!(selected_items(&config, &[0]), (vec![0], vec![]));
    }

    #[test]
    fn display_rows_never_contain_control_characters() {
        let mut config = config();
        config.terminals[0].name = "api\nlaunch evil".to_string();

        let items = build_item_display_list(&config);
        assert_eq!(items.len(), 3, "one row per configured item");
        assert!(!items[0].contains('\n'));
        assert!(items[0].contains("apilaunch evil"));
    }
}
