package ui

import (
	"fmt"
	"io"
	"os"
	"regexp"
	"strings"

	"golang.org/x/term"
)

// ColorChoice is the --color setting.
type ColorChoice int

const (
	ColorAuto ColorChoice = iota
	ColorAlways
	ColorNever
)

var choice = ColorAuto

// SetColorChoice applies the --color flag. An explicit always/never beats the
// NO_COLOR and CLICOLOR_FORCE environment variables.
func SetColorChoice(c ColorChoice) { choice = c }

// ParseColorChoice parses a --color value.
func ParseColorChoice(s string) (ColorChoice, error) {
	switch s {
	case "auto":
		return ColorAuto, nil
	case "always":
		return ColorAlways, nil
	case "never":
		return ColorNever, nil
	}
	return ColorAuto, fmt.Errorf("invalid value %q for --color (possible values: auto, always, never)", s)
}

func colorEnabled(f *os.File) bool {
	switch choice {
	case ColorAlways:
		return true
	case ColorNever:
		return false
	}
	if os.Getenv("NO_COLOR") != "" {
		return false
	}
	if v := os.Getenv("CLICOLOR_FORCE"); v != "" && v != "0" {
		return true
	}
	if os.Getenv("CLICOLOR") == "0" {
		return false
	}
	if os.Getenv("TERM") == "dumb" {
		return false
	}
	return term.IsTerminal(int(f.Fd()))
}

var ansi = regexp.MustCompile("\x1b\\[[0-9;]*[A-Za-z]")

// Strip removes ANSI escape sequences from s.
func Strip(s string) string { return ansi.ReplaceAllString(s, "") }

type stream struct{ f *os.File }

func (s stream) Write(p []byte) (int, error) {
	if colorEnabled(s.f) {
		return s.f.Write(p)
	}
	stripped := Strip(string(p))
	if _, err := io.WriteString(s.f, stripped); err != nil {
		return 0, err
	}
	return len(p), nil
}

// Stdout and Stderr strip styles when color is disabled for that stream.
var (
	Stdout io.Writer = stream{os.Stdout}
	Stderr io.Writer = stream{os.Stderr}
)

// Println prints to Stdout.
func Println(a ...any) { fmt.Fprintln(Stdout, a...) }

// Print prints to Stdout without a newline.
func Print(a ...any) { fmt.Fprint(Stdout, a...) }

// Eprintln prints to Stderr.
func Eprintln(a ...any) { fmt.Fprintln(Stderr, a...) }

// Eprint prints to Stderr without a newline.
func Eprint(a ...any) { fmt.Fprint(Stderr, a...) }

// Lines splits s into lines without a trailing empty element.
func Lines(s string) []string {
	s = strings.ReplaceAll(s, "\r\n", "\n")
	s = strings.TrimSuffix(s, "\n")
	if s == "" {
		return nil
	}
	return strings.Split(s, "\n")
}
