// Package config loads and saves QuickDev's global index and per-project
// .quickdev.toml files.
package config

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/pelletier/go-toml/v2"

	"github.com/Abrar118/QuickDev/internal/fzf"
	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/tomledit"
	"github.com/Abrar118/QuickDev/internal/ui"
)

// ProjectFileName is the per-project config file.
const ProjectFileName = ".quickdev.toml"

// SupportedEmulators are the values accepted for `emulator`.
var SupportedEmulators = []string{"ghostty", "terminal", "gnome-terminal", "ptyxis", "kitty"}

// IsSupportedEmulator reports whether value is in SupportedEmulators.
func IsSupportedEmulator(value string) bool {
	for _, e := range SupportedEmulators {
		if e == value {
			return true
		}
	}
	return false
}

const globalCommentHeader = `# QuickDev global configuration
#
# emulator = (optional) Default terminal emulator: "ghostty", "terminal", "gnome-terminal", "ptyxis", "kitty"
# terminal_app_tabbing_prompt_declined = internal flag; avoids re-prompting after decline
#
# Projects are auto-managed by quickdev init / deregister
`

const projectCommentHeader = `# QuickDev project configuration
# Edit this file directly or use: quickdev add, quickdev remove
#
# [project]
#   name = Display name for this project
#
# [[terminals]]
#   name    = Label for this terminal tab
#   path    = Working directory relative to project root (e.g., ".", "./src")
#   command  = (optional) Startup command to run when the terminal opens
#   emulator = (optional) Terminal emulator: "ghostty", "terminal", "gnome-terminal", "ptyxis", "kitty". Omit for auto-detect
#
# [[applications]]
#   name = Application display name
#   path = Executable path or .app bundle (e.g., "/Applications/Cursor.app")
#   args = (optional) Arguments list. Placeholders: {root} {config} {name} {cwd}
#          e.g., ["{root}"] opens project root; ["{config}"] opens this file
`

// ---------------------------------------------------------------------------
// Decoding

