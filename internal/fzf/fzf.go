// Package fzf drives the fzf fuzzy finder for interactive selection.
package fzf

import (
	"bytes"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"
	"unicode"

	"github.com/Abrar118/QuickDev/internal/adapters"
)

// ErrCancelled is returned when the user cancels a picker (ESC / ctrl-c or an
// empty selection). main maps it to a clean "Cancelled." and exit 0.
var ErrCancelled = errors.New("cancelled")

// IsCancellation reports whether err is a cancelled picker.
func IsCancellation(err error) bool { return errors.Is(err, ErrCancelled) }

// CheckFzf reports whether fzf is installed.
func CheckFzf() bool { return adapters.CommandExists("fzf") }

// InstallHint tells the user how to install fzf on this platform.
func InstallHint() string {
	hint := "apt install fzf"
	switch runtime.GOOS {
	case "darwin":
		hint = "brew install fzf"
	case "windows":
		hint = "choco install fzf"
	}
	return "fzf is required for interactive selection.\nInstall: " + hint
}

func run(items []string, header string, extraArgs []string) ([]string, error) {
	if !CheckFzf() {
		return nil, errors.New(InstallHint())
	}
	if len(items) == 0 {
		return nil, errors.New("no items to select from")
	}

	args := append([]string{"--header", header, "--height", "~50%", "--reverse"}, extraArgs...)
	cmd := exec.Command("fzf", args...)
	cmd.Stdin = strings.NewReader(strings.Join(items, "\n"))
	var stdout bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			return nil, ErrCancelled
		}
		return nil, fmt.Errorf("failed to start fzf: %w", err)
	}

	var selected []string
	for _, line := range strings.Split(stdout.String(), "\n") {
		if line = strings.TrimSpace(line); line != "" {
			selected = append(selected, line)
		}
	}
	if len(selected) == 0 {
		return nil, ErrCancelled
	}
	return selected, nil
}

// SelectOne returns the single line the user picked.
func SelectOne(items []string, header string) (string, error) {
	selected, err := run(items, header, nil)
	if err != nil {
		return "", err
	}
	return selected[0], nil
}

// SanitizeRow strips characters that would break the one-item-per-line picker
// protocol: a newline or tab inside user data would forge or corrupt rows.
func SanitizeRow(value string) string {
	return strings.Map(func(r rune) rune {
		if unicode.IsControl(r) {
			return -1
		}
		return r
	}, value)
}

// indexedArgs hide the leading index column from the user.
var indexedArgs = []string{"--delimiter", "\t", "--with-nth", "2.."}

func indexedRows(items []string) []string {
	rows := make([]string, len(items))
	for i, item := range items {
		rows[i] = fmt.Sprintf("%d\t%s", i, SanitizeRow(item))
	}
	return rows
}

// SelectMultiIndexed presents items and returns the positions of the rows the
// user picked. Each row is sent as `<index>\t<display>` with only the display
// visible, so the index round-trips regardless of what the display contains.
func SelectMultiIndexed(items []string, header string) ([]int, error) {
	selected, err := run(indexedRows(items), header, append([]string{"--multi"}, indexedArgs...))
	if err != nil {
		return nil, err
	}
	return ParseIndexedSelection(selected, len(items))
}

// SelectOneIndexed is the single-selection counterpart of SelectMultiIndexed.
func SelectOneIndexed(items []string, header string) (int, error) {
	selected, err := run(indexedRows(items), header, indexedArgs)
	if err != nil {
		return 0, err
	}
	positions, err := ParseIndexedSelection(selected, len(items))
	if err != nil {
		return 0, err
	}
	if len(positions) == 0 {
		return 0, ErrCancelled
	}
	return positions[0], nil
}

// ParseIndexedSelection recovers row positions from fzf's hidden index column.
// Anything that isn't an in-range index is an error rather than a silent skip:
// acting on a partial selection would remove or launch the wrong items.
func ParseIndexedSelection(selected []string, count int) ([]int, error) {
	positions := make([]int, 0, len(selected))
	for _, line := range selected {
		index, _, found := strings.Cut(line, "\t")
		n, err := strconv.Atoi(strings.TrimSpace(index))
		if !found || err != nil || n < 0 || n >= count {
			return nil, fmt.Errorf("unrecognized selection: %s", line)
		}
		positions = append(positions, n)
	}
	return positions, nil
}
