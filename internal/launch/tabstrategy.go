package launch

import (
	"strconv"
	"strings"
)

// TabStrategy is how a group of terminals gets opened as tabs.
type TabStrategy int

const (
	CliTab TabStrategy = iota
	AppleScriptTab
	TerminalAppTab
	GnomeTerminalLoadConfig
	KittySession
	WindowOnly
)

// TabCapabilities are the probed facts the strategy depends on.
type TabCapabilities struct {
	// GhosttyAvailable mirrors the PATH check the per-terminal fallback uses,
	// distinct from whether Ghostty's AppleScript API is usable.
	GhosttyAvailable       bool
	GhosttyVersion         string
	GhosttyAppleScript     bool
	PtyxisAvailable        bool
	GnomeTerminalAvailable bool
	KittyAvailable         bool
	WtAvailable            bool
}

// SelectTabStrategy picks how to group terminals for goos ("darwin", "linux",
// "windows") and the configured emulator ("" for auto-detect).
func SelectTabStrategy(goos, emulator string, caps TabCapabilities) TabStrategy {
	switch goos {
	case "darwin":
		switch {
		// kitty groups tabs through a --session file exactly as on Linux, and
		// auto-detect prefers it, so an explicit pin and auto agree.
		case emulator == "kitty" && caps.KittyAvailable:
			return KittySession
		case emulator == "kitty":
			return WindowOnly
		case emulator == "" && caps.KittyAvailable:
			return KittySession
		case (emulator == "ghostty" || emulator == "") && MacOSGhosttyAppleScriptSupported(caps):
			return AppleScriptTab
		case emulator == "ghostty":
			return WindowOnly
		// Auto with Ghostty installed but AppleScript unsupported: defer to the
		// per-terminal fallback, which opens Ghostty windows. Choosing
		// Terminal.app here would silently switch the user off Ghostty.
		case emulator == "" && caps.GhosttyAvailable:
			return WindowOnly
		case emulator == "terminal" || emulator == "":
			return TerminalAppTab
		}
		return WindowOnly
	case "linux":
		auto := emulator == "terminal" || emulator == ""
		switch {
		case emulator == "kitty" && caps.KittyAvailable:
			return KittySession
		case emulator == "kitty":
			return WindowOnly
		case emulator == "gnome-terminal" && caps.GnomeTerminalAvailable:
			return GnomeTerminalLoadConfig
		case emulator == "gnome-terminal":
			return WindowOnly
		// Explicit Ptyxis/Ghostty open windows (no single-window CLI tabs).
		case emulator == "ptyxis" || emulator == "ghostty":
			return WindowOnly
		// Auto: kitty first, and never silently prefer gnome-terminal over
		// Ptyxis. Only tab when gnome-terminal is the terminal actually used.
		case auto && caps.KittyAvailable:
			return KittySession
		case auto && caps.PtyxisAvailable:
			return WindowOnly
		case auto && caps.GnomeTerminalAvailable:
			return GnomeTerminalLoadConfig
		}
		return WindowOnly
	case "windows":
		if caps.WtAvailable || emulator == "terminal" || emulator == "" {
			return CliTab
		}
		return WindowOnly
	}
	return WindowOnly
}

// MacOSGhosttyAppleScriptSupported requires Ghostty 1.3+ with AppleScript on.
func MacOSGhosttyAppleScriptSupported(caps TabCapabilities) bool {
	if !caps.GhosttyAppleScript {
		return false
	}
	major, minor, ok := parseVersionMajorMinor(caps.GhosttyVersion)
	return ok && (major > 1 || major == 1 && minor >= 3)
}

func parseVersionMajorMinor(version string) (int, int, bool) {
	part := ""
	for _, field := range strings.FieldsFunc(version, func(r rune) bool {
		return !(r >= '0' && r <= '9' || r == '.')
	}) {
		if field != "" {
			part = field
			break
		}
	}
	if part == "" {
		return 0, 0, false
	}
	pieces := strings.Split(part, ".")
	major, err := strconv.Atoi(pieces[0])
	if err != nil {
		return 0, 0, false
	}
	minor := 0
	if len(pieces) > 1 {
		if minor, err = strconv.Atoi(pieces[1]); err != nil {
			return 0, 0, false
		}
	}
	return major, minor, true
}
