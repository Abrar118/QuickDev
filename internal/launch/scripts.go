package launch

import (
	"errors"
	"fmt"
	"strings"
	"unicode"
)

// EscapeAppleScriptString escapes s for use inside an AppleScript "string".
func EscapeAppleScriptString(s string) string {
	return strings.ReplaceAll(strings.ReplaceAll(s, `\`, `\\`), `"`, `\"`)
}

// EscapePowerShellSingleQuotes escapes s for use inside a PowerShell 'string'.
func EscapePowerShellSingleQuotes(s string) string { return strings.ReplaceAll(s, "'", "''") }

func shellSingleQuoteEscape(s string) string { return strings.ReplaceAll(s, "'", `'\''`) }

// ResolvedTerminal is a terminal ready for an AppleScript builder.
type ResolvedTerminal struct {
	Cwd     string
	Command string // "" for none
}

// BuildGhosttyScript opens one Ghostty window with a tab per terminal.
func BuildGhosttyScript(terminals []ResolvedTerminal) (string, error) {
	if len(terminals) == 0 {
		return "", errors.New("no terminals to launch")
	}
	var b strings.Builder
	b.WriteString("tell application \"Ghostty\"\n")
	for i, t := range terminals {
		name := fmt.Sprintf("c%d", i)
		fmt.Fprintf(&b, "  set %s to {%s}\n", name, surfaceConfiguration(t))
		if i == 0 {
			fmt.Fprintf(&b, "  set w to new window with configuration %s\n", name)
		} else {
			fmt.Fprintf(&b, "  new tab in w with configuration %s\n", name)
		}
	}
	b.WriteString("end tell")
	return b.String(), nil
}

func surfaceConfiguration(t ResolvedTerminal) string {
	fields := []string{fmt.Sprintf(`initial working directory:"%s"`, EscapeAppleScriptString(t.Cwd))}
	if t.Command != "" {
		// Typed into the interactive shell (with a carriage return) rather than
		// Ghostty's `command` field, which runs a one-shot program and exits.
		fields = append(fields, fmt.Sprintf(`initial input:"%s" & return`, EscapeAppleScriptString(t.Command)))
	}
	return strings.Join(fields, ", ")
}

// TabbingPreference is macOS's AppleWindowTabbingMode.
type TabbingPreference int

const (
	TabbingUnknown TabbingPreference = iota
	TabbingAlways
	TabbingFullscreen
	TabbingManual
)

// ParseTabbingPreference parses `defaults read … AppleWindowTabbingMode`.
func ParseTabbingPreference(output string) TabbingPreference {
	switch strings.TrimSpace(output) {
	case "always":
		return TabbingAlways
	case "fullscreen":
		return TabbingFullscreen
	case "manual":
		return TabbingManual
	}
	return TabbingUnknown
}

// BuildTerminalCommand is the shell line Terminal.app runs for a terminal.
func BuildTerminalCommand(cwd, command string) string {
	cd := "cd '" + shellSingleQuoteEscape(cwd) + "'"
	if command != "" {
		return cd + " && " + command
	}
	return cd
}

// BuildAutoTabScript relies on AppleWindowTabbingMode=always to group windows.
func BuildAutoTabScript(terminals []ResolvedTerminal) string {
	var b strings.Builder
	b.WriteString("tell application \"Terminal\"\n  activate\n")
	for _, t := range terminals {
		fmt.Fprintf(&b, "  do script \"%s\"\n", EscapeAppleScriptString(BuildTerminalCommand(t.Cwd, t.Command)))
	}
	b.WriteString("end tell")
	return b.String()
}

// BuildWindowScript opens one Terminal.app window.
func BuildWindowScript(t ResolvedTerminal) string {
	return fmt.Sprintf("tell application \"Terminal\"\n  activate\n  do script \"%s\"\nend tell",
		EscapeAppleScriptString(BuildTerminalCommand(t.Cwd, t.Command)))
}

// BuildSystemEventsTabScript opens tabs by sending ⌘T through System Events.
func BuildSystemEventsTabScript(terminals []ResolvedTerminal) string {
	var b strings.Builder
	b.WriteString("tell application \"Terminal\"\n  activate\n")
	for i, t := range terminals {
		command := EscapeAppleScriptString(BuildTerminalCommand(t.Cwd, t.Command))
		if i == 0 {
			fmt.Fprintf(&b, "  do script \"%s\"\n", command)
			continue
		}
		b.WriteString("  tell application \"System Events\" to tell process \"Terminal\" to keystroke \"t\" using command down\n")
		b.WriteString("  delay 0.3\n")
		fmt.Fprintf(&b, "  do script \"%s\" in selected tab of front window\n", command)
	}
	b.WriteString("end tell")
	return b.String()
}

// Tab is one terminal in a grouped launch.
type Tab struct {
	Title, Cwd, Command string
}

// SessionTab is a tab's entry in a session file: its title and wrapper path.
type SessionTab struct {
	Title, WrapperPath string
}

// BuildWrapperScript is a per-tab wrapper: cd, run the optional command, then
// exec a login shell so the tab stays open after the command exits.
func BuildWrapperScript(cwd, command string) string {
	var b strings.Builder
	b.WriteString("#!/bin/sh\n")
	fmt.Fprintf(&b, "cd '%s' || exit 1\n", shellSingleQuoteEscape(cwd))
	if command != "" {
		b.WriteString(command)
		b.WriteString("\n")
	}
	b.WriteString("exec \"${SHELL:-/bin/sh}\" -l\n")
	return b.String()
}

// EscapeGKeyFileValue escapes a single-line GKeyFile string value.
func EscapeGKeyFileValue(value string) string {
	return strings.ReplaceAll(strings.ReplaceAll(value, `\`, `\\`), "\n", `\n`)
}

// BuildLoadConfig is a gnome-terminal --load-config keyfile: one window
// grouping all tabs. Wrapper paths must not contain single quotes; QuickDev
// always generates them under its own temp directory.
func BuildLoadConfig(tabs []SessionTab) string {
	ids := make([]string, len(tabs))
	for i := range tabs {
		ids[i] = fmt.Sprintf("Terminal%d", i)
	}
	var b strings.Builder
	b.WriteString("[GNOME Terminal Configuration]\nVersion=1\nCompatVersion=1\nWindows=Window0;\n\n")
	b.WriteString("[Window0]\n")
	fmt.Fprintf(&b, "Terminals=%s;\n", strings.Join(ids, ";"))
	b.WriteString("ActiveTerminal=Terminal0\n\n")
	for i, t := range tabs {
		fmt.Fprintf(&b, "[Terminal%d]\n", i)
		fmt.Fprintf(&b, "Title=%s\n", EscapeGKeyFileValue(t.Title))
		// Single-quoted so a temp dir with spaces survives g_shell_parse_argv.
		fmt.Fprintf(&b, "Command=/bin/sh '%s'\n\n", t.WrapperPath)
	}
	return b.String()
}

// EscapeSessionValue escapes a value for single quotes on a kitty session line.
func EscapeSessionValue(value string) string { return shellSingleQuoteEscape(value) }

// SanitizeTitle drops control characters: kitty reads its session file line by
// line, so a newline in a title could inject extra directives.
func SanitizeTitle(title string) string {
	return strings.Map(func(r rune) rune {
		if unicode.IsControl(r) {
			return -1
		}
		return r
	}, title)
}

// BuildKittySession is a kitty --session file: one OS window, one tab per
// entry. The first tab is implicit (kitty opens one for the first launch), so
// only later tabs get a `new_tab` line.
func BuildKittySession(tabs []SessionTab) string {
	var b strings.Builder
	for i, t := range tabs {
		title := SanitizeTitle(t.Title)
		if i == 0 {
			fmt.Fprintf(&b, "launch --title '%s' /bin/sh '%s'\n", EscapeSessionValue(title), t.WrapperPath)
		} else {
			fmt.Fprintf(&b, "new_tab %s\n", title)
			fmt.Fprintf(&b, "launch /bin/sh '%s'\n", t.WrapperPath)
		}
	}
	return b.String()
}
