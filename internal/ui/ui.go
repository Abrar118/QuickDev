// Package ui holds the terminal styling shared by every command's output.
//
// Renderers always emit ANSI styles; printing goes through Stdout/Stderr, which
// strip them when the stream isn't a terminal, NO_COLOR is set, or --color says
// so.
package ui

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"unicode/utf8"
)

// Style is an ANSI SGR prefix. The zero value is plain text.
type Style string

const (
	Plain  Style = ""
	Green  Style = "\x1b[32m"
	Red    Style = "\x1b[31m"
	Yellow Style = "\x1b[33m"
	Cyan   Style = "\x1b[36m"
	Bold   Style = "\x1b[1m"
	Dim    Style = "\x1b[2m"
)

const reset = "\x1b[0m"

// With combines two styles, e.g. Red.With(Bold).
func (s Style) With(other Style) Style { return other + s }

// Paint wraps text in style, resetting afterwards.
func Paint(style Style, text any) string {
	if style == Plain {
		return fmt.Sprint(text)
	}
	return string(style) + fmt.Sprint(text) + reset
}

// OK prefixes msg with a green check mark.
func OK(msg any) string { return Paint(Green, "✓") + " " + fmt.Sprint(msg) }

// Warn prefixes msg with a yellow warning sign.
func Warn(msg any) string { return Paint(Yellow, "⚠") + " " + fmt.Sprint(msg) }

// Fail prefixes msg with a red cross.
func Fail(msg any) string { return Paint(Red, "✗") + " " + fmt.Sprint(msg) }

// Heading is a bold title with an optional dimmed note after it.
func Heading(title, note string) string {
	if note == "" {
		return Paint(Bold, title)
	}
	return Paint(Bold, title) + "  " + Paint(Dim, note)
}

// Width is the number of characters in s (Unicode scalar values).
func Width(s string) int { return utf8.RuneCountInString(s) }

// Pad left-aligns text to width columns, padding after styling so ANSI codes
// don't count toward the width.
func Pad(style Style, text string, width int) string {
	fill := width - Width(text)
	if fill < 0 {
		fill = 0
	}
	return Paint(style, text) + strings.Repeat(" ", fill)
}

// Tilde is the display form of a path: the home directory abbreviated to ~.
func Tilde(path string) string {
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return path
	}
	home = filepath.Clean(home)
	clean := filepath.Clean(path)
	if clean == home {
		return "~"
	}
	prefix := home
	if !strings.HasSuffix(prefix, string(filepath.Separator)) {
		prefix += string(filepath.Separator)
	}
	if strings.HasPrefix(clean, prefix) {
		return "~" + string(filepath.Separator) + clean[len(prefix):]
	}
	return path
}

// Count is n followed by word, pluralized with a trailing "s" when n != 1.
func Count(n int, word string) string {
	if n == 1 {
		return fmt.Sprintf("%d %s", n, word)
	}
	return fmt.Sprintf("%d %ss", n, word)
}

// Closest is the candidate most similar to input, if any is close enough to be
// worth suggesting (Jaro similarity above 0.7).
func Closest(input string, candidates []string) (string, bool) {
	best, bestScore := "", 0.0
	for _, c := range candidates {
		score := Jaro(input, c)
		if score > 0.7 && score >= bestScore {
			best, bestScore = c, score
		}
	}
	return best, bestScore > 0.7
}

// WithSuggestion appends a "did you mean" tip when there is a suggestion.
func WithSuggestion(message, suggestion string, ok bool) string {
	if !ok {
		return message
	}
	return fmt.Sprintf("%s\n\n  %s did you mean '%s'?", message, Paint(Green, "tip:"), suggestion)
}

// Ago is a coarse "how long ago" for a Unix timestamp. A timestamp in the
// future (clock skew) reads as "just now".
func Ago(then, now int64) string {
	secs := now - then
	if secs < 0 {
		secs = 0
	}
	switch {
	case secs < 60:
		return "just now"
	case secs < 3600:
		return fmt.Sprintf("%dm ago", secs/60)
	case secs < 86400:
		return fmt.Sprintf("%dh ago", secs/3600)
	default:
		return fmt.Sprintf("%dd ago", secs/86400)
	}
}

// Jaro is the Jaro similarity of a and b, from 0 (no match) to 1 (equal).
func Jaro(a, b string) float64 {
	ar, br := []rune(a), []rune(b)
	if len(ar) == 0 && len(br) == 0 {
		return 1
	}
	if len(ar) == 0 || len(br) == 0 {
		return 0
	}
	window := max(len(ar), len(br))/2 - 1
	if window < 0 {
		window = 0
	}
	aMatched := make([]bool, len(ar))
	bMatched := make([]bool, len(br))
	matches := 0
	for i, ca := range ar {
		lo := max(0, i-window)
		hi := min(len(br)-1, i+window)
		for j := lo; j <= hi; j++ {
			if !bMatched[j] && br[j] == ca {
				aMatched[i], bMatched[j] = true, true
				matches++
				break
			}
		}
	}
	if matches == 0 {
		return 0
	}
	transpositions, j := 0, 0
	for i := range ar {
		if !aMatched[i] {
			continue
		}
		for !bMatched[j] {
			j++
		}
		if ar[i] != br[j] {
			transpositions++
		}
		j++
	}
	m := float64(matches)
	return (m/float64(len(ar)) + m/float64(len(br)) + (m-float64(transpositions/2))/m) / 3
}
