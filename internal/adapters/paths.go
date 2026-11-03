package adapters

import (
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

// ResolveTerminalPath joins a terminal's configured path onto the project root,
// refusing anything that could leave the project (absolute paths, rooted
// paths, drive prefixes, or `..` components).
func ResolveTerminalPath(projectRoot, relativePath string) (string, error) {
	escapes := filepath.IsAbs(relativePath) ||
		strings.HasPrefix(relativePath, "/") ||
		filepath.VolumeName(relativePath) != ""
	if runtime.GOOS == "windows" && strings.HasPrefix(relativePath, `\`) {
		escapes = true
	}
	for _, part := range splitPath(relativePath) {
		if part == ".." {
			escapes = true
		}
	}
	if escapes {
		return "", fmt.Errorf("terminal path %q must stay inside the project root", relativePath)
	}
	return filepath.Join(projectRoot, relativePath), nil
}

func splitPath(p string) []string {
	return strings.FieldsFunc(p, func(r rune) bool {
		return r == '/' || (runtime.GOOS == "windows" && r == '\\')
	})
}

// ResolveExistingTerminalPath is ResolveTerminalPath plus a check that the
// result is a directory on disk.
//
// Grouped (tabbed) launches hand every directory to the emulator at once, so a
// missing directory would only make that tab's `cd` fail while QuickDev marked
// every tab successful. Catching it here keeps the reported result honest.
func ResolveExistingTerminalPath(projectRoot, relativePath string) (string, error) {
	resolved, err := ResolveTerminalPath(projectRoot, relativePath)
	if err != nil {
		return "", err
	}
	info, err := os.Stat(resolved)
	if err != nil {
		return "", fmt.Errorf("path not found: %s", resolved)
	}
	if !info.IsDir() {
		return "", fmt.Errorf("path is not a directory: %s", resolved)
	}
	return resolved, nil
}

// PlaceholderContext holds the values substituted into application args.
type PlaceholderContext struct {
	Root, Config, Name, Cwd string
}

// KnownPlaceholders are the token names recognized in application args.
var KnownPlaceholders = []string{"root", "config", "name", "cwd"}

// ResolveAppArgs substitutes {root} {config} {name} {cwd} in each arg. A whole
// arg equal to "." means {root}, for backward compatibility.
func ResolveAppArgs(args []string, ctx PlaceholderContext) []string {
	out := make([]string, 0, len(args))
	for _, arg := range args {
		if arg == "." {
			out = append(out, ctx.Root)
			continue
		}
		out = append(out, substitutePlaceholders(arg, ctx))
	}
	return out
}

// substitutePlaceholders replaces each {token} once, never re-scanning
// replacement values, so a project named "{cwd}" is not double-expanded.
// Unknown tokens and an unmatched { are left untouched.
func substitutePlaceholders(input string, ctx PlaceholderContext) string {
	var out strings.Builder
	rest := input
	for {
		start := strings.IndexByte(rest, '{')
		if start < 0 {
			break
		}
		out.WriteString(rest[:start])
		after := rest[start:]
		end := strings.IndexByte(after, '}')
		if end < 0 {
			out.WriteString(after)
			rest = ""
			break
		}
		token := after[:end+1]
		switch token {
		case "{root}":
			out.WriteString(ctx.Root)
		case "{config}":
			out.WriteString(ctx.Config)
		case "{name}":
			out.WriteString(ctx.Name)
		case "{cwd}":
			out.WriteString(ctx.Cwd)
		default:
			out.WriteString(token)
		}
		rest = after[end+1:]
	}
	out.WriteString(rest)
	return out.String()
}

// UnknownPlaceholders lists {token} names in arg that are not known
// placeholders.
func UnknownPlaceholders(arg string) []string {
	var unknown []string
	rest := arg
	for {
		open := strings.IndexByte(rest, '{')
		if open < 0 {
			break
		}
		closeRel := strings.IndexByte(rest[open:], '}')
		if closeRel < 0 {
			break
		}
		closeIdx := open + closeRel
		token := rest[open+1 : closeIdx]
		known := false
		for _, k := range KnownPlaceholders {
			if k == token {
				known = true
			}
		}
		if token != "" && !known {
			unknown = append(unknown, token)
		}
		rest = rest[closeIdx+1:]
	}
	return unknown
}

// EditorArgs are the args for an editor tool: the configured ones when
// present, otherwise the project root.
func EditorArgs(args []string, projectRoot string) []string {
	if len(args) > 0 {
		return append([]string(nil), args...)
	}
	return []string{projectRoot}
}

// EffectiveAppArgs are the args an app will actually launch with: editors
// default to opening the project root.
func EffectiveAppArgs(name, path string, args []string, projectRoot string) []string {
	if id := InferToolID(name, NormalizePath(path)); id != "" && IsEditorTool(id) {
		return EditorArgs(args, projectRoot)
	}
	return args
}
