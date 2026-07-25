use crate::fzf;
use crate::models::{GlobalConfig, GlobalProjectEntry, ProjectConfig};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{value, Array, ArrayOfTables, DocumentMut, Item, Table};

/// Replace `path`'s contents atomically.
///
/// `fs::write` truncates the destination before writing, so a crash, a full
/// disk, or a permission failure partway through leaves a half-written config
/// that no longer parses. Writing a sibling temp file and renaming it over the
/// destination means readers only ever see the old file or the complete new one.
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    use std::io::Write;

    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = dir.unwrap_or_else(|| Path::new("."));
    // Same directory as the destination: rename is only atomic within a
    // filesystem, and the temp dir may be on a different one.
    let mut file = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("failed to stage config write: {e}"))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to write config: {e}"))?;
    // Carry over the destination's permissions. The staged file is created 0600,
    // so replacing without this would silently re-chmod a config the user may
    // have deliberately opened up. A brand-new config keeps the 0600 default.
    #[cfg(unix)]
    if let Ok(existing) = fs::metadata(path) {
        let _ = file.as_file().set_permissions(existing.permissions());
    }
    file.as_file()
        .sync_all()
        .map_err(|e| format!("failed to flush config: {e}"))?;
    file.persist(path)
        .map_err(|e| format!("failed to replace {}: {e}", path.display()))?;
    Ok(())
}

/// Contents of `path`, or `None` when it does not exist.
fn read_optional(path: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("failed to read {}: {e}", path.display())),
    }
}

/// What this process last saw in each config file — `None` meaning "absent".
///
/// Every mutating command is read–modify–write: load a config, change it in
/// memory, write the whole thing back. Nothing coordinates two concurrent
/// invocations, so without a check the second writer silently discards the
/// first's change (`quickdev add terminal a` and `add terminal b` racing would
/// keep only one). Remembering what we read lets the write refuse when the file
/// no longer matches.
static SEEN: std::sync::Mutex<std::collections::BTreeMap<PathBuf, Option<String>>> =
    std::sync::Mutex::new(std::collections::BTreeMap::new());

fn remember(path: &Path, content: Option<&str>) {
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    seen.insert(path.to_path_buf(), content.map(str::to_string));
}

/// Advisory lock file guarding a config's write section.
///
/// A sibling file rather than the config itself: [`atomic_write`] replaces the
/// config by rename, so a lock taken on the config's inode would no longer refer
/// to the file other processes open. The lock file is created once and left in
/// place; it holds no content.
pub fn lock_path_for(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    path.with_file_name(name)
}

/// Run `write_section` holding an exclusive lock on `path`'s lock file.
///
/// Scoped deliberately tight: re-read, compare, replace. Callers do their
/// interactive work (fzf pickers, prompts) outside it, so one invocation never
/// blocks on another's open picker — only on another's brief save.
///
/// Without this, two writers could both read the same contents, both pass
/// [`ensure_unchanged`], and both rename: the second silently wins. The check
/// and the replace have to be inside the same critical section to close that.
fn with_config_lock<T>(
    path: &Path,
    write_section: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let lock_path = lock_path_for(path);
    let file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("failed to open {}: {e}", lock_path.display()))?;
    let mut lock = fd_lock::RwLock::new(file);
    // Blocking: the holder is only ever inside its own write section, and the
    // kernel releases the lock if that process dies, so this cannot hang on a
    // stale lock the way a hand-rolled lock file would.
    let _guard = lock
        .write()
        .map_err(|e| format!("failed to lock {}: {e}", lock_path.display()))?;
    write_section()
}

