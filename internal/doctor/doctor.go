// Package doctor renders `quickdev doctor` reports.
package doctor

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/ui"
)

// Report is a snapshot of global environment health.
type Report struct {
	GlobalConfigOK bool
	FzfAvailable   bool
	Projects       []config.ProjectStatus
}

// HasErrors is true for a bad global config or any unhealthy project. A
// missing fzf is a warning, not an error.
func (r Report) HasErrors() bool {
	if !r.GlobalConfigOK {
		return true
	}
	for _, p := range r.Projects {
		if !p.IsHealthy() {
			return true
		}
	}
	return false
}

// JSON is the machine-readable report for `doctor --json`.
func (r Report) JSON() string {
	return fmt.Sprintf("{\n  \"ok\": %t,\n  \"global_config_ok\": %t,\n  \"fzf_available\": %t,\n  \"projects\": %s\n}",
		!r.HasErrors(), r.GlobalConfigOK, r.FzfAvailable, config.ProjectsJSON(r.Projects))
}

// Render is the human-readable report.
func (r Report) Render() string {
	var b strings.Builder
	fmt.Fprintf(&b, "%s\n\n", ui.Paint(ui.Bold, "QuickDev doctor"))
	if r.GlobalConfigOK {
		fmt.Fprintf(&b, "  %s\n", ui.OK("global config OK"))
	} else {
		fmt.Fprintf(&b, "  %s\n", ui.Fail("global config missing or unparseable"))
	}
	if r.FzfAvailable {
		fmt.Fprintf(&b, "  %s\n", ui.OK("fzf available"))
	} else {
		fmt.Fprintf(&b, "  %s\n", ui.Warn("fzf not found "+ui.Paint(ui.Dim, "(interactive selection disabled)")))
	}
	if len(r.Projects) == 0 {
		fmt.Fprintf(&b, "  %s\n", ui.Paint(ui.Dim, "· no projects registered"))
		return b.String()
	}
	fmt.Fprintf(&b, "\n%s\n\n", ui.Heading("Projects", strconv.Itoa(len(r.Projects))))
	width := 0
	for _, p := range r.Projects {
		width = max(width, ui.Width(p.Name))
	}
	for _, p := range r.Projects {
		path := ui.Paint(ui.Dim, ui.Tilde(p.Path))
		var row string
		if issue := p.Issue(); issue == "" {
			row = ui.OK(ui.Pad(ui.Bold, p.Name, width) + "  " + path)
		} else {
			row = ui.Fail(ui.Pad(ui.Bold, p.Name, width) + "  " + path + "  " + ui.Paint(ui.Red, issue))
		}
		fmt.Fprintf(&b, "  %s\n", row)
	}
	return b.String()
}
