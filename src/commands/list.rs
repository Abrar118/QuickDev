use crate::config::ProjectStatus;
use crate::config::{
    global_config_path, load_global_config, load_project_config, missing_statuses,
    project_statuses, projects_json,
};
use crate::fzf::sanitize_row;
use crate::launch::{make_placeholder_ctx, resolve_app_args};
use crate::ui::{self, BOLD, CYAN, DIM, GREEN, RED, YELLOW};
use anstream::adapter::strip_str;
use anstream::println;
use anstyle::Style;
use std::path::{Path, PathBuf};

pub(crate) fn cmd_list(missing: bool, json: bool) -> Result<(), String> {
    let global_path = global_config_path()?;
    let global = load_global_config(&global_path)?;
    let statuses = project_statuses(&global);

    let selected: Vec<_> = if missing {
        missing_statuses(&statuses)
    } else {
        statuses.iter().collect()
    };

    if json {
        let owned: Vec<_> = selected.iter().map(|s| (*s).clone()).collect();
        println!("{}", projects_json(&owned));
        return Ok(());
    }

    if global.projects.is_empty() {
        println!("No projects registered yet.");
        println!(
            "{}",
            ui::paint(
                DIM,
                "Run 'quickdev init' in a project directory to add one."
            )
        );
        return Ok(());
    }

    if missing && selected.is_empty() {
        println!("{}", ui::ok("All registered projects are healthy."));
        return Ok(());
    }

    let title = if missing {
        "Missing projects"
    } else {
        "Projects"
    };
    println!("{}", ui::heading(title, &selected.len().to_string()));

    let blocks: Vec<(bool, Vec<String>)> = selected
        .iter()
        .map(|status| project_block(status, global.emulator.as_deref()))
        .collect();
    let unhealthy = blocks.iter().filter(|(healthy, _)| !healthy).count();
    let width = blocks
        .iter()
        .flat_map(|(_, lines)| lines)
        .map(|line| strip_str(line).to_string().chars().count())
        .max()
        .unwrap_or(0)
        .clamp(40, 100);
    let rule = ui::paint(DIM, "─".repeat(width));

    println!("{rule}");
    for (_, lines) in &blocks {
        for line in lines {
            println!("{line}");
        }
        println!("{rule}");
    }
    println!();

    if unhealthy > 0 {
        println!(
            "{}",
            ui::paint(
                DIM,
                "Run 'quickdev doctor' for details or 'quickdev prune' to drop missing projects."
            )
        );
    }

    Ok(())
}

/// Lines describing one project: a header, then every terminal and app with
/// the details a launch would use. Returns whether the project is healthy.
fn project_block(status: &ProjectStatus, global_emulator: Option<&str>) -> (bool, Vec<String>) {
    let config = match status.issue() {
        Some(issue) => Err(issue),
        None => load_project_config(&PathBuf::from(&status.path).join(".quickdev.toml"))
            .map_err(|_| "config error"),
    };
    let marker = if config.is_ok() {
        ui::paint(GREEN, "●")
    } else {
        ui::paint(RED, "✗")
    };
    let mut header = format!(
        "{marker} {}  {}",
        ui::paint(BOLD, sanitize_row(&status.name)),
        ui::paint(DIM, ui::tilde(&sanitize_row(&status.path)))
    );
    if let Some(then) = status.last_launched {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(then, |d| d.as_secs());
        header.push_str(&format!(
            "  {}",
            ui::paint(DIM, format!("· launched {}", ui::ago(then, now)))
        ));
    }
    let mut lines = vec![header];

    let cfg = match config {
        Err(issue) => {
            lines.push(format!("  {}", ui::paint(RED, issue)));
            return (false, lines);
        }
        Ok(cfg) => cfg,
    };

    if cfg.terminals.is_empty() && cfg.applications.is_empty() {
        lines.push(format!(
            "  {}",
            ui::paint(DIM, "nothing configured — run 'quickdev add'")
        ));
        return (true, lines);
    }

    if !cfg.terminals.is_empty() {
        lines.push(String::new());
        lines.push(section("Terminals", cfg.terminals.len()));
        let rows: Vec<Vec<Cell>> = cfg
            .terminals
            .iter()
            .map(|t| {
                let emulator = match t.emulator.as_deref().or(global_emulator) {
                    Some(e) => cell(CYAN, e),
                    None => cell(DIM, "auto"),
                };
                let command = match t.command.as_deref().map(str::trim) {
                    Some(c) if !c.is_empty() => cell(YELLOW, &format!("$ {c}")),
                    _ => cell(PLAIN, ""),
                };
                vec![
                    cell(BOLD, &t.name),
                    cell(PLAIN, &ui::tilde(&t.path)),
                    emulator,
                    command,
                ]
            })
            .collect();
        lines.extend(table(&rows, "    "));
    }

    if !cfg.applications.is_empty() {
        lines.push(String::new());
        lines.push(section("Apps", cfg.applications.len()));
        // Same substitution launch performs, so `{root}` etc. show real paths.
        let ctx = make_placeholder_ctx(&cfg, Path::new(&status.path));
        let rows: Vec<Vec<Cell>> = cfg
            .applications
            .iter()
            .map(|a| {
                let args = match &a.args {
                    Some(args) if !args.is_empty() => {
                        let shown: Vec<String> = resolve_app_args(args, &ctx)
                            .iter()
                            .map(|arg| ui::tilde(arg))
                            .collect();
                        cell(PLAIN, &format!("→ {}", shown.join(" ")))
                    }
                    _ => cell(PLAIN, ""),
                };
                vec![cell(BOLD, &a.name), args, cell(DIM, &ui::tilde(&a.path))]
            })
            .collect();
        lines.extend(table(&rows, "    "));
    }

    (true, lines)
}

fn section(title: &str, n: usize) -> String {
    format!("  {} {}", ui::paint(CYAN, title), ui::paint(DIM, n))
}

/// A styled table cell. Text is stripped of control characters: it comes from
/// `.quickdev.toml`, and a stray escape sequence would garble the terminal.
type Cell = (Style, String);

const PLAIN: Style = Style::new();

fn cell(style: Style, text: &str) -> Cell {
    (style, sanitize_row(text))
}

/// Render rows with every column aligned. Trailing empty cells are dropped so
/// no row ends in padding.
fn table(rows: &[Vec<Cell>], indent: &str) -> Vec<String> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|c| {
            rows.iter()
                .filter_map(|row| row.get(c))
                .map(|(_, text)| text.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();

    rows.iter()
        .map(|row| {
            let last = row
                .iter()
                .rposition(|(_, text)| !text.is_empty())
                .unwrap_or(0);
            let cells: Vec<String> = row[..=last]
                .iter()
                .enumerate()
                .map(|(c, (style, text))| {
                    if c == last {
                        ui::paint(*style, text)
                    } else {
                        ui::pad(*style, text, widths[c])
                    }
                })
                .collect();
            format!("{indent}{}", cells.join("  "))
        })
        .collect()
}