/// Remove `config_path` only if `commit` succeeds — either both happen, or
/// neither does.
///
/// The whole sequence runs under the same lock the save path takes. Deleting is
/// a write to the config like any other: without the lock, a concurrent save
/// could read the config, wait while this moves it aside, then write a fresh one
/// — leaving an unregistered `.quickdev.toml` behind after this reported it
/// deleted.
///
/// The config is moved to a uniquely-named staging file beside it first.
/// Uniquely, because `rename` replaces its destination on Unix: a fixed staging
/// name would destroy a recovery copy left behind by an earlier failed cleanup.
/// If `commit` fails the config is moved back, and if that restore also fails
/// the error names the staging file so it can be recovered by hand.
///
/// The lock file itself is deliberately left in place. Unlinking it would let a
/// later invocation create a fresh one while another process still holds the
/// old, now-unnamed inode — two processes each holding "the" lock.
pub fn remove_config_with<T>(
    config_path: &Path,
    commit: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    with_config_lock(config_path, || {
        let parent = config_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let staged = tempfile::Builder::new()
            .prefix(".quickdev-deregister-")
            .tempfile_in(parent)
            .map_err(|e| format!("failed to stage {}: {e}", config_path.display()))?;
        // Take the exclusively-created path; the rename replaces the placeholder.
        // The handle is dropped first so Windows can rename over it.
        let (file, staged) = staged
            .keep()
            .map_err(|e| format!("failed to stage {}: {e}", config_path.display()))?;
        drop(file);

        if let Err(e) = fs::rename(config_path, &staged) {
            let _ = fs::remove_file(&staged);
            return Err(format!(
                "failed to move {} aside: {e}",
                config_path.display()
            ));
        }

        let committed = match commit() {
            Ok(value) => value,
            Err(e) => {
                return Err(match fs::rename(&staged, config_path) {
                    Ok(()) => e,
                    Err(restore) => format!(
                    "{e}\nadditionally, {} could not be put back ({restore}) — recover it from {}",
                    config_path.display(),
                    staged.display()
                ),
                })
            }
        };

        fs::remove_file(&staged).map_err(|e| {
            format!(
                "removed {} from the index, but its staged copy at {} could not be deleted: {e}",
                config_path.display(),
                staged.display()
            )
        })?;
        // Deliberately *not* `remember(config_path, None)`. Recording the
        // deletion would tell the rest of the process the file is legitimately
        // absent, so a save still holding the pre-delete config would look like
        // a create and recreate it. Leaving the pre-delete record in place makes
        // that save fail its change check instead, which is what should happen.
        Ok(committed)
    })
}

/// Refuse to write when the file is not what this process last read.
///
/// Guards against a *stale in-memory model*: a command that loaded the config,
/// spent time in a picker, and is now writing back a version that predates
/// someone else's change. [`with_config_lock`] serializes the write sections
/// themselves; this catches the edit that was already out of date on arrival.
fn ensure_unchanged(path: &Path, current: Option<&str>) -> Result<(), String> {
    let seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    match seen.get(path) {
        Some(recorded) if recorded.as_deref() == current => Ok(()),
        Some(_) => Err(format!(
            "{} changed on disk since it was read (another quickdev command may have run at the same time); re-run this command",
            path.display()
        )),
        // Never read in this process: writing is only safe if we are creating
        // the file. Otherwise we would be clobbering contents we never saw.
        None if current.is_none() => Ok(()),
        None => Err(format!(
            "refusing to overwrite {} without reading it first",
            path.display()
        )),
    }
}

