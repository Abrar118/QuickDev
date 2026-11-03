package doctor

import (
	"strings"
	"testing"

	"github.com/Abrar118/QuickDev/internal/config"
	"github.com/Abrar118/QuickDev/internal/ui"
)

func status(name string, healthy bool) config.ProjectStatus {
	return config.ProjectStatus{Name: name, Path: "/p/" + name, PathExists: true, ConfigExists: healthy}
}

func TestDoctorReports(t *testing.T) {
	healthy := Report{GlobalConfigOK: true, FzfAvailable: true, Projects: []config.ProjectStatus{status("a", true)}}
	if healthy.HasErrors() || strings.Contains(ui.Strip(healthy.Render()), "✗") {
		t.Fatal("healthy report flagged")
	}
	sick := Report{GlobalConfigOK: true, FzfAvailable: true, Projects: []config.ProjectStatus{status("b", false)}}
	if !sick.HasErrors() || !strings.Contains(ui.Strip(sick.Render()), ".quickdev.toml missing") {
		t.Fatal("unhealthy project not flagged")
	}
	noFzf := Report{GlobalConfigOK: true}
	if noFzf.HasErrors() || !strings.Contains(ui.Strip(noFzf.Render()), "⚠ fzf not found") {
		t.Fatal("missing fzf must be a warning only")
	}
	if !(Report{}).HasErrors() {
		t.Fatal("bad global config must be an error")
	}
	js := sick.JSON()
	if !strings.Contains(js, `"ok": false`) || !strings.Contains(js, `"healthy": false`) {
		t.Fatal(js)
	}
}
