package launch

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Session directories hold the wrapper scripts and session files for kitty and
// gnome-terminal tab launches. They must outlive this process (both terminals
// read the files after being spawned), so they are randomly named, mode 0700,
// filled with O_EXCL creates, and reaped on a later launch once stale.

// staleAfter: a wrapper is read once at tab startup, so an hour is far beyond
// the window in which the terminal still needs the files.
const staleAfter = time.Hour

// sessionPrefix is narrower than "quickdev-" so the reaper never touches the
// npm installer's staging directories.
const sessionPrefix = "quickdev-tabs-"

// sessionMarker is written in every session directory; the reaper only removes
// directories carrying it.
const sessionMarker = ".quickdev-session"

// CreateSessionDir makes a fresh owner-only session directory after reaping
// stale ones. It is intentionally not removed when this process exits.
func CreateSessionDir() (string, error) {
	reapStaleSessionDirs()
	dir, err := os.MkdirTemp("", sessionPrefix)
	if err != nil {
		return "", fmt.Errorf("failed to create temp dir: %w", err)
	}
	if err := os.Chmod(dir, 0o700); err != nil {
		os.RemoveAll(dir)
		return "", fmt.Errorf("failed to restrict temp dir permissions: %w", err)
	}
	if err := WriteNew(filepath.Join(dir, sessionMarker), "", 0o600); err != nil {
		os.RemoveAll(dir)
		return "", err
	}
	return dir, nil
}

// WriteNew writes body to a new file at path with mode. It fails if path
// already exists (including as a symlink), rather than following it.
func WriteNew(path, body string, mode os.FileMode) error {
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, mode)
	if err != nil {
		return fmt.Errorf("failed to create %s: %w", path, err)
	}
	defer f.Close()
	// The umask may have narrowed mode; set it exactly.
	_ = f.Chmod(mode)
	if _, err := f.WriteString(body); err != nil {
		return fmt.Errorf("failed to write %s: %w", path, err)
	}
	return nil
}

// reapStaleSessionDirs removes old session directories, best effort. Another
// user's directory on a shared temp filesystem is not ours to delete, and a
// failed cleanup must never block a launch.
func reapStaleSessionDirs() {
	tmp := os.TempDir()
	entries, err := os.ReadDir(tmp)
	if err != nil {
		return
	}
	now := time.Now()
	for _, e := range entries {
		path := filepath.Join(tmp, e.Name())
		if IsReapable(path, now) {
			os.RemoveAll(path)
		}
	}
}

// IsReapable reports whether path is one of our session directories (prefix,
// a real directory, our marker) and older than staleAfter.
func IsReapable(path string, now time.Time) bool {
	if !strings.HasPrefix(filepath.Base(path), sessionPrefix) {
		return false
	}
	info, err := os.Lstat(path)
	if err != nil || !info.IsDir() {
		return false
	}
	marker, err := os.Lstat(filepath.Join(path, sessionMarker))
	if err != nil || !marker.Mode().IsRegular() {
		return false
	}
	return now.Sub(info.ModTime()) > staleAfter
}

func writeWrappers(dir string, tabs []Tab) ([]SessionTab, error) {
	out := make([]SessionTab, len(tabs))
	for i, t := range tabs {
		wrapper := filepath.Join(dir, fmt.Sprintf("tab%d.sh", i))
		if err := WriteNew(wrapper, BuildWrapperScript(t.Cwd, t.Command), 0o700); err != nil {
			return nil, err
		}
		out[i] = SessionTab{Title: t.Title, WrapperPath: wrapper}
	}
	return out, nil
}

// WriteKittySession writes wrappers and a kitty session file into dir and
// returns the session file's path.
func WriteKittySession(dir string, tabs []Tab) (string, error) {
	sessionTabs, err := writeWrappers(dir, tabs)
	if err != nil {
		return "", err
	}
	path := filepath.Join(dir, "session.kitty")
	return path, WriteNew(path, BuildKittySession(sessionTabs), 0o600)
}

// WriteGnomeSession writes wrappers and a --load-config file into dir and
// returns the config file's path.
func WriteGnomeSession(dir string, tabs []Tab) (string, error) {
	sessionTabs, err := writeWrappers(dir, tabs)
	if err != nil {
		return "", err
	}
	path := filepath.Join(dir, "tabs.conf")
	return path, WriteNew(path, BuildLoadConfig(sessionTabs), 0o600)
}