/// Parse `content` for in-place editing.
///
/// A file that exists but does not parse is an error rather than a silent
/// regeneration: overwriting it would destroy whatever the user was editing.
fn parse_document(path: &Path, content: &str) -> Result<DocumentMut, String> {
    content
        .parse::<DocumentMut>()
        .map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

/// Set `key` to `val`, or remove it when `val` is `None`.
fn set_optional_str(table: &mut Table, key: &str, val: Option<&str>) {
    match val {
        Some(v) => table[key] = value(v),
        None => {
            table.remove(key);
        }
    }
}

/// Look up an existing array-of-tables entry by its `name` key so a rewrite can
/// reuse it, carrying along its comments and any keys QuickDev does not model.
fn table_named(tables: &ArrayOfTables, name: &str) -> Option<Table> {
    tables
        .iter()
        .find(|t| t.get("name").and_then(Item::as_str) == Some(name))
        .cloned()
}

fn existing_tables(doc: &DocumentMut, key: &str) -> ArrayOfTables {
    doc.get(key)
        .and_then(Item::as_array_of_tables)
        .cloned()
        .unwrap_or_default()
}

fn put_tables(doc: &mut DocumentMut, key: &str, tables: ArrayOfTables) {
    if tables.is_empty() {
        doc.remove(key);
    } else {
        doc[key] = Item::ArrayOfTables(tables);
    }
}

fn apply_project_config(doc: &mut DocumentMut, config: &ProjectConfig) {
    doc["project"]["name"] = value(config.project.name.as_str());

    let previous = existing_tables(doc, "terminals");
    let mut terminals = ArrayOfTables::new();
    for terminal in &config.terminals {
        let mut table = table_named(&previous, &terminal.name).unwrap_or_default();
        table["name"] = value(terminal.name.as_str());
        table["path"] = value(terminal.path.as_str());
        set_optional_str(&mut table, "command", terminal.command.as_deref());
        set_optional_str(&mut table, "emulator", terminal.emulator.as_deref());
        terminals.push(table);
    }
    put_tables(doc, "terminals", terminals);

    let previous = existing_tables(doc, "applications");
    let mut applications = ArrayOfTables::new();
    for app in &config.applications {
        let mut table = table_named(&previous, &app.name).unwrap_or_default();
        table["name"] = value(app.name.as_str());
        table["path"] = value(app.path.as_str());
        match &app.args {
            Some(args) => table["args"] = value(args.iter().map(String::as_str).collect::<Array>()),
            None => {
                table.remove("args");
            }
        }
        applications.push(table);
    }
    put_tables(doc, "applications", applications);
}

fn apply_global_config(doc: &mut DocumentMut, config: &GlobalConfig) {
    set_optional_str(doc.as_table_mut(), "emulator", config.emulator.as_deref());
    doc["terminal_app_tabbing_prompt_declined"] =
        value(config.terminal_app_tabbing_prompt_declined);

    let previous = existing_tables(doc, "projects");
    let mut projects = ArrayOfTables::new();
    for project in &config.projects {
        let mut table = table_named(&previous, &project.name).unwrap_or_default();
        table["name"] = value(project.name.as_str());
        table["path"] = value(project.path.as_str());
        projects.push(table);
    }
    put_tables(doc, "projects", projects);
}

pub fn global_config_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("could not determine home directory")?;
    Ok(home.join("Documents").join("quickdev").join("config.toml"))
}

pub fn load_global_config(path: &Path) -> Result<GlobalConfig, String> {
    let content = read_optional(path)?;
    remember(path, content.as_deref());
    let Some(content) = content else {
        return Ok(GlobalConfig {
            emulator: None,
            terminal_app_tabbing_prompt_declined: false,
            projects: vec![],
        });
    };
    toml::from_str(&content).map_err(|e| format!("failed to parse global config: {e}"))
}

const GLOBAL_COMMENT_HEADER: &str = "\
# QuickDev global configuration
#
# emulator = (optional) Default terminal emulator: \"ghostty\", \"terminal\", \"gnome-terminal\", \"ptyxis\", \"kitty\"
# terminal_app_tabbing_prompt_declined = internal flag; avoids re-prompting after decline
#
# Projects are auto-managed by quickdev init / deregister
";

pub fn save_global_config(path: &Path, config: &GlobalConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config directory: {e}"))?;
    }
    with_config_lock(path, || {
        let current = read_optional(path)?;
        ensure_unchanged(path, current.as_deref())?;
        let content = match current {
            // Edit the file the user has: their comments, formatting, and any
            // keys a newer QuickDev writes but this build does not model survive.
            Some(existing) => {
                let mut doc = parse_document(path, &existing)?;
                apply_global_config(&mut doc, config);
                doc.to_string()
            }
            None => {
                let serialized = toml::to_string_pretty(config)
                    .map_err(|e| format!("failed to serialize global config: {e}"))?;
                format!("{GLOBAL_COMMENT_HEADER}\n{serialized}")
            }
        };
        atomic_write(path, &content)?;
        remember(path, Some(&content));
        Ok(())
    })
}

pub fn load_project_config(path: &Path) -> Result<ProjectConfig, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("failed to read project config: {e}"))?;
    remember(path, Some(&content));
    toml::from_str(&content).map_err(|e| format!("failed to parse project config: {e}"))
}

