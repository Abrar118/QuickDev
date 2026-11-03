package validate

import (
	"strings"
	"testing"

	"github.com/Abrar118/QuickDev/internal/models"
)

const root = "/tmp/project"

func cfg(name string, terminals []models.TerminalEntry, apps []models.AppEntry) *models.ProjectConfig {
	return &models.ProjectConfig{Project: models.ProjectEntry{Name: name}, Terminals: terminals, Applications: apps}
}

func contains(items []string, sub string) bool {
	for _, i := range items {
		if strings.Contains(i, sub) {
			return true
		}
	}
	return false
}

func TestCleanConfigIsOK(t *testing.T) {
	r := ProjectConfig(cfg("p", []models.TerminalEntry{{Name: "api", Path: "."}},
		[]models.AppEntry{{Name: "sh", Path: "/bin/sh", Args: []string{"{root}"}}}), root)
	if !r.OK() || len(r.Warnings) != 0 {
		t.Fatalf("%+v", r)
	}
}

func TestProjectConfigErrorsAndWarnings(t *testing.T) {
	r := ProjectConfig(cfg(" ",
		[]models.TerminalEntry{
			{Name: "esc", Path: "../x"},
			{Name: "emu", Path: ".", Emulator: models.Str("nonexistent-terminal")},
			{Name: "  ", Path: "."},
			{Name: "api\nweb", Path: "."},
		},
		[]models.AppEntry{
			{Name: "gone", Path: "/definitely/not/installed.app"},
			{Name: "ph", Path: "/bin/sh", Args: []string{"{bogus}"}},
			{Name: "", Path: "/bin/sh"},
		}), root)
	for _, want := range []string{"project.name is empty", "must stay inside the project root", "unsupported emulator", "cannot be empty", "control characters"} {
		if !contains(r.Errors, want) {
			t.Errorf("missing error %q in %v", want, r.Errors)
		}
	}
	for _, want := range []string{"path does not exist and is not on PATH", "unknown placeholder '{bogus}'"} {
		if !contains(r.Warnings, want) {
			t.Errorf("missing warning %q in %v", want, r.Warnings)
		}
	}
}

func TestAppEntry(t *testing.T) {
	if AppEntry(models.AppEntry{Name: "Cursor", Path: "/Applications/Cursor.app"}) != nil {
		t.Fatal("valid app rejected")
	}
	if AppEntry(models.AppEntry{Name: "", Path: "/x"}) == nil || AppEntry(models.AppEntry{Name: "x", Path: "  "}) == nil {
		t.Fatal("invalid app accepted")
	}
}

func TestReportJSON(t *testing.T) {
	path := "/p/.quickdev.toml"
	out := Report{Errors: []string{`bad "path"`}}.JSON(&path)
	for _, want := range []string{`"path": "/p/.quickdev.toml"`, `"valid": false`, `"errors": ["bad \"path\""]`, `"warnings": []`} {
		if !strings.Contains(out, want) {
			t.Errorf("missing %s in %s", want, out)
		}
	}
	if !strings.Contains(Report{}.JSON(nil), `"path": null`) {
		t.Error("nil path must be null")
	}
}
