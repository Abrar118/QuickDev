package apps

import (
	"errors"
	"os/exec"
	"runtime"
	"strings"

	"github.com/Abrar118/QuickDev/internal/models"
)

// NormalizeBundlePath strips a single trailing "/" so `…/Cursor.app/` and
// `…/Cursor.app` compare equal.
func NormalizeBundlePath(path string) string { return strings.TrimSuffix(path, "/") }

// DetectedToApps maps running app bundle paths to installed bundles, using the
// installed bundle's name and path. Unmatched paths are dropped; the result is
// sorted by name and unique by normalized path.
func DetectedToApps(runningPaths []string, installed []NamePath) []models.AppEntry {
	var apps []models.AppEntry
	seen := map[string]bool{}
	for _, p := range runningPaths {
		norm := NormalizeBundlePath(p)
		if seen[norm] {
			continue
		}
		seen[norm] = true
		for _, inst := range installed {
			if NormalizeBundlePath(inst.Path) == norm {
				apps = append(apps, models.AppEntry{Name: inst.Name, Path: inst.Path})
				break
			}
		}
	}
	sortByName(apps)
	return apps
}

// MergeApps returns the captured apps not already in existing (compared by
// normalized path), preserving captured order.
func MergeApps(existing, captured []models.AppEntry) []models.AppEntry {
	var out []models.AppEntry
	for _, c := range captured {
		cn := NormalizeBundlePath(c.Path)
		present := false
		for _, e := range existing {
			if NormalizeBundlePath(e.Path) == cn {
				present = true
				break
			}
		}
		if !present {
			out = append(out, c)
		}
	}
	return out
}

// DetectRunningApps returns the bundle paths of running GUI apps (macOS only).
// An osascript failure (e.g. Automation permission denied) is an error, so it
// is not mistaken for "nothing running".
func DetectRunningApps() ([]string, error) {
	if runtime.GOOS != "darwin" {
		return nil, nil
	}
	cmd := exec.Command("osascript",
		"-e", "set text item delimiters to linefeed",
		"-e", `tell application "System Events" to get (POSIX path of (application file of every process whose background only is false)) as text`)
	var stderr strings.Builder
	cmd.Stderr = &stderr
	out, err := cmd.Output()
	if err != nil {
		var exitErr *exec.ExitError
		if !errors.As(err, &exitErr) {
			return nil, errors.New("failed to run osascript: " + err.Error())
		}
		detail := strings.TrimSpace(stderr.String())
		if detail == "" {
			detail = "osascript failed (is Automation/Accessibility permission granted?)"
		}
		return nil, errors.New("could not list running apps: " + detail)
	}
	var paths []string
	for _, line := range strings.Split(string(out), "\n") {
		if line = strings.TrimSpace(line); line != "" {
			paths = append(paths, line)
		}
	}
	return paths, nil
}
