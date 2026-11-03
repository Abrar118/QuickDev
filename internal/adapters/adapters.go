// Package adapters knows how to find and identify external tools: editors,
// terminal emulators, and arbitrary commands on PATH.
package adapters

import (
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

var tools = []struct{ toolID, launchCommand string }{
	{"vscode", "code"},
	{"cursor", "cursor"},
	{"zed", "zed"},
	{"ghostty", "ghostty"},
}

// LaunchCommandForTool is the CLI that launches toolID, if known.
func LaunchCommandForTool(toolID string) (string, bool) {
	for _, t := range tools {
		if t.toolID == toolID {
			return t.launchCommand, true
		}
	}
	return "", false
}

// InferToolIDFromPath identifies a known tool from the launch path alone
// (never the display name).
//
// Used to decide whether to substitute the tool's own CLI at launch time. A
// wrapper path like `flatpak` or a Squirrel `Update.exe` must not be treated as
// the tool just because the display name matches, otherwise the CLI would be
// invoked with the wrapper's arguments.
func InferToolIDFromPath(path string) string {
	p := strings.ToLower(path)
	switch {
	case strings.Contains(p, "cursor"):
		return "cursor"
	case strings.Contains(p, "visual studio code"),
		strings.HasSuffix(p, "/code"),
		strings.HasSuffix(p, "code.app"),
		strings.HasSuffix(p, "code.exe"):
		return "vscode"
	case strings.Contains(p, "zed"):
		return "zed"
	case strings.Contains(p, "ghostty"):
		return "ghostty"
	}
	return ""
}

// InferToolID identifies a known tool from its path, falling back to its
// display name. Returns "" when unknown.
func InferToolID(name, path string) string {
	if id := InferToolIDFromPath(path); id != "" {
		return id
	}
	n := strings.ToLower(name)
	switch {
	case strings.Contains(n, "cursor"):
		return "cursor"
	case strings.Contains(n, "vscode"), n == "code", n == "visual studio code":
		return "vscode"
	case strings.Contains(n, "zed"):
		return "zed"
	case strings.Contains(n, "ghostty"):
		return "ghostty"
	}
	return ""
}

// IsEditorTool reports whether toolID opens a project directory.
func IsEditorTool(toolID string) bool {
	return toolID == "vscode" || toolID == "cursor" || toolID == "zed"
}

// ResolveCommand finds command on PATH and returns its full path.
func ResolveCommand(command string) (string, bool) {
	if command == "" {
		return "", false
	}
	path, err := exec.LookPath(command)
	if err != nil {
		return "", false
	}
	return path, true
}

// CommandExists reports whether command is on PATH.
func CommandExists(command string) bool {
	_, ok := ResolveCommand(command)
	return ok
}

// MacOSKittyBundlePaths lists where a macOS kitty install that isn't on PATH
// lives. kitty's macOS installers drop an app bundle without putting `kitty` on
// PATH, so a PATH-only probe would miss a perfectly good installation.
func MacOSKittyBundlePaths(home string) []string {
	paths := []string{"/Applications/kitty.app/Contents/MacOS/kitty"}
	if home != "" {
		paths = append(paths,
			home+"/Applications/kitty.app/Contents/MacOS/kitty",
			home+"/.local/kitty.app/bin/kitty")
	}
	return paths
}

// ResolveKitty finds the kitty executable: PATH first, then (on macOS) the
// standard app-bundle locations. Every kitty probe and launch goes through this
// so detection and launching never disagree.
func ResolveKitty() (string, bool) {
	if path, ok := ResolveCommand("kitty"); ok {
		return path, true
	}
	if runtime.GOOS == "darwin" {
		home, _ := os.UserHomeDir()
		for _, p := range MacOSKittyBundlePaths(home) {
			if info, err := os.Stat(p); err == nil && info.Mode().IsRegular() {
				return p, true
			}
		}
	}
	return "", false
}

// EmulatorInstalled reports whether a supported emulator can launch here,
// probed the same way the launchers probe it.
func EmulatorInstalled(emulator string) bool {
	unixNotMac := runtime.GOOS != "darwin" && runtime.GOOS != "windows"
	_, kitty := ResolveKitty()
	switch emulator {
	case "kitty":
		return runtime.GOOS != "windows" && kitty
	case "ghostty":
		return CommandExists("ghostty")
	case "terminal":
		if !unixNotMac || kitty {
			return true
		}
		for _, bin := range []string{"ptyxis", "gnome-terminal", "konsole", "alacritty", "xterm"} {
			if CommandExists(bin) {
				return true
			}
		}
		return false
	case "gnome-terminal", "ptyxis":
		return unixNotMac && CommandExists(emulator)
	}
	return false
}

// NormalizePath expands a leading "~/" to the home directory.
func NormalizePath(path string) string {
	if rest, ok := strings.CutPrefix(path, "~/"); ok {
		if home, err := os.UserHomeDir(); err == nil && home != "" {
			return home + "/" + rest
		}
	}
	return path
}

// AppTargetResolvable reports whether an application target is launchable: a
// path on disk, or a bare command name found on PATH.
func AppTargetResolvable(path string) bool {
	normalized := NormalizePath(path)
	if _, err := os.Stat(normalized); err == nil {
		return true
	}
	if strings.ContainsRune(normalized, filepath.Separator) {
		return false
	}
	return CommandExists(normalized)
}
