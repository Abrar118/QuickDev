// Package validate checks project configs and the entries `add` writes.
package validate

import (
	"errors"
	"fmt"
	"strings"
	"unicode"

	"github.com/Abrar118/QuickDev/internal/adapters"
	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/models"
)

// validateName rejects a display name QuickDev cannot round-trip: an empty
// name matches nothing, and a control character would forge picker rows.
func validateName(kind, name string) error {
	if strings.TrimSpace(name) == "" {
		return fmt.Errorf("%s name cannot be empty", kind)
	}
	for _, r := range name {
		if unicode.IsControl(r) {
			return fmt.Errorf("%s name cannot contain control characters or newlines", kind)
		}
	}
	return nil
}

// TerminalEntry validates a terminal before it is written, so `add` cannot
// persist an entry that `validate` or `launch` would reject afterwards.
func TerminalEntry(entry models.TerminalEntry, projectRoot string) error {
	if err := validateName("terminal", entry.Name); err != nil {
		return err
	}
	if strings.TrimSpace(entry.Path) == "" {
		return errors.New("terminal path cannot be empty")
	}
	if _, err := adapters.ResolveTerminalPath(projectRoot, entry.Path); err != nil {
		return fmt.Errorf("invalid path '%s': %v", entry.Path, err)
	}
	if entry.Emulator != nil && !config.IsSupportedEmulator(*entry.Emulator) {
		return fmt.Errorf("unsupported emulator '%s' (supported: %s)",
			*entry.Emulator, strings.Join(config.SupportedEmulators, ", "))
	}
	return nil
}

// AppEntry validates an application before it is written.
func AppEntry(entry models.AppEntry) error {
	if err := validateName("application", entry.Name); err != nil {
		return err
	}
	if strings.TrimSpace(entry.Path) == "" {
		return errors.New("application path cannot be empty")
	}
	return nil
}

// Report holds hard errors and softer warnings.
type Report struct {
	Errors   []string
	Warnings []string
}

// OK reports whether there are no errors.
func (r Report) OK() bool { return len(r.Errors) == 0 }

// JSON renders the report for `validate --json`. configPath is nil when no
// config was found.
func (r Report) JSON(configPath *string) string {
	list := func(items []string) string {
		quoted := make([]string, len(items))
		for i, item := range items {
			quoted[i] = config.JSONString(item)
		}
		return "[" + strings.Join(quoted, ", ") + "]"
	}
	path := "null"
	if configPath != nil {
		path = config.JSONString(*configPath)
	}
	return fmt.Sprintf("{\n  \"path\": %s,\n  \"valid\": %t,\n  \"errors\": %s,\n  \"warnings\": %s\n}",
		path, r.OK(), list(r.Errors), list(r.Warnings))
}

// ProjectConfig checks a whole config with the same rules `add` enforces, so a
// hand-edited file cannot hold an entry the CLI would have refused to create.
func ProjectConfig(cfg *models.ProjectConfig, projectRoot string) Report {
	var r Report
	if strings.TrimSpace(cfg.Project.Name) == "" {
		r.Errors = append(r.Errors, "project.name is empty")
	}
	for _, t := range cfg.Terminals {
		if err := TerminalEntry(t, projectRoot); err != nil {
			r.Errors = append(r.Errors, fmt.Sprintf("terminal '%s': %v", t.Name, err))
		}
	}
	for _, a := range cfg.Applications {
		if err := AppEntry(a); err != nil {
			r.Errors = append(r.Errors, fmt.Sprintf("application '%s': %v", a.Name, err))
		}
		if !adapters.AppTargetResolvable(a.Path) {
			r.Warnings = append(r.Warnings, fmt.Sprintf(
				"application '%s': path does not exist and is not on PATH: %s", a.Name, a.Path))
		}
		for _, arg := range a.Args {
			for _, token := range adapters.UnknownPlaceholders(arg) {
				r.Warnings = append(r.Warnings, fmt.Sprintf(
					"application '%s': unknown placeholder '{%s}'", a.Name, token))
			}
		}
	}
	return r
}