const TOML_COMMENT_HEADER: &str = "\
# QuickDev project configuration
# Edit this file directly or use: quickdev add, quickdev remove
#
# [project]
#   name = Display name for this project
#
# [[terminals]]
#   name    = Label for this terminal tab
#   path    = Working directory relative to project root (e.g., \".\", \"./src\")
#   command  = (optional) Startup command to run when the terminal opens
#   emulator = (optional) Terminal emulator: \"ghostty\", \"terminal\", \"gnome-terminal\", \"ptyxis\", \"kitty\". Omit for auto-detect
#
# [[applications]]
#   name = Application display name
#   path = Executable path or .app bundle (e.g., \"/Applications/Cursor.app\")
#   args = (optional) Arguments list. Placeholders: {root} {config} {name} {cwd}
#          e.g., [\"{root}\"] opens project root; [\"{config}\"] opens this file
";

pub fn save_project_config(path: &Path, config: &ProjectConfig) -> Result<(), String> {
    with_config_lock(path, || {
        let current = read_optional(path)?;
        ensure_unchanged(path, current.as_deref())?;
        let content = match current {
            Some(existing) => {
                let mut doc = parse_document(path, &existing)?;
                apply_project_config(&mut doc, config);
                doc.to_string()
            }
            None => {
                let serialized = toml::to_string_pretty(config)
                    .map_err(|e| format!("failed to serialize project config: {e}"))?;
                format!("{TOML_COMMENT_HEADER}\n{serialized}")
            }
        };
        atomic_write(path, &content)?;
        remember(path, Some(&content));
        Ok(())
    })
}

pub fn find_project_config(start: &Path) -> Result<(PathBuf, PathBuf), String> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(".quickdev.toml");
        if candidate.exists() {
            return Ok((candidate, current));
        }
        if !current.pop() {
            return Err("no .quickdev.toml found in current or parent directories".to_string());
        }
    }
}

pub fn unique_project_name(base_name: &str, config: &GlobalConfig) -> String {
    let names: Vec<&str> = config.projects.iter().map(|p| p.name.as_str()).collect();
    if !names.contains(&base_name) {
        return base_name.to_string();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base_name}-{suffix}");
        if !names.contains(&candidate.as_str()) {
            return candidate;
        }
        suffix += 1;
    }
}

pub fn register_existing_project_config(
    config_path: &Path,
    project_path: String,
    global: &mut GlobalConfig,
) -> Result<String, String> {
    let mut existing = load_project_config(config_path)?;
    let project_name = unique_project_name(&existing.project.name, global);
    if existing.project.name != project_name {
        existing.project.name = project_name.clone();
        save_project_config(config_path, &existing)?;
    }
    global.projects.push(GlobalProjectEntry {
        name: project_name.clone(),
        path: project_path,
    });
    Ok(project_name)
}

pub fn resolve_project_config(start: &Path) -> Result<(PathBuf, PathBuf), String> {
    match find_project_config(start) {
        Ok(result) => Ok(result),
        Err(_) => fzf_select_project(),
    }
}

pub const SUPPORTED_EMULATORS: &[&str] =
    &["ghostty", "terminal", "gnome-terminal", "ptyxis", "kitty"];

pub fn is_supported_emulator(value: &str) -> bool {
    SUPPORTED_EMULATORS.contains(&value)
}

fn unknown_key_error(key: &str) -> String {
    format!("unknown config key {key:?} (supported: emulator)")
}

pub fn set_global_setting(
    config: &mut GlobalConfig,
    key: &str,
    value: &str,
) -> Result<String, String> {
    match key {
        "emulator" => {
            if !is_supported_emulator(value) {
                return Err(format!(
                    "unsupported emulator {value:?} (supported: {})",
                    SUPPORTED_EMULATORS.join(", ")
                ));
            }
            config.emulator = Some(value.to_string());
            Ok(format!("Set emulator = {value}"))
        }
        other => Err(unknown_key_error(other)),
    }
}

