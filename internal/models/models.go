// Package models holds the data shapes stored in QuickDev's TOML files.
package models

// GlobalConfig is the machine-wide index of registered projects.
type GlobalConfig struct {
	Emulator                         *string              `toml:"emulator,omitempty"`
	TerminalAppTabbingPromptDeclined bool                 `toml:"terminal_app_tabbing_prompt_declined"`
	Projects                         []GlobalProjectEntry `toml:"projects,omitempty"`
}

// GlobalProjectEntry is one registered project.
type GlobalProjectEntry struct {
	Name string `toml:"name"`
	Path string `toml:"path"`
	// LastLaunched is the Unix time (seconds) of the last successful
	// `quickdev launch`; it orders `list` and the project picker.
	LastLaunched *int64 `toml:"last_launched,omitempty"`
}

// ProjectConfig is the contents of a project's .quickdev.toml.
type ProjectConfig struct {
	Project      ProjectEntry    `toml:"project"`
	Terminals    []TerminalEntry `toml:"terminals,omitempty"`
	Applications []AppEntry      `toml:"applications,omitempty"`
}

// ProjectEntry is the [project] table.
type ProjectEntry struct {
	Name string `toml:"name"`
}

// TerminalEntry is one [[terminals]] table.
type TerminalEntry struct {
	Name     string  `toml:"name"`
	Path     string  `toml:"path"`
	Command  *string `toml:"command,omitempty"`
	Emulator *string `toml:"emulator,omitempty"`
}

// AppEntry is one [[applications]] table. Empty Args means "not set": every
// consumer treats an empty list and a missing one the same way.
type AppEntry struct {
	Name string   `toml:"name"`
	Path string   `toml:"path"`
	Args []string `toml:"args,omitempty"`
}

// Str returns a pointer to s, for optional fields.
func Str(s string) *string { return &s }

// Deref returns *p, or "" when p is nil.
func Deref(p *string) string {
	if p == nil {
		return ""
	}
	return *p
}
