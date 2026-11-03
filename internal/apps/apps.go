// Package apps discovers installed applications and matches running ones.
package apps

import (
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"

	"github.com/Abrar118/QuickDev/internal/models"
	"github.com/Abrar118/QuickDev/internal/shellwords"
)

// ParseDesktopEntry parses the [Desktop Entry] group of a .desktop file.
//
// Returns false for entries the OS launcher would hide: Type != Application,
// NoDisplay=true, Hidden=true, missing Name/Exec, or a TryExec that
// tryExecResolvable rejects. Later groups ([Desktop Action …]) are ignored.
func ParseDesktopEntry(contents string, tryExecResolvable func(string) bool) (models.AppEntry, bool) {
	inGroup := false
	var typ, name, exec, tryExec *string
	noDisplay, hidden := false, false

	for _, line := range strings.Split(contents, "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "[") && strings.HasSuffix(line, "]") {
			inGroup = line == "[Desktop Entry]"
			continue
		}
		if !inGroup {
			continue
		}
		key, value, ok := strings.Cut(line, "=")
		if !ok {
			continue
		}
		key, value = strings.TrimSpace(key), strings.TrimSpace(value)
		v := value
		switch key {
		case "Type":
			typ = &v
		case "Name":
			if name == nil {
				name = &v
			}
		case "Exec":
			exec = &v
		case "TryExec":
			tryExec = &v
		case "NoDisplay":
			noDisplay = value == "true"
		case "Hidden":
			hidden = value == "true"
		}
	}

	if typ == nil || *typ != "Application" || noDisplay || hidden {
		return models.AppEntry{}, false
	}
	if tryExec != nil && !tryExecResolvable(*tryExec) {
		return models.AppEntry{}, false
	}
	if name == nil || *name == "" || exec == nil || *exec == "" {
		return models.AppEntry{}, false
	}
	path, args := ParseExec(*exec)
	if path == "" {
		return models.AppEntry{}, false
	}
	return models.AppEntry{Name: *name, Path: path, Args: args}, true
}

// CombineAppArgs merges discovered launch args with user-entered ones, keeping
// discovered args first (they are launch-critical, e.g. `flatpak run app.id`).
func CombineAppArgs(discovered, user []string) []string {
	combined := append(append([]string(nil), discovered...), user...)
	if len(combined) == 0 {
		return nil
	}
	return combined
}

var fieldCodes = map[string]bool{
	"%f": true, "%F": true, "%u": true, "%U": true, "%i": true, "%c": true, "%k": true,
	"%d": true, "%D": true, "%n": true, "%N": true, "%v": true, "%m": true,
}

// ParseExec cleans a Freedesktop Exec= value into executable and args. Field
// codes are stripped and `%%` becomes a literal `%`.
func ParseExec(exec string) (string, []string) {
	const sentinel = "\x00"
	tokens, err := shellwords.Split(strings.ReplaceAll(exec, "%%", sentinel))
	if err != nil {
		tokens = nil
	}
	var cleaned []string
	for _, token := range tokens {
		if fieldCodes[token] {
			continue
		}
		if t := strings.ReplaceAll(token, sentinel, "%"); t != "" {
			cleaned = append(cleaned, t)
		}
	}
	if len(cleaned) == 0 {
		return "", nil
	}
	if len(cleaned) == 1 {
		return cleaned[0], nil
	}
	return cleaned[0], cleaned[1:]
}

// DiscoverApps lists installed applications sorted by name and deduplicated by
// name, for the `add` picker.
func DiscoverApps() []models.AppEntry {
	var out []models.AppEntry
	for _, a := range installedApps() {
		if len(out) > 0 && strings.ToLower(out[len(out)-1].Name) == strings.ToLower(a.Name) {
			continue
		}
		out = append(out, a)
	}
	return out
}

// NamePath is an installed app's display name and path.
type NamePath struct{ Name, Path string }

// DiscoverAppsUniqueByPath lists installed applications deduplicated by path
// only. capture matches by path, so it must see every distinct path even when
// two directories hold a same-named app.
func DiscoverAppsUniqueByPath() []NamePath {
	seen := map[string]bool{}
	var out []NamePath
	for _, a := range installedApps() {
		if seen[a.Path] {
			continue
		}
		seen[a.Path] = true
		out = append(out, NamePath{a.Name, a.Path})
	}
	return out
}

func sortByName(apps []models.AppEntry) {
	sort.SliceStable(apps, func(i, j int) bool {
		return strings.ToLower(apps[i].Name) < strings.ToLower(apps[j].Name)
	})
}

func installedApps() []models.AppEntry {
	var apps []models.AppEntry
	switch runtime.GOOS {
	case "darwin":
		dirs := []string{"/Applications"}
		if home, err := os.UserHomeDir(); err == nil {
			dirs = append(dirs, filepath.Join(home, "Applications"))
		}
		for _, dir := range dirs {
			entries, _ := os.ReadDir(dir)
			for _, e := range entries {
				n := e.Name()
				if strings.HasPrefix(n, ".") || !strings.HasSuffix(n, ".app") {
					continue
				}
				apps = append(apps, models.AppEntry{
					Name: strings.TrimSuffix(n, ".app"),
					Path: filepath.Join(dir, n),
				})
			}
		}
	case "linux":
		dirs := []string{
			"/usr/share/applications",
			"/usr/local/share/applications",
			"/var/lib/flatpak/exports/share/applications",
			"/var/lib/snapd/desktop/applications",
		}
		if home, err := os.UserHomeDir(); err == nil {
			dirs = append(dirs, filepath.Join(home, ".local/share/applications"))
		}
		for _, dir := range dirs {
			entries, _ := os.ReadDir(dir)
			for _, e := range entries {
				if !strings.HasSuffix(strings.ToLower(e.Name()), ".desktop") {
					continue
				}
				data, err := os.ReadFile(filepath.Join(dir, e.Name()))
				if err != nil {
					continue
				}
				if app, ok := ParseDesktopEntry(string(data), tryExecResolvable); ok {
					apps = append(apps, app)
				}
			}
		}
	case "windows":
		apps = windowsStartMenuApps()
	}
	sortByName(apps)
	return apps
}

// tryExecResolvable reports whether a .desktop TryExec value points at an
// executable file: a path, or a bare name found on PATH.
func tryExecResolvable(cmd string) bool {
	isExec := func(p string) bool {
		info, err := os.Stat(p)
		return err == nil && info.Mode().IsRegular() && info.Mode().Perm()&0o111 != 0
	}
	if strings.Contains(cmd, "/") {
		return isExec(cmd)
	}
	for _, dir := range filepath.SplitList(os.Getenv("PATH")) {
		if isExec(filepath.Join(dir, cmd)) {
			return true
		}
	}
	return false
}