pub fn get_global_setting(config: &GlobalConfig, key: &str) -> Result<String, String> {
    match key {
        "emulator" => Ok(match &config.emulator {
            Some(v) => format!("emulator = {v}"),
            None => "emulator is not set (auto-detect)".to_string(),
        }),
        other => Err(unknown_key_error(other)),
    }
}

pub fn unset_global_setting(config: &mut GlobalConfig, key: &str) -> Result<String, String> {
    match key {
        "emulator" => {
            config.emulator = None;
            Ok("Unset emulator".to_string())
        }
        other => Err(unknown_key_error(other)),
    }
}

/// Health of a registered project: its directory and its `.quickdev.toml` must both exist.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectStatus {
    pub name: String,
    pub path: String,
    pub path_exists: bool,
    pub config_exists: bool,
}

impl ProjectStatus {
    pub fn is_healthy(&self) -> bool {
        self.path_exists && self.config_exists
    }

    pub fn issue(&self) -> Option<&'static str> {
        if !self.path_exists {
            Some("path missing")
        } else if !self.config_exists {
            Some(".quickdev.toml missing")
        } else {
            None
        }
    }
}

pub fn project_status(entry: &GlobalProjectEntry) -> ProjectStatus {
    let path = Path::new(&entry.path);
    let path_exists = path.exists();
    let config_exists = path.join(".quickdev.toml").exists();
    ProjectStatus {
        name: entry.name.clone(),
        path: entry.path.clone(),
        path_exists,
        config_exists,
    }
}

pub fn project_statuses(global: &GlobalConfig) -> Vec<ProjectStatus> {
    global.projects.iter().map(project_status).collect()
}

/// Subset of statuses that are not healthy (path or config missing).
pub fn missing_statuses(statuses: &[ProjectStatus]) -> Vec<&ProjectStatus> {
    statuses.iter().filter(|s| !s.is_healthy()).collect()
}

/// Removes registrations whose path or `.quickdev.toml` is missing.
/// Returns the names of removed projects, in their original order.
pub fn prune_projects(global: &mut GlobalConfig) -> Vec<String> {
    let mut removed = Vec::new();
    global.projects.retain(|entry| {
        if project_status(entry).is_healthy() {
            true
        } else {
            removed.push(entry.name.clone());
            false
        }
    });
    removed
}

/// Serialize project statuses to a JSON array string for `list --json`.
pub fn projects_json(statuses: &[ProjectStatus]) -> String {
    fn esc(s: &str) -> String {
        use std::fmt::Write as _;

        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                c if c < ' ' => {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                c => out.push(c),
            }
        }
        out
    }

    if statuses.is_empty() {
        return "[]".to_string();
    }

    let mut out = String::from("[");
    for (i, s) in statuses.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "\n  {{\"name\": \"{}\", \"path\": \"{}\", \"healthy\": {}, \"path_exists\": {}, \"config_exists\": {}}}",
            esc(&s.name),
            esc(&s.path),
            s.is_healthy(),
            s.path_exists,
            s.config_exists
        ));
    }
    out.push_str("\n]");
    out
}

fn fzf_select_project() -> Result<(PathBuf, PathBuf), String> {
    let global_path = global_config_path()?;
    let global = load_global_config(&global_path)?;

    if global.projects.is_empty() {
        return Err(
            "No projects registered. Run 'quickdev init' in a project directory.".to_string(),
        );
    }

    if !fzf::check_fzf() {
        return Err(
            "no .quickdev.toml found in current or parent directories.\nTip: install fzf for interactive project selection"
                .to_string(),
        );
    }

    let items: Vec<String> = global
        .projects
        .iter()
        .map(|p| format!("{}    {}", p.name, p.path))
        .collect();

    // Indexed picker: the row's position identifies the project, so a name or
    // path containing the visible separator cannot break the round-trip.
    let index = fzf::fzf_select_one_indexed(&items, "Select a project:")?;
    let entry = global.projects.get(index).ok_or("invalid selection")?;

    let root = PathBuf::from(&entry.path);
    let config_path = root.join(".quickdev.toml");

    if !config_path.exists() {
        return Err(format!(
            ".quickdev.toml not found at {}",
            config_path.display()
        ));
    }

    Ok((config_path, root))
}
