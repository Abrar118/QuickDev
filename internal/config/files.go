package config

import (
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"runtime"
	"sync"
)

// atomicWrite replaces path's contents atomically.
//
// Truncating and writing in place means a crash, a full disk, or a permission
// failure partway through leaves a half-written config that no longer parses.
// Writing a sibling temp file and renaming it over the destination means
// readers only ever see the old file or the complete new one.
func atomicWrite(path, content string) error {
	dir := filepath.Dir(path)
	// Same directory as the destination: rename is only atomic within a
	// filesystem, and the temp dir may be on a different one.
	file, err := os.CreateTemp(dir, ".quickdev-write-")
	if err != nil {
		return fmt.Errorf("failed to stage config write: %w", err)
	}
	tmp := file.Name()
	committed := false
	defer func() {
		if !committed {
			file.Close()
			os.Remove(tmp)
		}
	}()
	if _, err := file.WriteString(content); err != nil {
		return fmt.Errorf("failed to write config: %w", err)
	}
	// Carry over the destination's permissions. The staged file is created
	// 0600, so replacing without this would silently re-chmod a config the user
	// may have deliberately opened up. A brand-new config keeps 0600.
	if runtime.GOOS != "windows" {
		if info, err := os.Stat(path); err == nil {
			_ = file.Chmod(info.Mode().Perm())
		}
	}
	if err := file.Sync(); err != nil {
		return fmt.Errorf("failed to flush config: %w", err)
	}
	if err := file.Close(); err != nil {
		return fmt.Errorf("failed to write config: %w", err)
	}
	if err := os.Rename(tmp, path); err != nil {
		return fmt.Errorf("failed to replace %s: %w", path, err)
	}
	committed = true
	return nil
}

// readOptional returns path's contents, or nil when it does not exist.
func readOptional(path string) (*string, error) {
	data, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		return nil, nil
	}
	if err != nil {
		return nil, fmt.Errorf("failed to read %s: %w", path, err)
	}
	s := string(data)
	return &s, nil
}

// seen records what this process last read from each config file (nil meaning
// "absent").
//
// Every mutating command is read–modify–write. Nothing else coordinates two
// concurrent invocations, so without a check the second writer would silently
// discard the first's change. Remembering what we read lets the write refuse
// when the file no longer matches.
var (
	seenMu sync.Mutex
	seen   = map[string]*string{}
)

func remember(path string, content *string) {
	seenMu.Lock()
	defer seenMu.Unlock()
	if content == nil {
		seen[path] = nil
		return
	}
	c := *content
	seen[path] = &c
}

// ensureUnchanged refuses to write when the file is not what this process last
// read: the in-memory model is stale (someone else changed the file while a
// picker was open, say).
func ensureUnchanged(path string, current *string) error {
	seenMu.Lock()
	defer seenMu.Unlock()
	recorded, ok := seen[path]
	if !ok {
		// Never read in this process: writing is only safe if we are creating
		// the file. Otherwise we would clobber contents we never saw.
		if current == nil {
			return nil
		}
		return fmt.Errorf("refusing to overwrite %s without reading it first", path)
	}
	if (recorded == nil) == (current == nil) && (recorded == nil || *recorded == *current) {
		return nil
	}
	return fmt.Errorf("%s changed on disk since it was read (another quickdev command may have run at the same time); re-run this command", path)
}

// LockPathFor is the advisory lock file guarding a config's write section.
//
// A sibling file rather than the config itself: atomicWrite replaces the config
// by rename, so a lock on the config's inode would no longer refer to the file
// other processes open. The lock file is created once and left in place.
func LockPathFor(path string) string { return path + ".lock" }

// withConfigLock runs writeSection holding an exclusive lock on path's lock
// file. Scoped tight (re-read, compare, replace) so one invocation never blocks
// on another's open picker, only on another's brief save. The kernel releases
// the lock if the holder dies, so this cannot hang on a stale lock.
func withConfigLock[T any](path string, writeSection func() (T, error)) (T, error) {
	var zero T
	lockPath := LockPathFor(path)
	f, err := os.OpenFile(lockPath, os.O_CREATE|os.O_WRONLY, 0o600)
	if err != nil {
		return zero, fmt.Errorf("failed to open %s: %w", lockPath, err)
	}
	defer f.Close()
	if err := lockFile(f); err != nil {
		return zero, fmt.Errorf("failed to lock %s: %w", lockPath, err)
	}
	defer unlockFile(f)
	return writeSection()
}

// RemoveConfigWith removes configPath only if commit succeeds: either both
// happen, or neither does.
//
// It runs under the same lock saves take, so a concurrent save cannot recreate
// the file mid-delete. The config is moved to a uniquely named staging file
// first (unique, so an earlier failed cleanup's recovery copy is never
// clobbered). If commit fails the config is moved back, and if that also fails
// the error names the staging file so it can be recovered by hand.
func RemoveConfigWith[T any](configPath string, commit func() (T, error)) (T, error) {
	return withConfigLock(configPath, func() (T, error) {
		var zero T
		staging, err := os.CreateTemp(filepath.Dir(configPath), ".quickdev-deregister-")
		if err != nil {
			return zero, fmt.Errorf("failed to stage %s: %w", configPath, err)
		}
		staged := staging.Name()
		// Closed first so Windows can rename over it.
		staging.Close()

		if err := os.Rename(configPath, staged); err != nil {
			os.Remove(staged)
			return zero, fmt.Errorf("failed to move %s aside: %w", configPath, err)
		}

		value, err := commit()
		if err != nil {
			if restoreErr := os.Rename(staged, configPath); restoreErr != nil {
				return zero, fmt.Errorf("%w\nadditionally, %s could not be put back (%v) — recover it from %s",
					err, configPath, restoreErr, staged)
			}
			return zero, err
		}

		if err := os.Remove(staged); err != nil {
			return zero, fmt.Errorf("removed %s from the index, but its staged copy at %s could not be deleted: %w",
				configPath, staged, err)
		}
		// Deliberately not remember(configPath, nil): a save still holding the
		// pre-delete config must fail its change check, not recreate the file.
		return value, nil
	})
}