func decodeGlobal(content string) (*models.GlobalConfig, error) {
	var raw map[string]any
	if err := toml.Unmarshal([]byte(content), &raw); err != nil {
		return nil, err
	}
	if err := requireStrings(raw, "projects", "name", "path"); err != nil {
		return nil, err
	}
	var cfg models.GlobalConfig
	if err := toml.Unmarshal([]byte(content), &cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

func decodeProject(content string) (*models.ProjectConfig, error) {
	var raw map[string]any
	if err := toml.Unmarshal([]byte(content), &raw); err != nil {
		return nil, err
	}
	project, ok := raw["project"].(map[string]any)
	if !ok {
		return nil, errors.New("missing table `project`")
	}
	if _, ok := project["name"].(string); !ok {
		return nil, errors.New("missing field `name` in `project`")
	}
	if err := requireStrings(raw, "terminals", "name", "path"); err != nil {
		return nil, err
	}
	if err := requireStrings(raw, "applications", "name", "path"); err != nil {
		return nil, err
	}
	var cfg models.ProjectConfig
	if err := toml.Unmarshal([]byte(content), &cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}

// requireStrings checks that every table in the array raw[key] has the given
// string fields, since a zero value would otherwise pass silently.
func requireStrings(raw map[string]any, key string, fields ...string) error {
	value, present := raw[key]
	if !present {
		return nil
	}
	list, ok := value.([]any)
	if !ok {
		return fmt.Errorf("`%s` must be an array of tables", key)
	}
	for i, entry := range list {
		table, ok := entry.(map[string]any)
		if !ok {
			return fmt.Errorf("`%s[%d]` must be a table", key, i)
		}
		for _, field := range fields {
			if _, ok := table[field].(string); !ok {
				return fmt.Errorf("missing string field `%s` in `%s[%d]`", field, key, i)
			}
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Encoding (in place)

func namedTable(tables []*tomledit.Section, name string) *tomledit.Section {
	for _, t := range tables {
		if n, ok := t.GetString("name"); ok && n == name {
			return t.Clone()
		}
	}
	return nil
}

func setOptional(s *tomledit.Section, key string, v *string) {
	if v == nil {
		s.Remove(key)
	} else {
		s.Set(key, *v)
	}
}

func applyProjectConfig(doc *tomledit.Doc, cfg *models.ProjectConfig) {
	doc.Table("project").Set("name", cfg.Project.Name)

	previous := doc.ArrayTables("terminals")
	var terminals []*tomledit.Section
	for _, t := range cfg.Terminals {
		table := namedTable(previous, t.Name)
		if table == nil {
			table = tomledit.NewArrayTable("terminals")
		}
		table.Set("name", t.Name)
		table.Set("path", t.Path)
		setOptional(table, "command", t.Command)
		setOptional(table, "emulator", t.Emulator)
		terminals = append(terminals, table)
	}
	doc.ReplaceArray("terminals", terminals)

	previous = doc.ArrayTables("applications")
	var applications []*tomledit.Section
	for _, a := range cfg.Applications {
		table := namedTable(previous, a.Name)
		if table == nil {
			table = tomledit.NewArrayTable("applications")
		}
		table.Set("name", a.Name)
		table.Set("path", a.Path)
		if len(a.Args) > 0 {
			table.Set("args", a.Args)
		} else {
			table.Remove("args")
		}
		applications = append(applications, table)
	}
	doc.ReplaceArray("applications", applications)
}

func applyGlobalConfig(doc *tomledit.Doc, cfg *models.GlobalConfig) {
	if cfg.Emulator == nil {
		doc.RemoveRoot("emulator")
	} else {
		doc.SetRoot("emulator", *cfg.Emulator)
	}
	doc.SetRoot("terminal_app_tabbing_prompt_declined", cfg.TerminalAppTabbingPromptDeclined)

	previous := doc.ArrayTables("projects")
	var projects []*tomledit.Section
	for _, p := range cfg.Projects {
		table := namedTable(previous, p.Name)
		if table == nil {
			table = tomledit.NewArrayTable("projects")
		}
		table.Set("name", p.Name)
		table.Set("path", p.Path)
		if p.LastLaunched != nil {
			table.Set("last_launched", *p.LastLaunched)
		} else {
			table.Remove("last_launched")
		}
		projects = append(projects, table)
	}
	doc.ReplaceArray("projects", projects)
}

// render produces the new file contents. An existing file is edited in place
// so its comments, formatting, and keys a newer QuickDev wrote survive; a file
// that does not parse is an error rather than a silent regeneration, which
// would destroy whatever the user was editing.
func render(path string, existing *string, header string, tables, arrays []string, apply func(*tomledit.Doc)) (string, error) {
	if existing == nil {
		doc := tomledit.Parse("")
		apply(doc)
		return header + "\n" + doc.String(), nil
	}
	var probe map[string]any
	if err := toml.Unmarshal([]byte(*existing), &probe); err != nil {
		return "", fmt.Errorf("failed to parse %s: %w", path, err)
	}
	doc := tomledit.Parse(*existing)
	if !doc.Supports(tables, arrays) {
		// A layout the editor does not model (dotted keys, inline tables):
		// regenerate the data, keeping the file's leading comments.
		fresh := tomledit.Parse("")
		apply(fresh)
		lead := tomledit.LeadingComments(*existing)
		if lead == "" {
			return fresh.String(), nil
		}
		return lead + "\n" + fresh.String(), nil
	}
	apply(doc)
	return doc.String(), nil
}

// ---------------------------------------------------------------------------
// Global config location

type location struct {
	override bool
	path     string
	legacy   string
}

func globalConfigLocations() (location, error) {
	if p := os.Getenv("QUICKDEV_CONFIG"); p != "" {
		return location{override: true, path: p}, nil
	}
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return location{}, errors.New("could not determine home directory")
	}
	var base string
	if runtime.GOOS == "windows" {
		// LOCALAPPDATA rather than the roaming APPDATA: the index holds
		// absolute, machine-specific paths.
		base = os.Getenv("LOCALAPPDATA")
		if base == "" {
			return location{}, errors.New("could not determine %LOCALAPPDATA%")
		}
	} else {
		// ~/.config on macOS too, like most developer CLIs. XDG says to ignore
		// a relative XDG_CONFIG_HOME.
		base = os.Getenv("XDG_CONFIG_HOME")
		if !filepath.IsAbs(base) {
			base = filepath.Join(home, ".config")
		}
	}
	return location{
		path:   filepath.Join(base, "quickdev", "config.toml"),
		legacy: filepath.Join(home, "Documents", "quickdev", "config.toml"),
	}, nil
}

func exists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

// GlobalConfigPath is where the global index lives: $QUICKDEV_CONFIG when set,
// otherwise $XDG_CONFIG_HOME/quickdev/config.toml (default ~/.config) on macOS
// and Linux, %LOCALAPPDATA%\quickdev\config.toml on Windows.
//
// Releases before 0.5 kept it in ~/Documents/quickdev/. The first call that
// finds only that file moves it. If the move fails the old file keeps being
// used, so an upgrade can never strand a user's project index.
func GlobalConfigPath() (string, error) {
	loc, err := globalConfigLocations()
	if err != nil {
		return "", err
	}
	if loc.override || exists(loc.path) || !exists(loc.legacy) {
		return loc.path, nil
	}
	if err := MigrateGlobalConfig(loc.legacy, loc.path); err != nil {
		ui.Eprintln(ui.Warn(fmt.Sprintf("could not move global config to %s: %v", loc.path, err)))
		return loc.legacy, nil
	}
	ui.Eprintln(ui.OK(fmt.Sprintf("Moved global config to %s (was %s)", ui.Tilde(loc.path), ui.Tilde(loc.legacy))))
	return loc.path, nil
}

// GlobalConfigPathReadonly is GlobalConfigPath without the migration, for
// shell completion, which must neither write files nor print.
func GlobalConfigPathReadonly() (string, error) {
	loc, err := globalConfigLocations()
	if err != nil {
		return "", err
	}
	if !loc.override && !exists(loc.path) && exists(loc.legacy) {
		return loc.legacy, nil
	}
	return loc.path, nil
}

// MigrateGlobalConfig moves the global index from legacy to path, leaving the
// old file renamed to config.toml.migrated so it is clearly no longer live.
//
// Holds the legacy file's lock throughout, so a write already in progress
// finishes before the copy is taken. Two processes migrating at once are fine:
// the second finds path already in place and does nothing.
func MigrateGlobalConfig(legacy, path string) error {
	_, err := withConfigLock(legacy, func() (struct{}, error) {
		if exists(path) {
			return struct{}{}, nil
		}
		content, err := os.ReadFile(legacy)
		if err != nil {
			return struct{}{}, fmt.Errorf("failed to read %s: %w", legacy, err)
		}
		dir := filepath.Dir(path)
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return struct{}{}, fmt.Errorf("failed to create %s: %w", dir, err)
		}
		staged, err := os.CreateTemp(dir, ".quickdev-migrate-")
		if err != nil {
			return struct{}{}, fmt.Errorf("failed to stage %s: %w", path, err)
		}
		tmp := staged.Name()
		defer os.Remove(tmp)
		_, err = staged.Write(content)
		if err == nil {
			err = staged.Sync()
		}
		if runtime.GOOS != "windows" {
			if info, statErr := os.Stat(legacy); statErr == nil {
				_ = staged.Chmod(info.Mode().Perm())
			}
		}
		if closeErr := staged.Close(); err == nil {
			err = closeErr
		}
		if err != nil {
			return struct{}{}, fmt.Errorf("failed to write %s: %w", path, err)
		}
		// No-clobber: a hard link fails if a config appeared at the new path
		// since the check above, and that one wins over the copy.
		if err := os.Link(tmp, path); err != nil {
			if errors.Is(err, os.ErrExist) {
				return struct{}{}, nil
			}
			return struct{}{}, fmt.Errorf("failed to create %s: %w", path, err)
		}

		// Best effort: the new file already takes precedence. Use the first
		// unused name so an earlier migration's backup is never replaced.
		for n := 1; ; n++ {
			retired := legacy + ".migrated"
			if n > 1 {
				retired += "." + strconv.Itoa(n)
			}
			if _, err := os.Lstat(retired); err != nil {
				_ = os.Rename(legacy, retired)
				break
			}
		}
		return struct{}{}, nil
	})
	return err
}

// ---------------------------------------------------------------------------
// Load / save

// LoadGlobalConfig reads the global index; a missing file is an empty index.
func LoadGlobalConfig(path string) (*models.GlobalConfig, error) {
	content, err := readOptional(path)
	if err != nil {
		return nil, err
	}
	remember(path, content)
	if content == nil {
		return &models.GlobalConfig{}, nil
	}
	cfg, err := decodeGlobal(*content)
	if err != nil {
		return nil, fmt.Errorf("failed to parse global config: %w", err)
	}
	return cfg, nil
}

// SaveGlobalConfig writes the global index, editing the existing file in place.
func SaveGlobalConfig(path string, cfg *models.GlobalConfig) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}
	_, err := withConfigLock(path, func() (struct{}, error) {
		current, err := readOptional(path)
		if err != nil {
			return struct{}{}, err
		}
		if err := ensureUnchanged(path, current); err != nil {
			return struct{}{}, err
		}
		content, err := render(path, current, globalCommentHeader, nil, []string{"projects"},
			func(d *tomledit.Doc) { applyGlobalConfig(d, cfg) })
		if err != nil {
			return struct{}{}, err
		}
		if err := atomicWrite(path, content); err != nil {
			return struct{}{}, err
		}
		remember(path, &content)
		return struct{}{}, nil
	})
	return err
}

// LoadProjectConfig reads a .quickdev.toml.
func LoadProjectConfig(path string) (*models.ProjectConfig, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read project config: %w", err)
	}
	content := string(data)
	remember(path, &content)
	cfg, err := decodeProject(content)
	if err != nil {
		return nil, fmt.Errorf("failed to parse project config: %w", err)
	}
	return cfg, nil
}

// SaveProjectConfig writes a .quickdev.toml, editing the existing file in place.
func SaveProjectConfig(path string, cfg *models.ProjectConfig) error {
	_, err := withConfigLock(path, func() (struct{}, error) {
		current, err := readOptional(path)
		if err != nil {
			return struct{}{}, err
		}
		if err := ensureUnchanged(path, current); err != nil {
			return struct{}{}, err
		}
		content, err := render(path, current, projectCommentHeader,
			[]string{"project"}, []string{"terminals", "applications"},
			func(d *tomledit.Doc) { applyProjectConfig(d, cfg) })
		if err != nil {
			return struct{}{}, err
		}
		if err := atomicWrite(path, content); err != nil {
			return struct{}{}, err
		}
		remember(path, &content)
		return struct{}{}, nil
	})
	return err
}

// ---------------------------------------------------------------------------
// Project lookup

// FindProjectConfig walks up from start to the nearest .quickdev.toml and
// returns its path and directory.
func FindProjectConfig(start string) (configPath, root string, err error) {
	current := start
	for {
		candidate := filepath.Join(current, ProjectFileName)
		if exists(candidate) {
			return candidate, current, nil
		}
		parent := filepath.Dir(current)
		if parent == current {
			return "", "", errors.New("no .quickdev.toml found in current or parent directories")
		}
		current = parent
	}
}

// ResolveProjectConfig is FindProjectConfig, falling back to an fzf picker over
// registered projects.
func ResolveProjectConfig(start string) (string, string, error) {
	if configPath, root, err := FindProjectConfig(start); err == nil {
		return configPath, root, nil
	}
	return fzfSelectProject()
}

func fzfSelectProject() (string, string, error) {
	globalPath, err := GlobalConfigPath()
	if err != nil {
		return "", "", err
	}
	global, err := LoadGlobalConfig(globalPath)
	if err != nil {
		return "", "", err
	}
	if len(global.Projects) == 0 {
		return "", "", errors.New("No projects registered. Run 'quickdev init' in a project directory.")
	}
	if !fzf.CheckFzf() {
		return "", "", errors.New("no .quickdev.toml found in current or parent directories.\nTip: install fzf for interactive project selection")
	}

	projects := ProjectsByRecency(global)
	items := make([]string, len(projects))
	for i, p := range projects {
		items[i] = p.Name + "    " + p.Path
	}
	// Indexed picker: the row's position identifies the project, so a name or
	// path containing the visible separator cannot break the round-trip.
	index, err := fzf.SelectOneIndexed(items, "Select a project:")
	if err != nil {
		return "", "", err
	}
	root := projects[index].Path
	configPath := filepath.Join(root, ProjectFileName)
	if !exists(configPath) {
		return "", "", fmt.Errorf(".quickdev.toml not found at %s", configPath)
	}
	return configPath, root, nil
}

// UniqueProjectName returns base, or base-2, base-3… if it is taken.
func UniqueProjectName(base string, cfg *models.GlobalConfig) string {
	taken := map[string]bool{}
	for _, p := range cfg.Projects {
		taken[p.Name] = true
	}
	if !taken[base] {
		return base
	}
	for suffix := 2; ; suffix++ {
		candidate := fmt.Sprintf("%s-%d", base, suffix)
		if !taken[candidate] {
			return candidate
		}
	}
}

// RegisterExistingProjectConfig adds an existing .quickdev.toml to the index,
// renaming the project (in its file too) if the name is taken.
func RegisterExistingProjectConfig(configPath, projectPath string, global *models.GlobalConfig) (string, error) {
	existing, err := LoadProjectConfig(configPath)
	if err != nil {
		return "", err
	}
	name := UniqueProjectName(existing.Project.Name, global)
	if existing.Project.Name != name {
		existing.Project.Name = name
		if err := SaveProjectConfig(configPath, existing); err != nil {
			return "", err
		}
	}
	global.Projects = append(global.Projects, models.GlobalProjectEntry{Name: name, Path: projectPath})
	return name, nil
}

// ---------------------------------------------------------------------------
// Settings

func unknownKey(key string) error {
	return fmt.Errorf("unknown config key %q (supported: emulator)", key)
}

// SetGlobalSetting sets a `quickdev config` key and returns a confirmation.
func SetGlobalSetting(cfg *models.GlobalConfig, key, value string) (string, error) {
	if key != "emulator" {
		return "", unknownKey(key)
	}
	if !IsSupportedEmulator(value) {
		return "", fmt.Errorf("unsupported emulator %q (supported: %s)", value, strings.Join(SupportedEmulators, ", "))
	}
	cfg.Emulator = models.Str(value)
	return "Set emulator = " + value, nil
}

// GetGlobalSetting describes a `quickdev config` key's value.
func GetGlobalSetting(cfg *models.GlobalConfig, key string) (string, error) {
	if key != "emulator" {
		return "", unknownKey(key)
	}
	if cfg.Emulator == nil {
		return "emulator is not set (auto-detect)", nil
	}
	return "emulator = " + *cfg.Emulator, nil
}

// UnsetGlobalSetting clears a `quickdev config` key.
func UnsetGlobalSetting(cfg *models.GlobalConfig, key string) (string, error) {
	if key != "emulator" {
		return "", unknownKey(key)
	}
	cfg.Emulator = nil
	return "Unset emulator", nil
}

// ---------------------------------------------------------------------------
// Project health

// ProjectStatus is the health of a registered project: its directory and its
// .quickdev.toml must both exist.
type ProjectStatus struct {
	Name         string
	Path         string
	PathExists   bool
	ConfigExists bool
	LastLaunched *int64
}

// IsHealthy reports whether both the directory and the config exist.
func (s ProjectStatus) IsHealthy() bool { return s.PathExists && s.ConfigExists }

// Issue describes what is wrong, or "" when healthy.
func (s ProjectStatus) Issue() string {
	switch {
	case !s.PathExists:
		return "path missing"
	case !s.ConfigExists:
		return ".quickdev.toml missing"
	}
	return ""
}

// StatusOf checks one registered project.
func StatusOf(entry models.GlobalProjectEntry) ProjectStatus {
	return ProjectStatus{
		Name:         entry.Name,
		Path:         entry.Path,
		PathExists:   exists(entry.Path),
		ConfigExists: exists(filepath.Join(entry.Path, ProjectFileName)),
		LastLaunched: entry.LastLaunched,
	}
}

// ProjectStatuses checks every registered project, most recently launched first.
func ProjectStatuses(global *models.GlobalConfig) []ProjectStatus {
	projects := ProjectsByRecency(global)
	out := make([]ProjectStatus, len(projects))
	for i, p := range projects {
		out[i] = StatusOf(p)
	}
	return out
}

// ProjectsByRecency returns registered projects, most recently launched first;
// never-launched projects follow in index order.
func ProjectsByRecency(global *models.GlobalConfig) []models.GlobalProjectEntry {
	projects := append([]models.GlobalProjectEntry(nil), global.Projects...)
	sort.SliceStable(projects, func(i, j int) bool {
		a, b := projects[i].LastLaunched, projects[j].LastLaunched
		if a == nil || b == nil {
			return a != nil && b == nil
		}
		return *a > *b
	})
	return projects
}

// RecordLaunch stamps the project registered at root as launched now.
//
// It re-reads the index rather than taking the caller's copy: launch loaded it
// before a picker that can stay open for minutes, and writing that stale copy
// back would trip the changed-on-disk check. An unregistered root is not an
// error; there is nothing to stamp.
func RecordLaunch(globalPath, root string) error {
	global, err := LoadGlobalConfig(globalPath)
	if err != nil {
		return err
	}
	for i := range global.Projects {
		if global.Projects[i].Path == root {
			now := time.Now().Unix()
			global.Projects[i].LastLaunched = &now
			return SaveGlobalConfig(globalPath, global)
		}
	}
	return nil
}

// MissingStatuses is the subset of statuses that are not healthy.
func MissingStatuses(statuses []ProjectStatus) []ProjectStatus {
	var out []ProjectStatus
	for _, s := range statuses {
		if !s.IsHealthy() {
			out = append(out, s)
		}
	}
	return out
}

// PruneProjects removes registrations whose path or .quickdev.toml is missing
// and returns their names in their original order.
func PruneProjects(global *models.GlobalConfig) []string {
	var removed []string
	kept := global.Projects[:0]
	for _, entry := range global.Projects {
		if StatusOf(entry).IsHealthy() {
			kept = append(kept, entry)
		} else {
			removed = append(removed, entry.Name)
		}
	}
	global.Projects = kept
	return removed
}

// JSONString quotes s as a JSON string literal.
func JSONString(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch {
		case r == '"':
			b.WriteString(`\"`)
		case r == '\\':
			b.WriteString(`\\`)
		case r < ' ':
			fmt.Fprintf(&b, `\u%04x`, r)
		default:
			b.WriteRune(r)
		}
	}
	b.WriteByte('"')
	return b.String()
}

// ProjectsJSON serializes statuses as a JSON array for `list --json`.
func ProjectsJSON(statuses []ProjectStatus) string {
	if len(statuses) == 0 {
		return "[]"
	}
	var b strings.Builder
	b.WriteString("[")
	for i, s := range statuses {
		if i > 0 {
			b.WriteString(",")
		}
		last := "null"
		if s.LastLaunched != nil {
			last = strconv.FormatInt(*s.LastLaunched, 10)
		}
		fmt.Fprintf(&b, "\n  {\"name\": %s, \"path\": %s, \"healthy\": %t, \"path_exists\": %t, \"config_exists\": %t, \"last_launched\": %s}",
			JSONString(s.Name), JSONString(s.Path), s.IsHealthy(), s.PathExists, s.ConfigExists, last)
	}
	b.WriteString("\n]")
	return b.String()
}
